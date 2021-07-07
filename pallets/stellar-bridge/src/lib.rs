//! A PoC offchain worker that fetches data from Stellar Horizon Servers

#![cfg_attr(not(feature = "std"), no_std)]

mod horizon;
mod string;

use codec::{Decode, Encode};

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;

use frame_system::offchain::{SignedPayload, SigningTypes};
use sp_core::crypto::KeyTypeId;
use sp_runtime::RuntimeDebug;
use sp_std::{prelude::*, str};

use serde::Deserialize;
use string::String;

use pallet_balances::{Config as BalancesConfig, Pallet as BalancesPallet};
use pallet_transaction_payment::Config as PaymentConfig;

use frame_support::traits::Currency;

use self::horizon::*;

// Re-export pallet items so that they can be accessed from the crate namespace.
// pub use pallet::*;

pub type Balance = u128;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

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
pub struct DepositPayload<AccountId, Public, Balance> {
    amount: Balance,
    destination: AccountId,
    signed_by: Public,
}

impl<T: SigningTypes> SignedPayload<T> for DepositPayload<T::AccountId, T::Public, T::Balance>
where
    T: BalancesConfig,
{
    fn public(&self) -> T::Public {
        self.signed_by.clone()
    }
}

#[derive(Debug, Deserialize, Encode, Decode, Default)]
struct IndexingData(Vec<u8>, u64);

