use sp_std::prelude::*;

use codec::{Decode, Encode};
use serde::{Deserialize, Deserializer};

// This represents each record for a transaction in the Horizon API response
#[derive(Deserialize, Encode, Decode, Default, Debug)]
pub struct Transaction {
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub id: Vec<u8>,
    successful: bool,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub hash: Vec<u8>,
    ledger: u32,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub created_at: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub source_account: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub source_account_sequence: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub fee_account: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub fee_charged: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub max_fee: Vec<u8>,
    operation_count: u32,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub envelope_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub result_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub result_meta_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub fee_meta_xdr: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub memo_type: Vec<u8>,
}

// The following structs represent the whole response when fetching any Horizon API
// In this particular case we asunme the embedded payload will allways be for transactions
// ref https://developers.stellar.org/api/introduction/response-format/
#[derive(Deserialize, Debug)]
pub struct EmbeddedTransactions {
    pub records: Vec<Transaction>,
}

#[derive(Deserialize, Debug)]
pub struct HorizonAccountResponse {
    // We don't care about specifics of pagination, so we just tell serde that this will be a generic json value
    pub _links: serde_json::Value,

    #[serde(deserialize_with = "de_string_to_bytes")]
    pub id: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub account_id: Vec<u8>,
    #[serde(deserialize_with = "de_string_to_bytes")]
    pub sequence: Vec<u8>,
    // ...
}

#[derive(Deserialize, Debug)]
pub struct HorizonTransactionsResponse {
    // We don't care about specifics of pagination, so we just tell serde that this will be a generic json value
    pub _links: serde_json::Value,
    pub _embedded: EmbeddedTransactions,
}

pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(de)?;
    Ok(s.as_bytes().to_vec())
}
