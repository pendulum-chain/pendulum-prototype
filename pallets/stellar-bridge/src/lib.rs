//! A PoC offchain worker that fetches data from Stellar Horizon Servers

#![cfg_attr(not(feature = "std"), no_std)]
mod horizon;

use codec::{Decode, Encode};

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_system::offchain::{SignedPayload, SigningTypes};
use sp_core::crypto::KeyTypeId;
use sp_runtime::traits::StaticLookup;
use sp_runtime::RuntimeDebug;

use sp_std::convert::From;

use sp_std::{prelude::*, str};

use orml_traits::{MultiCurrency, MultiReservableCurrency};

use serde::Deserialize;

pub use substrate_stellar_sdk as stellar;
pub use substrate_stellar_sdk::XdrCodec;

use pallet_transaction_payment::Config as PaymentConfig;

use self::horizon::*;

pub use pallet::*;

pub use pendulum_common::currency::CurrencyId;
use pendulum_common::string::String;

type BalanceOf<T> =
    <<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;

type CurrencyIdOf<T> =
    <<T as Config>::Currency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

// Re-export pallet items so that they can be accessed from the crate namespace.
// pub use pallet::*;

pub type Balance = u128;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"abcd");

pub const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds

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
    use frame_system::offchain::SendUnsignedTransaction;
    use frame_system::offchain::{AppCrypto, CreateSignedTransaction, Signer};
    use sp_runtime::offchain::http::Request;
    use sp_runtime::offchain::storage::StorageValueRef;
    use sp_runtime::offchain::Duration;

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

        /// Conversion between Stellar balance type and this pallet trait for balances
        type BalanceConversion: StaticLookup<Source = BalanceOf<Self>, Target = i64>;

        /// Conversion between Stellar asset type and this pallet trait for Currency
        type CurrencyConversion: StaticLookup<Source = CurrencyIdOf<Self>, Target = stellar::Asset>;

        /// Conversion between Stellar Address type and this pallet trait for AccountId
        type AddressConversion: StaticLookup<Source = Self::AccountId, Target = stellar::PublicKey>;

        /// The escrow account
        type GatewayEscrowSecretKey: Get<stellar::SecretKey>;
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

            let res = Self::fetch_n_parse();
            let transactions = &res.unwrap()._embedded.records;

            if transactions.len() > 0 {
                Self::handle_new_transaction(&transactions[0]);
            }
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
            _amount: BalanceOf<T>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            let _pendulum_address = ensure_signed(origin)?;

            // TODO: Deduct amount from account
            // TODO: Create Stellar tx
            // TODO: Sign & submit Stellar tx
            // TODO: Retry submission if necessary

            // Self::withdrawal_event(Event:T:Withdrawal(source_address, amount));
            // Err(sp_runtime::DispatchError::Other("Not yet implemented"))
            unimplemented!();
        }
    }

    impl<T: Config> Pallet<T> {
        fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
            let escrow_account_secret_key = T::GatewayEscrowSecretKey::get();
            let request_url = String::from("https://horizon-testnet.stellar.org/accounts/")
                + str::from_utf8(escrow_account_secret_key.get_encoded_public().as_slice())
                    .unwrap()
                + "/transactions?order=desc&limit=1";

            debug::info!("Sending request to: {}", request_url.as_str());

            let request = Request::get(request_url.as_str());
            let timeout =
                sp_io::offchain::timestamp().add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));

            let pending = request
                .deadline(timeout)
                .send()
                .map_err(|_| Error::HttpFetchingError)?;

            let response = pending
                .try_wait(timeout)
                .map_err(|_| Error::HttpFetchingError)?
                .map_err(|_| Error::HttpFetchingError)?;

            if response.code != 200 {
                debug::error!("Unexpected HTTP request status code: {}", response.code);
                return Err(Error::HttpFetchingError);
            }

            let json_result: Vec<u8> = response.body().collect::<Vec<u8>>();

            Ok(json_result)
        }

        /// Fetch from remote and deserialize to HorizonResponse
        fn fetch_n_parse() -> Result<HorizonResponse, Error<T>> {
            let resp_bytes = Self::fetch_from_remote().map_err(|e| {
                debug::error!("fetch_from_remote error: {:?}", e);
                Error::HttpFetchingError
            })?;

            let resp_str = str::from_utf8(&resp_bytes).map_err(|_| Error::HttpFetchingError)?;

            // Deserializing JSON to struct, thanks to `serde` and `serde_derive`
            let horizon_response: HorizonResponse =
                serde_json::from_str(&resp_str).map_err(|_| Error::HttpFetchingError)?;

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

        fn is_escrow(public_key: [u8; 32]) -> bool {
            return public_key == *T::GatewayEscrowSecretKey::get().get_public().as_binary();
        }

        fn handle_new_transaction(tx: &Transaction) {
            const UP_TO_DATE: () = ();

            let latest_tx_id_utf8 = &tx.id;

            let id_storage = StorageValueRef::persistent(b"stellar-watch:last-tx-id");

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
                        let mut destination: Option<T::AccountId> = None;
                        let mut currency = None;

                        // Decode transaction to Base64 and then to Stellar XDR to get transaction details
                        let tx_xdr = base64::decode(&tx.envelope_xdr).unwrap();
                        let tx_envelope = stellar::TransactionEnvelope::from_xdr(&tx_xdr).unwrap();

                        if let stellar::TransactionEnvelope::EnvelopeTypeTx(env) = tx_envelope {
                            // Source account will be our destination account
                            if let stellar::MuxedAccount::KeyTypeEd25519(key) =
                                env.tx.source_account
                            {
                                let pubkey = stellar::PublicKey::from_binary(key);
                                match str::from_utf8(&pubkey.to_encoding()) {
                                    Ok(_) => {
                                        debug::info!("✔️  Source account is a valid Stellar account {:?}", pubkey.as_binary());
                                        destination = Some(T::AddressConversion::unlookup(pubkey));
                                    },
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
                                    debug::info!("Muxed account {:#?}", dest_account);

                                    if let stellar::MuxedAccount::KeyTypeEd25519(
                                        payment_dest_public_key,
                                    ) = payment_op.destination
                                    {
                                        if Self::is_escrow(payment_dest_public_key) {
                                            amount = Some(T::BalanceConversion::unlookup(
                                                payment_op.amount,
                                            ));
                                            currency = Some(T::CurrencyConversion::unlookup(
                                                payment_op.asset.clone(),
                                            ));
                                        }
                                    }
                                    debug::info!("Pendulum address for deposit {:?}", destination);
                                    debug::info!("Currency {:?}", currency);
                                    debug::info!("Amount {:?}", amount);
                                }
                            }
                        }

                        if currency.is_some() && amount.is_some() && destination.is_some() {
                            match Self::offchain_unsigned_tx_signed_payload(
                                currency.unwrap(),
                                amount.unwrap(),
                                destination.unwrap(),
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
                ValidTransaction::with_tag_prefix("stellar-watch")
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

        // Error returned when fetching remote info
        HttpFetchingError,
    }
}
