//! A PoC offchain worker that fetches data from Stellar Horizon Servers

#![cfg_attr(not(feature = "std"), no_std)]
#![feature(result_flattening)]

extern crate alloc;

mod horizon;

use alloc::string::String;
use codec::{Decode, Encode};

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_system::offchain::{SignedPayload, SigningTypes};
use sp_core::crypto::KeyTypeId;
use sp_runtime::offchain::Duration;
use sp_runtime::traits::StaticLookup;
use sp_runtime::{MultiSignature, RuntimeDebug};
use sp_std::{prelude::*, str};

use orml_traits::{MultiCurrency, MultiReservableCurrency};

use serde::Deserialize;

use substrate_stellar_sdk as stellar;

use pallet_transaction_payment::Config as PaymentConfig;

use self::horizon::*;

pub use pallet::*;

pub use pendulum_common::currency::CurrencyId;

type BalanceOf<T> =
    <<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

type CurrencyIdOf<T> =
    <<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

// Re-export pallet items so that they can be accessed from the crate namespace.
// pub use pallet::*;

pub type Balance = u128;

pub type Signature = MultiSignature;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"abcd");

pub const FETCH_TIMEOUT_PERIOD: Duration = Duration::from_millis(3000);
pub const SUBMISSION_TIMEOUT_PERIOD: Duration = Duration::from_millis(10000);

const UNSIGNED_TXS_PRIORITY: u64 = 100;

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// them with the pallet-specific identifier.
pub mod crypto {
    use super::KEY_TYPE;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::app_crypto::{app_crypto, sr25519};
    use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

    app_crypto!(sr25519, KEY_TYPE);

