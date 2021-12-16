use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tefi_oracle::proxy::ProxyQueryMsg;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub source_addr: String,
    pub quote_symbol: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: String,
    },
    /// Registers a new symbol mapping for an asset token, or updates the existing
    SetSymbolMapping {
        asset_token: String,
        symbol: String,
    },
    /// Removes an existing mapping
    RemoveSymbolMapping {
        asset_token: String,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Base(ProxyQueryMsg),
    Config {},
    /// If no asset_token is provided, returns all mappings
    SymbolMap {
        asset_token: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub source_addr: String,
    pub quote_symbol: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SymbolMapResponse {
    pub map: Vec<(String, String)>,
}

/// Band Protocol interface

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BandResponse {
    pub rate: Uint128,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BandMsg {
    GetReferenceData {
        base_symbol: String,
        quote_symbol: String,
    },
}
