//! A PoC offchain worker that fetches data from Stellar Horizon Servers

#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::traits::{Currency, Get};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult,
};
use parity_scale_codec::{Decode, Encode};

use substrate_stellar_xdr::{xdr, xdr_codec::XdrCodec};

use frame_system::{
    ensure_none,
    offchain::{
        AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
        SigningTypes,
    },
};
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::{http::Request, storage::StorageValueRef, Duration},
    transaction_validity::{
        InvalidTransaction, TransactionSource, TransactionValidity, ValidTransaction,
    },
    RuntimeDebug,
};
use sp_std::{collections::vec_deque::VecDeque, prelude::*, str};

use serde::{Deserialize, Deserializer};

use pallet_balances::{Config as BalancesConfig, Pallet as BalancesPallet};
use pallet_transaction_payment::Config as PaymentConfig;

pub type Balance = u128;

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"abcd");

pub const HTTP_REMOTE_REQUEST: &str = "https://horizon-testnet.stellar.org/accounts/GAIMY7QQDWDQLX3KH6KFR25JLRJS4VGXFKLTRK66MPI6VPU3YDOPS6KQ/transactions?order=desc&limit=1";
pub const FETCH_TIMEOUT_PERIOD: u64 = 3000; // in milli-seconds

const UNSIGNED_TXS_PRIORITY: u64 = 100;

