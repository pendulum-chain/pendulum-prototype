use sp_runtime::RuntimeDebug;

use codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    Native,
    TokenSymbol { code: [u8; 4] },
}

pub trait StellarAsset {
    fn asset_code(&self) -> Option<[u8; 4]>;
}

impl StellarAsset for CurrencyId {
    fn asset_code(&self) -> Option<[u8; 4]> {
        match self {
            CurrencyId::TokenSymbol { code} => Some(*code),
            _ => None,
        }
    }
}

impl CurrencyId {
    pub const fn create_from_slice(code: &str) -> Self {
        let bytes = code.as_bytes();
        let byte4 = if bytes.len() >= 4 { bytes[3] } else { 0 }; 
        CurrencyId::TokenSymbol { code: [bytes[0], bytes[1], bytes[2], byte4] }
    }

    pub const fn create_from_bytes(code:[u8; 4] ) -> Self {
        CurrencyId::TokenSymbol { code }
    }
}

impl Default for CurrencyId {
    fn default() -> Self {
        CurrencyId::Native
    }
}
