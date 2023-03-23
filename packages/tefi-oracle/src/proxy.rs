use cosmwasm_std::{Decimal, Decimal256};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProxyQueryMsg {
    Price { symbol: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct ProxyPriceResponse {
    pub rate: Decimal,     // rate denominated in base_denom
    pub last_updated: u64, // timestamp in seconds
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProxyBaseQuery {
    Base(ProxyQueryMsg),
}
