use frame_support::error::LookupError;
use pendulum_common::currency::CurrencyId;
use sp_runtime::traits::StaticLookup;
use sp_std::convert::TryInto;
use substrate_stellar_sdk::Asset;

pub struct CurrencyConversion;

fn to_look_up_error(_: &'static str) -> LookupError {
    LookupError
}

impl StaticLookup for CurrencyConversion {
    type Source = CurrencyId;
    type Target = Asset;

    fn lookup(
        currency_id: <Self as StaticLookup>::Source,
    ) -> Result<<Self as StaticLookup>::Target, LookupError> {
        let asset_conversion_result: Result<Asset, &str> = currency_id.try_into();
        asset_conversion_result.map_err(to_look_up_error)
    }

    fn unlookup(stellar_asset: <Self as StaticLookup>::Target) -> <Self as StaticLookup>::Source {
        CurrencyId::from(stellar_asset)
    }
}
