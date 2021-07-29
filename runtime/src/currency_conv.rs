use frame_support::error::LookupError;
use sp_runtime::traits::StaticLookup;
use pendulum_common::currency::CurrencyId;

pub struct CurrencyConversion;

impl StaticLookup for CurrencyConversion {
    type Source = CurrencyId;
    type Target = [u8; 4];

    fn lookup(currency_id: Self::Source) -> Result<Self::Target, LookupError> {
        match currency_id {
            CurrencyId::TokenSymbol { code } => Ok(code),
            _ => Err(LookupError)
        }
    }

    fn unlookup(currency_bytes: Self::Target) -> Self::Source {
        CurrencyId::create_from_bytes(currency_bytes)
    }
}