pub use pallet::*;

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
        frame_system::Config + CreateSignedTransaction<Call<Self>> + BalancesConfig + PaymentConfig
    {
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
        /// The overarching dispatch call type.
        type Call: From<Call<Self>>;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        type GatewayEscrowAccount: Get<&'static str>;
        type GatewayMockedAmount: Get<<Self as pallet_balances::Config>::Balance>;
        type GatewayMockedDestination: Get<<Self as frame_system::Config>::AccountId>;
    }

    /// Events are a simple means of reporting specific conditions and
    /// circumstances that have happened that users, Dapps and/or chain explorers would find
    /// interesting and otherwise difficult to detect.
    #[pallet::event]
    /// This attribute generate the function `deposit_event` to deposit one of this pallet event,
    /// it is optional, it is also possible to provide a custom implementation.
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    // #[pallet::generate_withdrawal(pub(super) fn withdrawal_event)]
    pub enum Event<T: Config> {
        /// Event generated when a new deposit is made on Stellar Escrow Account.
        Deposit(T::AccountId, BalanceOf<T>),

        /// Event generated when a new withdrawal has been completed on Stellar.
        Withdrawal(T::AccountId, BalanceOf<T>),
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
            sp_tracing::info!("Hello from an offchain worker 👋");

            let res = Self::fetch_n_parse();
            let transactions = &res.unwrap()._embedded.records;

            if transactions.len() > 0 {
                Self::handle_new_transaction(&transactions[0].id);
            }
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // This is your public interface. Be extremely careful.
        #[pallet::weight(10000)]
        pub fn submit_deposit_unsigned_with_signed_payload(
            origin: OriginFor<T>,
            payload: DepositPayload<T::AccountId, T::Public, T::Balance>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_none(origin)?;

            // FIXME: Verify signature
            // ~~we don't need to verify the signature here because it has been verified in
            //   `validate_unsigned` function when sending out the unsigned tx.~~
            let DepositPayload {
                amount,
                destination,
                signed_by,
            } = payload;
            sp_tracing::info!(
                "submit_deposit_unsigned_with_signed_payload: ({:?}, {:?}, {:?})",
                amount,
                destination,
                signed_by
            );

            let imbalance = <BalancesPallet<T, _>>::deposit_creating(&destination, amount);
            drop(imbalance);

            Self::deposit_event(Event::Deposit(destination, amount));
            Ok(().into())
        }

        #[pallet::weight(100000)]
        pub fn withdraw_to_stellar(
            origin: OriginFor<T>,
            _amount: T::Balance,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            let _pendulum_address = ensure_signed(origin)?;

            // TODO: Deduct amount from account
            // TODO: Create Stellar tx
            // TODO: Sign & submit Stellar tx
            // TODO: Retry submission if necessary

            // Self::withdrawal_event(Event::Withdrawal(source_address, amount));
            // Err(sp_runtime::DispatchError::Other("Not yet implemented"))
            unimplemented!();
        }
    }

    impl<T: Config> Pallet<T> {
        fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
            let request_url = String::from("https://horizon-testnet.stellar.org/accounts/")
                + T::GatewayEscrowAccount::get()
                + "/transactions?order=desc&limit=1";

            sp_tracing::info!("Sending request to: {}", request_url.as_str());

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
                sp_tracing::error!("Unexpected HTTP request status code: {}", response.code);
                return Err(Error::HttpFetchingError);
            }

            let json_result: Vec<u8> = response.body().collect::<Vec<u8>>();

            Ok(json_result)
        }

        /// Fetch from remote and deserialize to HorizonResponse
        fn fetch_n_parse() -> Result<HorizonResponse, Error<T>> {
            let resp_bytes = Self::fetch_from_remote().map_err(|e| {
                sp_tracing::error!("fetch_from_remote error: {:?}", e);
                Error::HttpFetchingError
            })?;

            let resp_str = str::from_utf8(&resp_bytes).map_err(|_| Error::HttpFetchingError)?;

            // Deserializing JSON to struct, thanks to `serde` and `serde_derive`
            let horizon_response: HorizonResponse =
                serde_json::from_str(&resp_str).map_err(|_| Error::HttpFetchingError)?;

            Ok(horizon_response)
        }

        fn offchain_unsigned_tx_signed_payload(
            deposit: T::Balance,
            destination: T::AccountId,
        ) -> Result<(), Error<T>> {
            // Retrieve the signer to sign the payload
            let signer = Signer::<T, T::AuthorityId>::any_account();

            // `send_unsigned_transaction` is returning a type of `Option<(Account<T>, Result<(), ()>)>`.
            //   Similar to `send_signed_transaction`, they account for:
            //   - `None`: no account is available for sending transaction
            //   - `Some((account, Ok(())))`: transaction is successfully sent
            //   - `Some((account, Err(())))`: error occured when sending the transaction
            if let Some((_, res)) = signer.send_unsigned_transaction(
                |acct| DepositPayload {
                    amount: deposit,
                    destination: destination.clone(),
                    signed_by: acct.public.clone(),
                },
                Call::submit_deposit_unsigned_with_signed_payload,
            ) {
                return res.map_err(|_| {
                    sp_tracing::error!("Failed in offchain_unsigned_tx_signed_payload");
                    Error::OffchainUnsignedTxSignedPayloadError
                });
            } else {
                // The case of `None`: no account is available for sending
                sp_tracing::error!("No local account available");
                Err(Error::NoLocalAcctForSigning)
            }
        }

        fn handle_new_transaction(latest_tx_id_utf8: &Vec<u8>) {
            const UP_TO_DATE: () = ();

            let id_storage = StorageValueRef::persistent(b"stellar-watch:last-tx-id");

            let prev_read = id_storage.get::<Vec<u8>>();
            let initial = !matches!(prev_read, Some(Some(_)));

            let res = id_storage.mutate(|last_stored_tx_id: Option<Option<Vec<u8>>>| {
                match last_stored_tx_id {
                    Some(Some(value)) if value == *latest_tx_id_utf8 => Err(UP_TO_DATE),
                    _ => Ok(latest_tx_id_utf8.clone()),
                }
            });

            // The result of `mutate` call will give us a nested `Result` type.
            // The first one matches the return of the closure passed to `mutate`, i.e.
            // if we return `Err` from the closure, we get an `Err` here.
            // In case we return `Ok`, here we will have another (inner) `Result` that indicates
            // if the value has been set to the storage correctly - i.e. if it wasn't
            // written to in the meantime.
            match res {
                // The value has been set correctly.
                Ok(Ok(saved_tx_id)) => {
                    if !initial {
                        sp_tracing::info!("✴️  New transaction from Horizon (id {:#?}). Starting to replicate transaction in Pendulum.", str::from_utf8(&saved_tx_id).unwrap());

                        let amount = T::GatewayMockedAmount::get();
                        let destination = T::GatewayMockedDestination::get();

                        match Self::offchain_unsigned_tx_signed_payload(amount, destination) {
                            Err(_) => sp_tracing::warn!("Sending the tx failed."),
                            Ok(_) => (),
                        }
                    }
                }
                // The transaction id is the same as before.
                Err(UP_TO_DATE) => {
                    sp_tracing::info!("Already up to date");
                }
                // We failed to acquire a lock. This indicates that another offchain worker that was running concurrently
                // most likely executed the same logic and succeeded at writing to storage.
                // We don't do anyhting by now, but ideally we should queue transaction ids for processing.
                Ok(Err(_)) => {
                    sp_tracing::info!("Failed to save last transaction id.");
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
