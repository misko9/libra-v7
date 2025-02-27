use move_core_types::{
    ident_str,
    language_storage::StructTag,
    move_resource::{MoveResource, MoveStructType},
};
use move_core_types::identifier::IdentStr;
use move_core_types::language_storage::TypeTag;
use serde::{Deserialize, Serialize};

use diem_types::{account_address::AccountAddress, event::EventHandle};
use once_cell::sync::Lazy;

use crate::ONCHAIN_DECIMAL_PRECISION;

/// The balance resource held under an account.
#[derive(Debug, Serialize, Deserialize)]
// #[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct GasCoinStoreResource {
    coin: u64,
    frozen: bool,
    deposit_events: EventHandle,
    withdraw_events: EventHandle,
}

impl GasCoinStoreResource {
    pub fn new(
        coin: u64,
        frozen: bool,
        deposit_events: EventHandle,
        withdraw_events: EventHandle,
    ) -> Self {
        Self {
            coin,
            frozen,
            deposit_events,
            withdraw_events,
        }
    }

    pub fn coin(&self) -> u64 {
        self.coin
    }

    pub fn frozen(&self) -> bool {
        self.frozen
    }

    pub fn deposit_events(&self) -> &EventHandle {
        &self.deposit_events
    }

    pub fn withdraw_events(&self) -> &EventHandle {
        &self.withdraw_events
    }
}

impl MoveStructType for GasCoinStoreResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinStore");

    fn type_params() -> Vec<TypeTag> {
        vec![GAS_COIN_TYPE.clone()]
    }
}

impl MoveResource for GasCoinStoreResource {}

// TODO: This might break reading from API maybe it must be diem_api_types::U64;

#[derive(Debug, Serialize, Deserialize)]
pub struct GasCoin {
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SlowWalletBalance {
    pub unlocked: u64,
    pub total: u64,
}

impl MoveStructType for SlowWalletBalance {
    const MODULE_NAME: &'static IdentStr = ident_str!("slow_wallet");
    const STRUCT_NAME: &'static IdentStr = ident_str!("SlowWallet");
}

impl MoveResource for SlowWalletBalance {}

impl SlowWalletBalance {
    pub fn from_value(value: Vec<serde_json::Value>) -> anyhow::Result<Self> {
        if value.len() != 2 {
            return Err(anyhow::anyhow!("invalid value length"));
        }
        let unlocked = serde_json::from_value::<String>(value[0].clone())?.parse::<u64>()?;
        let total = serde_json::from_value::<String>(value[1].clone())?.parse::<u64>()?;

        Ok(Self { unlocked, total })
    }

    // scale it to include decimals
    pub fn scaled(&self) -> LibraBalanceDisplay {
        LibraBalanceDisplay {
            unlocked: cast_coin_to_decimal(self.unlocked),
            total: cast_coin_to_decimal(self.total),
        }
    }
}

/// This is the same shape as Slow Wallet balance, except that it is scaled.
/// The slow wallet struct contains the coin value as it exists in the database which is without decimals. The decimal precision for GasCoin is 6. So we need to scale it for human consumption.
#[derive(Debug, Serialize, Deserialize)]

pub struct LibraBalanceDisplay {
    pub unlocked: f64,
    pub total: f64,
}
