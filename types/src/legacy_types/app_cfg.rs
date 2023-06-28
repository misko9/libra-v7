//! Configs for all 0L apps.

use anyhow::Context;
// use tokio::sync::futures;
use crate::{
    exports::{AccountAddress, AuthenticationKey, NamedChain},
    global_config_dir,
    legacy_types::mode_ol::MODE_0L,
};
use serde::{Deserialize, Serialize};
use url::Url;
use zapatos_crypto::ed25519::Ed25519PrivateKey;

use std::{
    fs::{self, File},
    io::{Read, Write},
    net::Ipv4Addr,
    path::PathBuf,
    str::FromStr,
};

use super::network_playlist::{NetworkPlaylist, self};

// const NODE_HOME: &str = ".0L";
const CONFIG_FILE_NAME: &str = "0L.toml";

/// MinerApp Configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppCfg {
    /// Workspace config
    pub workspace: Workspace,
    /// User Profile
    pub profile: Profile,
    /// Network profile
    // NOTE: new field from V7, so it's an option so that previous files can load.
    pub network_playlist: Option<Vec<NetworkPlaylist>>,
    /// Chain Info for all users
    pub chain_info: ChainInfo,
    /// Transaction configurations
    pub tx_configs: TxConfigs,
}

pub fn default_file_path() -> PathBuf {
    global_config_dir().join(CONFIG_FILE_NAME)
}

impl AppCfg {
    /// load from default path
    pub fn load(file: Option<PathBuf>) -> anyhow::Result<Self> {
        let path = file.unwrap_or(default_file_path());
        Self::parse_toml(path)
    }

    /// Get a AppCfg object from toml file
    pub fn parse_toml(path: PathBuf) -> anyhow::Result<Self> {
        let parent_dir = path.parent().context("no parent directory")?;
        if !parent_dir.exists() {
            println!(
                "Directory for app configs {} doesn't exist, exiting.",
                &parent_dir.to_str().unwrap()
            );
        }
        let mut toml_buf = "".to_string();
        let mut file = File::open(&path)?;
        file.read_to_string(&mut toml_buf)?;

        Ok(toml::from_str(&toml_buf)?)
    }

    /// Get where the block/proofs are stored.
    pub fn get_block_dir(&self) -> PathBuf {
        let mut home = self.workspace.node_home.clone();
        home.push(&self.workspace.block_dir);
        home
    }

    /// Get where node key_store.json stored.
    pub fn init_app_configs(
        authkey: AuthenticationKey,
        account: AccountAddress,
        config_path: Option<PathBuf>,
        network_id: Option<NamedChain>,
    ) -> anyhow::Result<Self> {
        // TODO: Check if configs exist and warn on overwrite.
        let mut default_config = AppCfg::default();
        default_config.profile.auth_key = authkey;
        default_config.profile.account = account;

        default_config.workspace.node_home =
            config_path.clone().unwrap_or_else(|| default_file_path());

        if let Some(id) = network_id {
            default_config.chain_info.chain_id = id.to_owned();
        };

        // skip questionnaire if CI
        if MODE_0L.clone() == NamedChain::TESTING {
            default_config.save_file()?;

            return Ok(default_config);
        }

        Ok(default_config)
    }

    /// save the config file to 0L.toml to the workspace home path
    pub fn save_file(&self) -> anyhow::Result<PathBuf> {
        let toml = toml::to_string(&self)?;
        let home_path = &self.workspace.node_home.clone();
        // create home path if doesn't exist, usually only in dev/ci environments.
        fs::create_dir_all(&home_path)?;
        let toml_path = home_path.join(CONFIG_FILE_NAME);
        let mut file = fs::File::create(&toml_path)?;
        file.write(&toml.as_bytes())?;

        println!(
            "\nhost configs initialized, file saved to: {:?}",
            &toml_path
        );
        Ok(toml_path)
    }

