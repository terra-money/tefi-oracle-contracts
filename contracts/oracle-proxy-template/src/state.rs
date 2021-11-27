use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::ConfigResponse;

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub source_addr: Addr,
}

impl Config {
    pub fn as_res(&self) -> ConfigResponse {
        ConfigResponse {
            source_addr: self.source_addr.to_string(),
        }
    }
}
