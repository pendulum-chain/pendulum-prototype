use frame_support::error::LookupError;
use sp_runtime::traits::StaticLookup;
use pendulum_common::currency::CurrencyId;

pub struct CurrencyConversion;

impl StaticLookup for CurrencyConversion {
    type Source = [u8; 4];
    type Target = CurrencyId;

    fn lookup(currency_bytes: Self::Source) -> Result<Self::Target, LookupError> {
        if currency_bytes.len() > 4 {
            Err(LookupError)
        } 
        Ok(CurrencyId::create_from_bytes(currency_bytes))
    }

    fn unlookup(currency_id: Self::Target) -> Self::Source {
        match currency_id {
            CurrencyId::TokenSymbol(val) => val,
            _ => [0, 0, 0, 0]
        }
    }
}
