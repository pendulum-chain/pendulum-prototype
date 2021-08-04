#[derive(Copy, Clone, Debug, PartialEq, Eq, codec::Decode, codec::Encode)]
pub enum StellarAsset {
    Native,
    AlphaNum4 {
        asset_code: [u8; 4],
        asset_issuer: [u8; 32],
    },
    AlphaNum12 {
        asset_code: [u8; 12],
        asset_issuer: [u8; 32],
    },
}

impl Default for StellarAsset {
    fn default() -> Self {
        StellarAsset::Native
    }
}

#[derive(codec::Decode)]
pub struct CompactAsset(pub StellarAsset);

#[derive(codec::Encode)]
pub struct CompactAssetRef<'a>(pub &'a StellarAsset);

impl<'a> From<&'a StellarAsset> for CompactAssetRef<'a> {
    fn from(x: &'a StellarAsset) -> Self {
        CompactAssetRef(x)
    }
}

impl From<StellarAsset> for CompactAsset {
    fn from(x: StellarAsset) -> CompactAsset {
        CompactAsset(x)
    }
}

impl From<CompactAsset> for StellarAsset {
    fn from(f: CompactAsset) -> Self {
        f.0
    }
}

impl codec::HasCompact for StellarAsset {
    type Type = CompactAsset;
}

impl<'a> codec::EncodeAsRef<'a, StellarAsset> for CompactAsset {
    type RefType = CompactAssetRef<'a>;
}