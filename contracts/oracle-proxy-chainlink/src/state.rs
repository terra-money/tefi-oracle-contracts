use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::msg::ConfigResponse;

pub const CONFIG: Item<Config> = Item::new("config");
pub const SOURCES: Map<&Addr, Addr> = Map::new("sources");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub owner: Addr,
}

impl Config {
    pub fn as_res(&self) -> ConfigResponse {
        ConfigResponse {
            owner: self.owner.to_string(),
        }
    }

    /// @dev Checks if the provided addr is owner
    /// @param addr : address to check
    pub fn is_owner(&self, addr: &Addr) -> bool {
        self.owner.eq(addr)
    }
}