    // /// Removes current node from upstream nodes
    // /// To be used when DB is corrupted for instance.
    // pub fn remove_node(&mut self, host: String) -> anyhow::Result<()> {
    //     let nodes = self.profile.upstream_nodes.clone();
    //     match nodes.len() {
    //         1 => bail!("Cannot remove last node"),
    //         _ => {
    //             self.profile.upstream_nodes = nodes
    //                 .into_iter()
    //                 .filter(|each| !each.to_string().contains(&host))
    //                 .collect();
    //             self.save_file()?;
    //             Ok(())
    //         }
    //     }
    // }

    pub async fn update_network_playlist(&mut self, chain_id: NamedChain) -> anyhow::Result<NetworkPlaylist>{
      let url = network_playlist::find_default_playlist(Some(chain_id))?;
      let np = NetworkPlaylist::from_url(url, None).await?;

      if let Some(playlist) = &mut self.network_playlist {
         for e in playlist.iter_mut(){
            if e.chain_id == Some(chain_id) { *e = np.clone(); }
         };
      } else { self.network_playlist = Some(vec![np.clone()])}

      Ok(np)

    }
    ///fetch a network profile, optionally by profile name
    pub fn get_network_profile(
        &self,
        chain_id: Option<NamedChain>,
    ) -> anyhow::Result<NetworkPlaylist> {
        // TODO: avoid clone
        let np = self
            .network_playlist
            .clone()
            .context("no network profiles set")?;

        let profile = if chain_id.is_some() {
            np.into_iter().find(|each| each.chain_id == chain_id)
        } else {
            np.into_iter().next()
        };

        profile.context("could not find a network profile")
    }

    pub async fn refresh_network_profile_and_save(
        &mut self,
        chain_id: Option<NamedChain>,
    ) -> anyhow::Result<NetworkPlaylist> {
        let mut np = self.get_network_profile(chain_id)?;
        np.refresh_sync_status().await?;
        self.save_file()?;
        Ok(np)
    }

    ///fetch a network profile, optionally by profile name
    pub fn best_url(&mut self, chain_id: Option<NamedChain>) -> anyhow::Result<Url> {
        let np = self.get_network_profile(chain_id)?;
        np.the_best_one()
    }
}

/// Default configuration settings.
impl Default for AppCfg {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            profile: Profile::default(),
            network_playlist: Some(vec![NetworkPlaylist::default()]),
            chain_info: ChainInfo::default(),
            tx_configs: TxConfigs::default(),
        }
    }
}

/// Information about the Chain to mined for
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Workspace {
    /// home directory of the diem node, may be the same as miner.
    pub node_home: PathBuf,
    /// Directory of source code (for developer tests only)
    pub source_path: Option<PathBuf>,
    /// Directory to store blocks in
    pub block_dir: String,
    /// Directory for the database
    #[serde(default = "default_db_path")]
    pub db_path: PathBuf,
    /// Path to which stdlib binaries for upgrades get built typically
    /// /language/diem-framework/staged/stdlib.mv
    pub stdlib_bin_path: Option<PathBuf>,
}

fn default_db_path() -> PathBuf {
    global_config_dir().join("db")
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            node_home: crate::global_config_dir(),
            source_path: None,
            block_dir: "vdf_proofs".to_owned(),
            db_path: default_db_path(),
            stdlib_bin_path: None,
        }
    }
}

/// Information about the Chain to mined for
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChainInfo {
    /// Chain that this work is being committed to
    pub chain_id: NamedChain,
}

// TODO: These defaults serving as test fixtures.
impl Default for ChainInfo {
    fn default() -> Self {
        Self {
            chain_id: NamedChain::MAINNET,
        }
    }
}

/// Miner profile to commit this work chain to a particular identity
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    /// The 0L account for the Miner and prospective validator. This is derived from auth_key
    pub account: AccountAddress,

    /// Miner Authorization Key for 0L Blockchain. Note: not the same as public key, nor account.
    pub auth_key: AuthenticationKey,

    /// An opportunity for the Miner to write a message on their genesis block.
    pub statement: String,

    /// ip address of this node. May be different from transaction URL.
    pub ip: Ipv4Addr,

    /// ip address of the validator fullnodee
    pub vfn_ip: Option<Ipv4Addr>,

    /// Other nodes to connect for fallback connections
    pub upstream_nodes: Vec<Url>,

    /// fullnode playlist URL to override default
    pub override_playlist: Option<Url>,

    /// Link to another delay tower.
    pub tower_link: Option<String>,

    /// Private key only for use with testing
    pub test_private_key: Option<Ed25519PrivateKey>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            account: AccountAddress::from_hex_literal("0x0").unwrap(),
            auth_key: AuthenticationKey::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            statement: "Protests rage across the nation".to_owned(),
            ip: "0.0.0.0".parse().unwrap(),
            vfn_ip: "0.0.0.0".parse().ok(),
            // default_node: Some("http://localhost:8080".parse().expect("parse url")),
            override_playlist: None,
            upstream_nodes: vec!["http://localhost:8080".parse().expect("parse url")],
            tower_link: None,
            test_private_key: None,
        }
    }
}

