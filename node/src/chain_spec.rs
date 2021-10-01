use pendulum_node_runtime::{
    AccountId, AuraConfig, BalancesConfig, ContractsConfig, CurrencyId, GenesisConfig,
    GrandpaConfig, Signature, SudoConfig, SystemConfig, TokensConfig, WASM_BINARY,
};
use sc_service::ChainType;
use sp_consensus_aura::ed25519::AuthorityId as AuraId;
use sp_core::{ed25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_std::convert::TryFrom;
use substrate_stellar_sdk as stellar;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn development_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Development",
        // ID
        "dev",
        ChainType::Development,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![authority_keys_from_seed("Alice")],
                // Sudo account
                get_account_id_from_seed::<ed25519::Public>("Alice"),
                // Pre-funded accounts
                vec![
                    get_account_id_from_seed::<ed25519::Public>("Alice"),
                    get_account_id_from_seed::<ed25519::Public>("Bob"),
                    get_account_id_from_seed::<ed25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<ed25519::Public>("Bob//stash"),
                ],
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    Ok(ChainSpec::from_genesis(
        // Name
        "Local Testnet",
        // ID
        "local_testnet",
        ChainType::Local,
        move || {
            testnet_genesis(
                wasm_binary,
                // Initial PoA authorities
                vec![
                    authority_keys_from_seed("Alice"),
                    authority_keys_from_seed("Bob"),
                ],
                // Sudo account
                get_account_id_from_seed::<ed25519::Public>("Alice"),
                // Pre-funded accounts
                vec![
                    get_account_id_from_seed::<ed25519::Public>("Alice"),
                    get_account_id_from_seed::<ed25519::Public>("Bob"),
                    get_account_id_from_seed::<ed25519::Public>("Charlie"),
                    get_account_id_from_seed::<ed25519::Public>("Dave"),
                    get_account_id_from_seed::<ed25519::Public>("Eve"),
                    get_account_id_from_seed::<ed25519::Public>("Ferdie"),
                    get_account_id_from_seed::<ed25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<ed25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<ed25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<ed25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<ed25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<ed25519::Public>("Ferdie//stash"),
                ],
                true,
            )
        },
        // Bootnodes
        vec![],
        // Telemetry
        None,
        // Protocol ID
        None,
        // Properties
        None,
        // Extensions
        None,
    ))
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    wasm_binary: &[u8],
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    let stellar_usdc_asset: CurrencyId = CurrencyId::try_from((
        "USDC",
        stellar::PublicKey::from_encoding(
            "GAKNDFRRWA3RPWNLTI3G4EBSD3RGNZZOY5WKWYMQ6CQTG3KIEKPYWAYC",
        )
        .unwrap()
        .as_binary()
        .clone(),
    ))
    .unwrap();

    GenesisConfig {
        frame_system: Some(SystemConfig {
            // Add Wasm runtime to storage.
            code: wasm_binary.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            // Configure endowed accounts with initial balance of 1 << 60.
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 100_000_000_000_000_000_000u128))
                .collect(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        pallet_contracts: Some(ContractsConfig {
            current_schedule: pallet_contracts::Schedule::default(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        pallet_sudo: Some(SudoConfig {
            // Assign network admin rights.
            key: root_key,
        }),
        orml_tokens: Some(TokensConfig {
            endowed_accounts: endowed_accounts
                .iter()
                .flat_map(|x| vec![(x.clone(), stellar_usdc_asset, 10u128.pow(12))])
                .collect(),
        }),
    }
}
