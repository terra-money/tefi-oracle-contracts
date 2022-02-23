use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::ConfigResponse;

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub owner: Addr,
    pub source_addr: Addr,
    pub quote_symbol: String,
}

impl Config {
    pub fn as_res(&self) -> ConfigResponse {
        ConfigResponse {
            owner: self.owner.to_string(),
            source_addr: self.source_addr.to_string(),
            quote_symbol: self.quote_symbol.to_string(),
        }
    }

    /// @dev Checks if the provided addr is owner
    /// @param addr : address to check
    pub fn is_owner(&self, addr: &Addr) -> bool {
        self.owner.eq(addr)
    }
}