/// Transaction types
pub enum TxType {
    /// critical txs
    Critical,
    /// management txs
    Mgmt,
    /// miner txs
    Miner,
    /// cheap txs
    Cheap,
}

/// Transaction types used in 0L clients
#[derive(Clone, Debug, Deserialize, Serialize)]
// #[serde(deny_unknown_fields)]
pub struct TxConfigs {
    /// baseline cost
    #[serde(default = "default_baseline_cost")]
    pub baseline_cost: TxCost,
    /// critical transactions cost
    #[serde(default = "default_critical_txs_cost")]
    pub critical_txs_cost: Option<TxCost>,
    /// management transactions cost
    #[serde(default = "default_management_txs_cost")]
    pub management_txs_cost: Option<TxCost>,
    /// Miner transactions cost
    #[serde(default = "default_miner_txs_cost")]
    pub miner_txs_cost: Option<TxCost>,
    /// Cheap or test transation costs
    #[serde(default = "default_cheap_txs_cost")]
    pub cheap_txs_cost: Option<TxCost>,
}

impl TxConfigs {
    /// get the user txs cost preferences for given transaction type
    pub fn get_cost(&self, tx_type: TxType) -> TxCost {
        let ref baseline = self.baseline_cost.clone();
        let cost = match tx_type {
            TxType::Critical => self.critical_txs_cost.as_ref().unwrap_or(baseline),
            TxType::Mgmt => self
                .management_txs_cost
                .as_ref()
                .unwrap_or_else(|| baseline),
            TxType::Miner => self.miner_txs_cost.as_ref().unwrap_or(baseline),
            TxType::Cheap => self.cheap_txs_cost.as_ref().unwrap_or(baseline),
        };
        cost.to_owned()
    }
}

/// Transaction preferences for a given type of transaction
#[derive(Clone, Debug, Deserialize, Serialize)]
// #[serde(deny_unknown_fields)]
pub struct TxCost {
    /// Max gas units to pay per transaction
    pub max_gas_unit_for_tx: u64, // gas UNITS of computation
    /// Max coin price per unit of gas
    pub coin_price_per_unit: u64, // price in micro GAS
    /// Time in seconds to timeout, from now
    pub user_tx_timeout: u64, // seconds,
}

impl TxCost {
    /// create new cost object
    pub fn new(cost: u64) -> Self {
        TxCost {
            max_gas_unit_for_tx: cost, // oracle upgrade transaction is expensive.
            coin_price_per_unit: 1,
            user_tx_timeout: 5_000,
        }
    }
}
impl Default for TxConfigs {
    fn default() -> Self {
        Self {
            baseline_cost: default_baseline_cost(),
            critical_txs_cost: default_critical_txs_cost(),
            management_txs_cost: default_management_txs_cost(),
            miner_txs_cost: default_miner_txs_cost(),
            cheap_txs_cost: default_cheap_txs_cost(),
        }
    }
}

fn default_baseline_cost() -> TxCost {
    TxCost::new(10_000)
}
fn default_critical_txs_cost() -> Option<TxCost> {
    Some(TxCost::new(1_000_000))
}
fn default_management_txs_cost() -> Option<TxCost> {
    Some(TxCost::new(100_000))
}
fn default_miner_txs_cost() -> Option<TxCost> {
    Some(TxCost::new(10_000))
}
fn default_cheap_txs_cost() -> Option<TxCost> {
    Some(TxCost::new(1_000))
}
