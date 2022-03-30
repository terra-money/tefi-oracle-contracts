use cosmwasm_std::testing::{MockApi, MockQuerier, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, to_binary, Coin, ContractResult, Decimal, Empty, OwnedDeps, Querier,
    QuerierResult, QueryRequest, SystemError, SystemResult, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tefi_oracle::proxy::ProxyPriceResponse;

/// mock_dependencies is a drop-in replacement for cosmwasm_std::testing::mock_dependencies
pub fn mock_dependencies(
    contract_balance: &[Coin],
) -> OwnedDeps<MockStorage, MockApi, WasmMockQuerier> {
    let custom_querier: WasmMockQuerier =
        WasmMockQuerier::new(MockQuerier::new(&[(MOCK_CONTRACT_ADDR, contract_balance)]));

    OwnedDeps {
        api: MockApi::default(),
        storage: MockStorage::default(),
        querier: custom_querier,
    }
}

pub struct WasmMockQuerier {
    base: MockQuerier<Empty>,
    oracle_proxy_price_querier: OracleProxyPriceQuerier,
}

#[derive(Clone, Default)]
pub struct OracleProxyPriceQuerier {
    oracle_price: HashMap<String, Decimal>,
}

impl OracleProxyPriceQuerier {
    pub fn new(oracle_price: &[(&String, &Decimal)]) -> Self {
        OracleProxyPriceQuerier {
            oracle_price: oracle_price_to_map(oracle_price),
        }
    }
}

pub(crate) fn oracle_price_to_map(
    oracle_price: &[(&String, &Decimal)],
) -> HashMap<String, Decimal> {
    let mut oracle_price_map: HashMap<String, Decimal> = HashMap::new();
    for (symbol, oracle_price) in oracle_price.iter() {
        oracle_price_map.insert((*symbol).clone(), **oracle_price);
    }

    oracle_price_map
}

impl Querier for WasmMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        self.handle_query(&request)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MockQueryMsg {
    Base(ProxyMockQueryMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMockQueryMsg {
    Price { symbol: String },
}

impl WasmMockQuerier {
    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: _,
                msg,
            }) => match from_binary(msg).unwrap() {
                MockQueryMsg::Base(ProxyMockQueryMsg::Price { symbol }) => {
                    match self.oracle_proxy_price_querier.oracle_price.get(&symbol) {
                        Some(price) => {
                            let res = ProxyPriceResponse {
                                rate: *price,
                                last_updated: 1000u64,
                            };

                            SystemResult::Ok(ContractResult::from(to_binary(&res)))
                        }
                        None => SystemResult::Err(SystemError::InvalidRequest {
                            error: "No oracle price exists".to_string(),
                            request: msg.as_slice().into(),
                        }),
                    }
                }
            },
            _ => self.base.handle_query(request),
        }
    }
}

impl WasmMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        WasmMockQuerier {
            base,
            oracle_proxy_price_querier: OracleProxyPriceQuerier::default(),
        }
    }

    // configure the oracle price mock querier
    pub fn with_proxy_price(&mut self, proxy_prices: &[(&String, &Decimal)]) {
        self.oracle_proxy_price_querier = OracleProxyPriceQuerier::new(proxy_prices);
    }
}
