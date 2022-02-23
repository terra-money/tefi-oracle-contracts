use cosmwasm_bignumber::Decimal256;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Decimal;

pub const DEFAULT_PRIORITY: u8 = 10;
pub const MAX_WHITELISTED_PROXIES: u8 = 30;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub base_denom: String,
    pub max_proxies_per_asset: u8,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HubExecuteMsg {
    /// Owner operation to update the owner parameter
    UpdateOwner { owner: String },
    /// Owner operation to update the max_proxies_per_asset parameter
    UpdateMaxProxies { max_proxies_per_asset: u8 },
    ///
    RegisterSource {
        asset_token: String,
        proxy_addr: String,
        priority: Option<u8>,
    },
    /// Updates the priority paramter of an existing proxy
    UpdateSourcePriority {
        asset_token: String,
        proxy_addr: String,
        priority: u8,
    },
    /// Remves an already whitelisted proxy
    RemoveSource {
        asset_token: String,
        proxy_addr: String,
    },
    ///
    WhitelistProxy { proxy_addr: String },
    ///
    RemoveProxy { proxy_addr: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HubQueryMsg {
    /// Queries contract configuration
    Config {},
    ///
    ProxyWhitelist {},
    /// Queries the information of all registered proxies for the provided asset_token
    Sources { asset_token: String },
    /// Queries the highes priority available price within the timeframe
    /// If timeframe is not provided, it will ignore the price age
    Price {
        asset_token: String,
        timeframe: Option<u64>,
    },
    /// Queries all registered proxy prices for the provied asset_token
    PriceList { asset_token: String },
    /// Anchor legacy query interface for oracle prices
    LegacyPrice { base: String, quote: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub base_denom: String,
    pub max_proxies_per_asset: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceResponse {
    pub rate: Decimal,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum PriceQueryResult {
    Success(PriceResponse),
    Fail,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PriceListResponse {
    pub price_list: Vec<(u8, PriceQueryResult)>, // (priority, result)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SourcesResponse {
    pub asset_token: String,
    pub proxies: Vec<(u8, String)>,
}

impl From<crate::proxy::ProxyPriceResponse> for PriceResponse {
    fn from(proxy_res: crate::proxy::ProxyPriceResponse) -> Self {
        PriceResponse {
            rate: proxy_res.rate,
            last_updated: proxy_res.last_updated,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProxyWhitelistResponse {
    pub proxies: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyPriceResponse {
    pub rate: Decimal256,
    pub last_updated_base: u64,
    pub last_updated_quote: u64,
}