/// Based on the above `KeyTypeId` we need to generate a pallet-specific crypto type wrapper.
/// We can utilize the supported crypto kinds (`sr25519`, `ed25519` and `ecdsa`) and augment
/// them with the pallet-specific identifier.
pub mod crypto {
    use crate::KEY_TYPE;
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

// This represents each record for a transaction in the Horizon API response
#[derive(Deserialize, Encode, Decode, Default, Debug)]
pub struct Transaction {
    #[serde(deserialize_with = "de_string_to_bytes")]
    id: Vec<u8>,
    successful: bool,
    #[serde(deserialize_with = "de_string_to_bytes")]
    hash: Vec<u8>,
    ledger: u32,
    #[serde(deserialize_with = "de_string_to_bytes")]
    created_at: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    source_account: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    source_account_sequence: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    fee_account: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    fee_charged: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    max_fee: Vec<u8>,
    operation_count: u32,
    #[serde(deserialize_with = "de_string_to_bytes")]
    envelope_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    result_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    result_meta_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    fee_meta_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    memo_type: Vec<u8>,
}

// The following structs represent the whole response when fetching any Horizon API
// In this particular case we asunme the embedded payload will allways be for transactions
// ref https://developers.stellar.org/api/introduction/response-format/
#[derive(Deserialize, Debug)]
pub struct HorizonEmbeddedPayload {
    records: Vec<Transaction>,
}

#[derive(Deserialize, Debug)]
pub struct HorizonResponse {
    // We don't care about specifics of pagination, so we just tell serde that this will be a generic json value
    _links: serde_json::Value,
    _embedded: HorizonEmbeddedPayload,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Payload<AccountId, Public, Balance> {
    deposit: Balance,
    destination: AccountId,
    signed_by: Public,
}

impl<T: SigningTypes> SignedPayload<T> for Payload<T::AccountId, T::Public, T::Balance>
where
    T: BalancesConfig,
{
    fn public(&self) -> T::Public {
        self.signed_by.clone()
    }
}

#[derive(Debug, Deserialize, Encode, Decode, Default)]
struct IndexingData(Vec<u8>, u64);

pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(de)?;
    Ok(s.as_bytes().to_vec())
}

/// This is the pallet's configuration trait
pub trait Config:
    frame_system::Config + CreateSignedTransaction<Call<Self>> + BalancesConfig + PaymentConfig
{
    type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
    /// The overarching dispatch call type.
    type Call: From<Call<Self>>;
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    type GatewayMockedAmount: Get<<Self as pallet_balances::Config>::Balance>;
    type GatewayMockedDestination: Get<<Self as frame_system::Config>::AccountId>;
}

decl_storage! {
    trait Store for Module<T: Config> as Call {
        /// A vector of recently submitted numbers. Bounded by NUM_VEC_LEN
        Numbers get(fn numbers): VecDeque<u64>;
    }
}

decl_event!(
    /// Events generated by the module.
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Balance = <T as BalancesConfig>::Balance,
    {
        /// Event generated when a new deposit is made on Stellar Escrow Account.
        Deposit(AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
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

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 10000]
        pub fn submit_deposit_unsigned_with_signed_payload(origin, payload: Payload<T::AccountId, T::Public, T::Balance>,
            _signature: T::Signature) -> DispatchResult
        {
            let _ = ensure_none(origin)?;
            // FIXME: Verify signature
            // ~~we don't need to verify the signature here because it has been verified in
            //   `validate_unsigned` function when sending out the unsigned tx.~~
            let Payload { deposit, destination, signed_by } = payload;
            debug::info!("submit_deposit_unsigned_with_signed_payload: ({:?}, {:?}, {:?})", deposit, destination, signed_by);

            let imbalance = <BalancesPallet<T, _>>::deposit_creating(&destination, deposit);
            drop(imbalance);

            Self::deposit_event(RawEvent::Deposit(destination, deposit));
            Ok(())
        }

        fn offchain_worker(_n: T::BlockNumber) {
            const UP_TO_DATE: () = ();

            debug::info!("Hello from an offchain worker 👋");

            let res = Self::fetch_n_parse();
            let transactions = &res.unwrap()._embedded.records;

            let id_storage = StorageValueRef::persistent(b"stellar-watch:last-tx-id");

            let fetched_last_tx_id_utf8 = transactions[0].id.clone();
            let fetched_last_tx_id = str::from_utf8(&transactions[0].id).unwrap();
            let t: &Transaction = &transactions[0];
            let xdr = base64::decode(&t.envelope_xdr).unwrap();
            let envelope = xdr::TransactionEnvelope::from_xdr(&xdr);

            match envelope {
                Ok(env) => debug::info!("{:#?}", env),
                Err(err) => debug::info!("{:#?}", err)
            }

            let prev_read = id_storage.get::<Vec<u8>>();
            let initial = !matches!(prev_read, Some(Some(_)));

            let res = id_storage.mutate(|last_tx_id: Option<Option<Vec<u8>>>| {
                match last_tx_id {
                    Some(Some(value)) if str::from_utf8(&value).unwrap() == fetched_last_tx_id => {
                        Err(UP_TO_DATE)
                    },
                    _ => Ok(fetched_last_tx_id_utf8)
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
                        debug::info!("New transaction from Horizon (id {:#?}). Starting to replicate transaction in Pendulum.", str::from_utf8(&saved_tx_id).unwrap());

                        let amount = T::GatewayMockedAmount::get();
                        let destination = T::GatewayMockedDestination::get();
                        Self::offchain_unsigned_tx_signed_payload(amount, destination).unwrap();
                    }
                },
                // The transaction id is the same as before.
                Err(UP_TO_DATE) => {
                    debug::info!("Already up to date");
                },
                // We failed to acquire a lock. This indicates that another offchain worker that was running concurrently
                // most likely executed the same logic and succeeded at writing to storage.
                // We don't do anyhting by now, but ideally we should queue transaction ids for processing.
                Ok(Err(_)) => {
                    debug::info!("Failed to save last transaction id.");
                }
            }
        }
    }
}

impl<T: Config> Module<T> {
    fn fetch_from_remote() -> Result<Vec<u8>, Error<T>> {
        debug::info!("sending request to: {}", HTTP_REMOTE_REQUEST);

        let request = Request::get(HTTP_REMOTE_REQUEST);

        let timeout = sp_io::offchain::timestamp().add(Duration::from_millis(FETCH_TIMEOUT_PERIOD));

        let pending = request
            .deadline(timeout)
            .send()
            .map_err(|_| <Error<T>>::HttpFetchingError)?;

        let response = pending
            .try_wait(timeout)
            .map_err(|_| <Error<T>>::HttpFetchingError)?
            .map_err(|_| <Error<T>>::HttpFetchingError)?;

        if response.code != 200 {
            debug::error!("Unexpected HTTP request status code: {}", response.code);
            return Err(<Error<T>>::HttpFetchingError);
        }

        let json_result: Vec<u8> = response.body().collect::<Vec<u8>>();

        Ok(json_result)
    }

    /// Fetch from remote and deserialize to HorizonResponse
    fn fetch_n_parse() -> Result<HorizonResponse, Error<T>> {
        let resp_bytes = Self::fetch_from_remote().map_err(|e| {
            debug::error!("fetch_from_remote error: {:?}", e);
            <Error<T>>::HttpFetchingError
        })?;

        let resp_str = str::from_utf8(&resp_bytes).map_err(|_| <Error<T>>::HttpFetchingError)?;

        // Deserializing JSON to struct, thanks to `serde` and `serde_derive`
        let horizon_response: HorizonResponse =
            serde_json::from_str(&resp_str).map_err(|_| <Error<T>>::HttpFetchingError)?;
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
            |acct| Payload {
                deposit,
                destination: destination.clone(),
                signed_by: acct.public.clone(),
            },
            Call::submit_deposit_unsigned_with_signed_payload,
        ) {
            return res.map_err(|_| {
                debug::error!("Failed in offchain_unsigned_tx_signed_payload");
                <Error<T>>::OffchainUnsignedTxSignedPayloadError
            });
        } else {
            // The case of `None`: no account is available for sending
            debug::error!("No local account available");
            Err(<Error<T>>::NoLocalAcctForSigning)
        }
    }
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
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