    pub struct TestAuthId;
    // implemented for ocw-runtime
    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    // implemented for mock runtime in test
    impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
        for TestAuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct DepositPayload<Currency, AccountId, Public, Balance> {
    currency_id: Currency,
    amount: Balance,
    destination: AccountId,
    signed_by: Public,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Withdrawal<Balance, Currency, Public>
where
    Balance: Encode + Decode,
    Currency: Encode + Decode,
    Public: Encode + Decode,
{
    amount: Balance,
    currency: Currency,
    pendulum_address: Public,
}

impl<T: SigningTypes> SignedPayload<T>
    for DepositPayload<CurrencyIdOf<T>, T::AccountId, T::Public, BalanceOf<T>>
where
    T: pallet::Config,
{
    fn public(&self) -> T::Public {
        self.signed_by.clone()
    }
}

#[derive(Debug, Deserialize, Encode, Decode, Default)]
struct IndexingData(Vec<u8>, u64);

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::dispatch::DispatchResultWithPostInfo;
    use frame_support::error::LookupError;
    use frame_system::offchain::SendUnsignedTransaction;
    use frame_system::offchain::{AppCrypto, CreateSignedTransaction, Signer};
    use sp_runtime::offchain::http::{Request, Response};
    use sp_runtime::offchain::HttpError;
    use sp_std::str::Utf8Error;
    use stellar::network::TEST_NETWORK;
    use stellar::{SecretKey, StellarSdkError, XdrCodec};

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + CreateSignedTransaction<Call<Self>>
        + PaymentConfig
        + orml_tokens::Config
    {
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
        /// The overarching dispatch call type.
        type Call: From<Call<Self>>;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The mechanics of the ORML tokens
        type Currency: MultiReservableCurrency<Self::AccountId>;
        type AddressConversion: StaticLookup<Source = Self::AccountId, Target = stellar::PublicKey>;
        type BalanceConversion: StaticLookup<Source = BalanceOf<Self>, Target = i64>;
        type CurrencyConversion: StaticLookup<Source = CurrencyIdOf<Self>, Target = [u8; 4]>;

        type GatewayEscrowAccount: Get<&'static str>;
        type GatewayEscrowKeypair: Get<SecretKey>;
        type GatewayMockedDestination: Get<<Self as frame_system::Config>::AccountId>;
        type GatewayMockedStellarAsset: Get<stellar::Asset>;
        type GatewayMockedWithdrawalDestination: Get<&'static str>;
    }

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId", BalanceOf<T> = "Balance", CurrencyIdOf<T> = "Currency")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    // #[pallet::generate_withdrawal(pub(super) fn withdrawal_event)]
    pub enum Event<T: Config> {
        /// Event generated when a new deposit is made on Stellar Escrow Account.
        Deposit(CurrencyIdOf<T>, T::AccountId, BalanceOf<T>),

        /// Event generated when a new withdrawal has been completed on Stellar.
        Withdrawal(CurrencyIdOf<T>, T::AccountId, BalanceOf<T>),
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // Pallet implements [`Hooks`] trait to define some logic to execute in some context.
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        // `on_initialize` is executed at the beginning of the block before any extrinsic are
        // dispatched.
        //
        // This function must return the weight consumed by `on_initialize` and `on_finalize`.
        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // Anything that needs to be done at the start of the block.
            // We don't do anything here.
            0
        }

        // `on_finalize` is executed at the end of block after all extrinsic are dispatched.
        fn on_finalize(_n: T::BlockNumber) {
            // Perform necessary data/state clean up here.
        }

        // A runtime code run after every block and have access to extended set of APIs.
        //
        // For instance you can generate extrinsics for the upcoming produced block.
        fn offchain_worker(_n: T::BlockNumber) {
            debug::info!("Hello from an offchain worker 👋");

            let res = Self::fetch_latest_txs();
            let transactions = &res.unwrap()._embedded.records;

            /////////////////////////////////////////
            // Handle Stellar txs inbound to escrow

            if transactions.len() > 0 {
                Self::handle_new_transaction(&transactions[0]);
            }

            //////////////////////////////////////
            // Execute pending escrow withdrawals

            // Limitations:
            // * Only processes one withdrawal per Pendulum block
            // * Should have a mutex to prevent multiple withdrawals if withdrawal
            //   takes longer than one Pendulum block (seq. no. clashes!)

            let submission_result = (|| {
                Self::pop_queued_withdrawal()
                    .map(|maybe_withdrawal| match maybe_withdrawal {
                        Some(withdrawal) => Self::execute_withdrawal(withdrawal),
                        None => Ok(()),
                    })
                    .flatten()
            })();

            submission_result
                .map_err(|error| {
                    debug::error!(
                        "🚨 Processing outbound Stellar tx queue failed: {:?}",
                        error
                    );
                })
                .ok();
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // This is your public interface. Be extremely careful.
        #[pallet::weight(10000)]
        pub fn submit_deposit_unsigned_with_signed_payload(
            origin: OriginFor<T>,
            payload: DepositPayload<CurrencyIdOf<T>, T::AccountId, T::Public, BalanceOf<T>>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_none(origin)?;

            let DepositPayload {
                currency_id,
                amount,
                destination,
                signed_by,
            } = payload;

            debug::info!(
                "submit_deposit_unsigned_with_signed_payload: ({:?}, {:?}, {:?})",
                amount,
                destination,
                signed_by
            );

            let result = T::Currency::deposit(currency_id, &destination, amount);
            debug::info!("{:?}", result);

            Self::deposit_event(Event::Deposit(currency_id, destination, amount));
            Ok(().into())
        }

        #[pallet::weight(100000)]
        pub fn withdraw_to_stellar(
            origin: OriginFor<T>,
            currency_id: CurrencyIdOf<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let pendulum_account_id = ensure_signed(origin)?;
            let stellar_address = T::AddressConversion::lookup(pendulum_account_id.clone())?;

            debug::debug!(
                "Queue withdrawal: ({:?}, {:?}, {:?})",
                currency_id,
                amount,
                stellar_address
            );

            Self::queue_withdrawal(pendulum_account_id.clone(), currency_id, amount);

            Self::deposit_event(Event::Withdrawal(currency_id, pendulum_account_id, amount));
            Ok(().into())
        }

        #[pallet::weight(100000)]
        pub fn pendulum_withdraw(
            origin: OriginFor<T>,
            currency_id: CurrencyIdOf<T>,
            amount: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {

            let pendulum_account_id = ensure_signed(origin)?;
            T::Currency::withdraw(currency_id, &pendulum_account_id, amount)
                .map_err(|_| <Error<T>>::BalanceChangeError)?;
            //Self::deposit_event(Event::Withdrawal(currency_id, origin, amount));
            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        fn create_withdrawal_tx(
            stellar_addr: &stellar::PublicKey,
            seq_num: i64,
            asset: stellar::Asset,
            amount: BalanceOf<T>,
        ) -> Result<stellar::Transaction, Error<T>> {
            let destination_addr = stellar_addr.as_binary();
            let source_pubkey = stellar::PublicKey::from_encoding(T::GatewayEscrowAccount::get())
                .map_err(|_| <Error<T>>::StellarAddressParsingError)?;

            let mut tx =
                stellar::Transaction::new(source_pubkey, seq_num, Some(10_000), None, None)?;

            tx.append_operation(stellar::Operation::new_payment(
                stellar::MuxedAccount::KeyTypeEd25519(*destination_addr),
                asset,
                stellar::StroopAmount(
                    T::BalanceConversion::lookup(amount)
                        .map_err(|_| <Error<T>>::BalanceConversionError)?,
                ),
            )?)?;

            Ok(tx)
        }

        fn execute_withdrawal(
            withdrawal: Withdrawal<BalanceOf<T>, CurrencyIdOf<T>, T::AccountId>,
        ) -> Result<(), Error<T>> {
            let amount = withdrawal.amount;
            let currency_id = withdrawal.currency;
            let pendulum_account_id = withdrawal.pendulum_address;

            let asset_code = T::CurrencyConversion::lookup(currency_id)?;
            let mocked_stellar_issuer = "GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC";
            let asset = stellar::Asset::from_asset_code(asset_code, mocked_stellar_issuer)?;

            let escrow_address = T::GatewayEscrowAccount::get();
            // let stellar_address = T::AddressConversion::lookup(pendulum_account_id.clone())?;
            let stellar_address = stellar::PublicKey::from_encoding(T::GatewayMockedWithdrawalDestination::get())?;

            debug::info!(
                "Execute withdrawal: ({:?}, {:?}, {:?})",
                currency_id,
                amount,
                str::from_utf8(stellar_address.to_encoding().as_slice())?,
            );

            let imbalance = T::Currency::withdraw(currency_id, &pendulum_account_id, amount)
                .map_err(|_| <Error<T>>::BalanceChangeError)?;

            drop(imbalance);

            let seq_no = Self::fetch_latest_seq_no(escrow_address).map(|seq_no| seq_no + 1)?;
            let transaction =
                Self::create_withdrawal_tx(&stellar_address, seq_no as i64, asset, amount)?;
            let signed_envelope =
                Self::sign_stellar_tx(transaction, T::GatewayEscrowKeypair::get())?;

            let result = Self::submit_stellar_tx(signed_envelope);
            debug::info!(
                "✔️  Successfully submitted withdrawal transaction to Stellar, crediting {}",
                str::from_utf8(stellar_address.to_encoding().as_slice()).unwrap()
            );

            result
        }

        fn pop_queued_withdrawal(
        ) -> Result<Option<Withdrawal<BalanceOf<T>, CurrencyIdOf<T>, T::AccountId>>, Error<T>>
        {
            // TODO: Should use VecDeque or at least Vec instead, but not trivial to do using offchain index
            let mut pending_withdrawal_storage =
                sp_runtime::offchain::storage::StorageValueRef::persistent(
                    b"stellar-bridge::pending-withdrawal",
                );

            match pending_withdrawal_storage
                .get::<Withdrawal<BalanceOf<T>, CurrencyIdOf<T>, T::AccountId>>()
            {
                Some(Some(withdrawal)) => {
                    debug::info!(
                        "Found queued withdrawal. Clearing it from storage and returning it…",
                    );
                    pending_withdrawal_storage.clear();
                    Ok(Some(withdrawal))
                }
                _ => Ok(None),
            }
        }

        fn queue_withdrawal(
            pendulum_address: T::AccountId,
            currency: CurrencyIdOf<T>,
            amount: BalanceOf<T>,
        ) {
            let withdrawal = Withdrawal {
                amount,
                currency,
                pendulum_address,
            };
            sp_io::offchain_index::set(
                b"stellar-bridge::pending-withdrawal",
                withdrawal.encode().as_slice(),
            );
            debug::info!("Wrote withdrawal data into offchain worker storage.");
        }

        fn sign_stellar_tx(
            tx: stellar::types::Transaction,
            secret_key: SecretKey,
        ) -> Result<stellar::TransactionEnvelope, Error<T>> {
            let mut envelope = tx.into_transaction_envelope();
            envelope.sign(&TEST_NETWORK, vec![&secret_key])?;

            Ok(envelope)
        }

        fn submit_stellar_tx(tx: stellar::TransactionEnvelope) -> Result<(), Error<T>> {
            let mut last_error: Option<Error<T>> = None;

            for attempt in 1..=3 {
                debug::debug!("Attempt #{} to submit Stellar transaction…", attempt);

                match Self::try_once_submit_stellar_tx(&tx) {
                    Ok(result) => {
                        return Ok(result);
                    }
                    Err(error) => {
                        last_error = Some(error);
                    }
                }
            }

            // Can only panic if no submission was ever attempted
            Err(last_error.unwrap())
        }

        fn try_once_submit_stellar_tx(tx: &stellar::TransactionEnvelope) -> Result<(), Error<T>> {
            let horizon_base_url = "https://horizon-testnet.stellar.org";
            let horizon = stellar::horizon::Horizon::new(horizon_base_url);

            debug::info!(
                "Submitting transaction to Stellar network: {}",
                horizon_base_url
            );

            let _response = horizon
                .submit_transaction(&tx, SUBMISSION_TIMEOUT_PERIOD.millis(), true)
                .map_err(|error| {
                    match error {
                        stellar::horizon::FetchError::UnexpectedResponseStatus { status, body } => {
                            debug::error!("Unexpected HTTP request status code: {}", status);
                            debug::error!("  Response body: {}", str::from_utf8(&body).unwrap());
                        }
                        _ => (),
                    }
                    <Error<T>>::HttpFetchingError
                })?;

            Ok(())
        }

        fn fetch_from_remote(request_url: &str) -> Result<Response, Error<T>> {
            debug::info!("Sending request to: {}", request_url);

            let request = Request::get(request_url);
            let timeout = sp_io::offchain::timestamp().add(FETCH_TIMEOUT_PERIOD);

            let pending = request.deadline(timeout).send()?;

            let response = pending
                .try_wait(timeout)
                .map_err(|_| <Error<T>>::HttpFetchingError)?
                .map_err(|_| <Error<T>>::HttpFetchingError)?;

            if response.code != 200 {
                debug::error!("Unexpected HTTP request status code: {}", response.code);
                debug::error!(
                    "  Response body: {}",
                    str::from_utf8(response.body().collect::<Vec<u8>>().as_slice())?
                );
                return Err(<Error<T>>::HttpFetchingError);
            }

            Ok(response)
        }

        fn fetch_latest_seq_no(stellar_addr: &str) -> Result<u64, Error<T>> {
            let request_url =
                String::from("https://horizon-testnet.stellar.org/accounts/") + stellar_addr;

            let response = Self::fetch_from_remote(request_url.as_str()).map_err(|e| {
                debug::error!("fetch_latest_seq_no error: {:?}", e);
                <Error<T>>::HttpFetchingError
            })?;

            let json_bytes: Vec<u8> = response.body().collect::<Vec<u8>>();
            let resp_str =
                str::from_utf8(&json_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;

            // Deserializing JSON to struct, thanks to `serde` and `serde_derive`
            let horizon_response: HorizonAccountResponse =
                serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;

            String::from_utf8(horizon_response.sequence)
                .map(|string| string.parse::<u64>().unwrap())
                .map_err(|_| <Error<T>>::SeqNoParsingError)
        }

        /// Fetch recent transactions from remote and deserialize to HorizonResponse
        fn fetch_latest_txs() -> Result<HorizonTransactionsResponse, Error<T>> {
            let request_url = String::from("https://horizon-testnet.stellar.org/accounts/")
                + T::GatewayEscrowAccount::get()
                + "/transactions?order=desc&limit=1";

            let response = Self::fetch_from_remote(request_url.as_str()).map_err(|e| {
                debug::error!("fetch_latest_txs error: {:?}", e);
                <Error<T>>::HttpFetchingError
            })?;

            let json_bytes: Vec<u8> = response.body().collect::<Vec<u8>>();
            let resp_str =
                str::from_utf8(&json_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;

            // Deserializing JSON to struct, thanks to `serde` and `serde_derive`
            let horizon_response: HorizonTransactionsResponse =
                serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;

            Ok(horizon_response)
        }

        fn offchain_unsigned_tx_signed_payload(
            currency_id: CurrencyIdOf<T>,
            deposit: BalanceOf<T>,
            destination: T::AccountId,
        ) -> Result<(), Error<T>> {
            let signer = Signer::<T, T::AuthorityId>::any_account();

            if let Some((_, res)) = signer.send_unsigned_transaction(
                |acct| DepositPayload {
                    currency_id: currency_id,
                    amount: deposit,
                    destination: destination.clone(),
                    signed_by: acct.public.clone(),
                },
                Call::submit_deposit_unsigned_with_signed_payload,
            ) {
                return res.map_err(|_| {
                    debug::error!("Failed in offchain_unsigned_tx_signed_payload");
                    Error::OffchainUnsignedTxSignedPayloadError
                });
            } else {
                // The case of `None`: no account is available for sending
                debug::error!("No local account available");
                Err(Error::NoLocalAcctForSigning)
            }
        }

        fn handle_new_transaction(tx: &Transaction) {
            const UP_TO_DATE: () = ();

            let latest_tx_id_utf8 = &tx.id;

            let id_storage = sp_runtime::offchain::storage::StorageValueRef::persistent(
                b"stellar-bridge:last-tx-id",
            );

            let prev_read = id_storage.get::<Vec<u8>>();
            let initial = !matches!(prev_read, Some(Some(_)));

            let res = id_storage.mutate(|last_stored_tx_id: Option<Option<Vec<u8>>>| {
                match last_stored_tx_id {
                    Some(Some(value)) if value == *latest_tx_id_utf8 => Err(UP_TO_DATE),
                    _ => Ok(latest_tx_id_utf8.clone()),
                }
            });

            match res {
                Ok(Ok(saved_tx_id)) => {
                    if !initial {
                        debug::info!("✴️  New transaction from Horizon (id {:#?}). Starting to replicate transaction in Pendulum.", str::from_utf8(&saved_tx_id).unwrap());

                        let mut amount: Option<BalanceOf<T>> = None;
                        let destination = T::GatewayMockedDestination::get();
                        let mut currency = None;

                        // Decode transaction to Base64 and then to Stellar XDR to get transaction details
                        let tx_xdr = base64::decode(&tx.envelope_xdr).unwrap();
                        let tx_envelope = stellar::TransactionEnvelope::from_xdr(&tx_xdr).unwrap();

                        if let stellar::TransactionEnvelope::EnvelopeTypeTx(env) = tx_envelope {
                            // Source account will be our destination account
                            if let stellar::MuxedAccount::KeyTypeEd25519(key) = env.tx.source_account {
                                let pubkey = stellar::PublicKey::from_binary(key).to_encoding();
                                match str::from_utf8(&pubkey) {
                                    Ok(stellar_account_id) => debug::info!(
                                        "✔️  Source account is a valid Stellar account {:?}",
                                        stellar_account_id
                                    ),
                                    Err(_err) => debug::error!(
                                        "❌  Source account is a not a valid Stellar account."
                                    ),
                                }
                            }

                            for op in env.tx.operations.get_vec() {
                                if let stellar::types::OperationBody::Payment(payment_op) = &op.body
                                {
                                    let dest_account =
                                        stellar::MuxedAccount::from(payment_op.destination.clone());
                                    debug::info!(
                                        "Muxed account {}",
                                        str::from_utf8(dest_account.to_encoding().as_slice())
                                            .unwrap()
                                    );

                                    if let stellar::MuxedAccount::KeyTypeEd25519(dest_unwrapped) =
                                        payment_op.destination
                                    {
                                        let pubkey = stellar::PublicKey::from_binary(dest_unwrapped)
                                            .to_encoding();
                                        let pubkey_str = str::from_utf8(&pubkey).unwrap();
                                        if pubkey_str.eq(T::GatewayEscrowAccount::get()) {
                                            debug::info!(
                                                "✔️  Destination account is the escrow account {:?}",
                                                pubkey_str
                                            );
                                        }
                                    }

                                    if let stellar::Asset::AssetTypeCreditAlphanum4(code) =
                                        &payment_op.asset
                                    {
                                        let asset_code = str::from_utf8(&code.asset_code).ok();
                                        debug::info!("✔️  Asset code to be minted {:?}", asset_code);
                                        currency = Some(T::CurrencyConversion::unlookup(code.asset_code));
                                        debug::info!("currency {:#?}", currency);
                                        amount =
                                            Some(T::BalanceConversion::unlookup(payment_op.amount));
                                        debug::info!("Amount {:#?}", amount);
                                    }
                                }
                            }
                        }

                        if currency.is_some() && amount.is_some() {
                            match Self::offchain_unsigned_tx_signed_payload(
                                currency.unwrap(),
                                amount.unwrap(),
                                destination,
                            ) {
                                Err(_) => debug::warn!("Sending the tx failed."),
                                Ok(_) => (),
                            }
                        }
                    }
                }
                Err(UP_TO_DATE) => {
                    debug::info!("Already up to date");
                }
                Ok(Err(_)) => {
                    debug::info!("Failed to save last transaction id.");
                }
            }
        }
    }

    impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("stellar-bridge")
                    .priority(UNSIGNED_TXS_PRIORITY)
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(true)
                    .build()
            };

            match call {
                Call::submit_deposit_unsigned_with_signed_payload(ref payload, ref signature) => {
                    if !SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone()) {
                        return InvalidTransaction::BadProof.into();
                    }
                    valid_tx(b"submit_deposit_unsigned_with_signed_payload".to_vec())
                }
                _ => InvalidTransaction::Call.into(),
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        // Error returned when not sure which ocw function to executed
        UnknownOffchainMux,

        // Error returned when making signed transactions in off-chain worker
        NoLocalAcctForSigning,
        OffchainSignedTxError,

        // Error returned when making unsigned transactions in off-chain worker
        OffchainUnsignedTxError,

        // Error returned when making unsigned transactions with signed payloads in off-chain worker
        OffchainUnsignedTxSignedPayloadError,

        // Failed to change a balance
        BalanceChangeError,

        // Failed to convert an amount or balance
        BalanceConversionError,

        // Error returned when fetching remote info
        HttpFetchingError,

        // Stellar XDR array size error
        ExceedsMaximumLengthError,

        // Could not parse sequence no.
        SeqNoParsingError,

        // Could not parse Stellar public key
        StellarAddressParsingError,

        // Some Stellar SDK error
        StellarSdkError,

        // Some charset encoding/decoding error
        Utf8Error,

        // XDR encoding/decoding error
        XdrCodecError,
    }

    impl<T> From<StellarSdkError> for Error<T> {
        fn from(error: StellarSdkError) -> Self {
            match error {
                StellarSdkError::ExceedsMaximumLength { .. } => Self::ExceedsMaximumLengthError,
                _ => Self::StellarSdkError,
            }
        }
    }

    impl<T> From<HttpError> for Error<T> {
        fn from(_: HttpError) -> Self {
            Self::HttpFetchingError
        }
    }

    impl<T> From<LookupError> for Error<T> {
        fn from(_: LookupError) -> Self {
            Self::BalanceConversionError
        }
    }

    impl<T> From<Utf8Error> for Error<T> {
        fn from(_: Utf8Error) -> Self {
            Self::Utf8Error
        }
    }
}
