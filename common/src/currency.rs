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
            CurrencyId::TokenSymbol { code } => Some(*code),
            _ => None,
        }
    }
}

impl CurrencyId {
    pub fn create_from_slice(slice: &str) -> Self {
        if slice.len() <= 4 {
            let mut code: [u8; 4] = [0; 4];
            code[..slice.len()].copy_from_slice(slice.as_bytes());
            CurrencyId::TokenSymbol { code }
        } else {
            panic!("More than 4 bytes not supported")
        }
    }

    pub fn create_from_bytes(code: [u8; 4]) -> Self {
        CurrencyId::TokenSymbol { code }
    }
}

impl Default for CurrencyId {
    fn default() -> Self {
        CurrencyId::Native
    }
}
