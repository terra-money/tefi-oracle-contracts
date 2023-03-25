use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Decimal, Uint64};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tefi_oracle::proxy::ProxyQueryMsg;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct InstantiateMsg {
    pub source_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub source_addr: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Base(ProxyQueryMsg),
    Config {},
}

#[cw_serde]
pub enum CandleQuery {
    Price { denom: String },
}

#[cw_serde]
pub struct CandleResponse {
    pub rate: Decimal,
    pub last_updated_timestamp: Uint64,
}

