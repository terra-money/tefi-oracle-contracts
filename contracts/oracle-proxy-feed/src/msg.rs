use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tefi_oracle::proxy::ProxyQueryMsg;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: String,
    },
    /// Used to register new asset or to update feeder
    RegisterFeed {
        symbol: String,
        feeder: String,
    },
    FeedPrices {
        prices: Vec<(String, Decimal)>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Base(ProxyQueryMsg),
    Config {},
    Feeder { symbol: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct FeederResponse {
    pub symbol: String,
    pub feeder: String,
}
