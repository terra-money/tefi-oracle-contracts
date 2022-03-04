use crate::contract::{execute, instantiate, query};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi};
use cosmwasm_std::{from_binary, Decimal, MemoryStorage, OwnedDeps, Response, StdResult};
use tefi_oracle::errors::ContractError;
use tefi_oracle::hub::{
    AllSourcesResponse, AssetSymbolMapResponse, ConfigResponse, HubExecuteMsg as ExecuteMsg,
    HubQueryMsg as QueryMsg, InstantiateMsg, PriceResponse, ProxyWhitelistResponse,
    SourcesResponse,
};

use super::mock_querier::{mock_dependencies, WasmMockQuerier};

const OWNER_ADDR: &str = "owner_0001";
const PROXY_ADDR_1: &str = "proxy_0001";
const PROXY_ADDR_2: &str = "proxy_0002";

// helper to successfully init
pub fn init(deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>) -> StdResult<Response> {
    let msg = InstantiateMsg {
        owner: OWNER_ADDR.to_string(),
        base_denom: "uusd".to_string(),
        max_proxies_per_symbol: 10u8,
    };
    let info = mock_info(OWNER_ADDR, &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg)
}

// helper to whitelsit a proxy
pub fn whitelist_proxy(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>,
    proxy_addr: &str,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::WhitelistProxy {
        proxy_addr: proxy_addr.to_string(),
    };
    let info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg)
}

// helper to register source
pub fn register_source(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>,
    symbol: &str,
    proxy_addr: &str,
    priority: Option<u8>,
) -> Result<Response, ContractError> {
    let msg = ExecuteMsg::RegisterSource {
        proxy_addr: proxy_addr.to_string(),
        symbol: symbol.to_string(),
        priority,
    };
    let info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), info, msg)
}

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    // it worked, let's query the state
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: OWNER_ADDR.to_string(),
            base_denom: "uusd".to_string(),
            max_proxies_per_symbol: 10u8,
        }
    );
}

#[test]
fn test_update_owner() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    let msg = ExecuteMsg::UpdateOwner {
        owner: "newowner0000".to_string(),
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    // check query is updated
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: "newowner0000".to_string(),
            base_denom: "uusd".to_string(),
            max_proxies_per_symbol: 10u8,
        }
    );
}

#[test]
fn test_update_max_proxies() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    let msg = ExecuteMsg::UpdateMaxProxies {
        max_proxies_per_symbol: 20u8,
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    // check query is updated
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(
        config,
        ConfigResponse {
            owner: OWNER_ADDR.to_string(),
            base_denom: "uusd".to_string(),
            max_proxies_per_symbol: 20u8, // updated
        }
    );
}

#[test]
fn test_whitelist_proxy() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    let msg = ExecuteMsg::WhitelistProxy {
        proxy_addr: PROXY_ADDR_1.to_string(),
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    // check updated whitelist
    let res = query(deps.as_ref(), mock_env(), QueryMsg::ProxyWhitelist {}).unwrap();
    let res: ProxyWhitelistResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        ProxyWhitelistResponse {
            proxies: vec![PROXY_ADDR_1.to_string()]
        }
    );

    // add another one
    whitelist_proxy(&mut deps, PROXY_ADDR_2).unwrap();

    let res = query(deps.as_ref(), mock_env(), QueryMsg::ProxyWhitelist {}).unwrap();
    let res: ProxyWhitelistResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        ProxyWhitelistResponse {
            proxies: vec![PROXY_ADDR_1.to_string(), PROXY_ADDR_2.to_string()]
        }
    );
}

#[test]
fn test_remove_proxy() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    whitelist_proxy(&mut deps, PROXY_ADDR_1).unwrap();
    whitelist_proxy(&mut deps, PROXY_ADDR_2).unwrap();

    let msg = ExecuteMsg::RemoveProxy {
        proxy_addr: PROXY_ADDR_2.to_string(),
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

    // check updated whitelist
    let res = query(deps.as_ref(), mock_env(), QueryMsg::ProxyWhitelist {}).unwrap();
    let res: ProxyWhitelistResponse = from_binary(&res).unwrap();
    assert_eq!(
        res,
        ProxyWhitelistResponse {
            proxies: vec![
                PROXY_ADDR_1.to_string() // onlyt proxy 1 should remain
            ]
        }
    );

    // attempt to remove one that does not exist
    let msg = ExecuteMsg::RemoveProxy {
        proxy_addr: "xxxx".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap_err();
    assert_eq!(err, ContractError::ProxyNotWhitelisted {});
}

#[test]
fn test_register_source() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    let msg = ExecuteMsg::RegisterSource {
        symbol: "TSLA".to_string(),
        proxy_addr: PROXY_ADDR_1.to_string(),
        priority: Some(2u8),
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // attempt to register non whitelisted proxy
    let owner_info = mock_info(OWNER_ADDR, &[]);
    let err = execute(deps.as_mut(), mock_env(), owner_info.clone(), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::ProxyNotWhitelisted {});

    // whitelist the proxy
    whitelist_proxy(&mut deps, PROXY_ADDR_1).unwrap();
    whitelist_proxy(&mut deps, PROXY_ADDR_2).unwrap();

    // successful attempt
    execute(deps.as_mut(), mock_env(), owner_info.clone(), msg.clone()).unwrap();

    // check query is updated
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AllSources {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let sources: AllSourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        AllSourcesResponse {
            list: vec![SourcesResponse {
                symbol: "TSLA".to_string(),
                proxies: vec![(2u8, PROXY_ADDR_1.to_string())]
            }]
        }
    );

    // if we register same proxy again, graceful return
    execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

    // register one more source for tsla
    let msg = ExecuteMsg::RegisterSource {
        symbol: "TSLA".to_string(),
        proxy_addr: PROXY_ADDR_2.to_string(),
        priority: Some(1u8),
    };
    execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AllSources {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let sources: AllSourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        AllSourcesResponse {
            list: vec![SourcesResponse {
                symbol: "TSLA".to_string(),
                proxies: vec![
                    (1u8, PROXY_ADDR_2.to_string()), // new proxy has higher priority
                    (2u8, PROXY_ADDR_1.to_string())
                ]
            }]
        }
    );

    // try query by symbol
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::SourcesBySymbol {
            symbol: "TSLA".to_string(),
        },
    )
    .unwrap();
    let sources: SourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        SourcesResponse {
            symbol: "TSLA".to_string(),
            proxies: vec![
                (1u8, PROXY_ADDR_2.to_string()), // new proxy has higher priority
                (2u8, PROXY_ADDR_1.to_string())
            ]
        }
    );
}

#[test]
fn test_remove_source() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    whitelist_proxy(&mut deps, PROXY_ADDR_1).unwrap();
    whitelist_proxy(&mut deps, PROXY_ADDR_2).unwrap();

    register_source(&mut deps, "TSLA", PROXY_ADDR_2, None).unwrap();
    register_source(&mut deps, "TSLA", PROXY_ADDR_1, None).unwrap();

    let msg = ExecuteMsg::RemoveSource {
        symbol: "TSLA".to_string(),
        proxy_addr: PROXY_ADDR_1.to_string(),
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::SourcesBySymbol {
            symbol: "TSLA".to_string(),
        },
    )
    .unwrap();
    let sources: SourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        SourcesResponse {
            symbol: "TSLA".to_string(),
            proxies: vec![
                (10u8, PROXY_ADDR_2.to_string()), // only proxy 2 remains
            ]
        }
    );

    // attempt to remove proxy that does not exist
    let msg = ExecuteMsg::RemoveSource {
        symbol: "TSLA".to_string(),
        proxy_addr: "asdfadsf".to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::ProxyNotRegistered {});

    // attempt to remove symbol that does not exist
    let msg = ExecuteMsg::RemoveSource {
        symbol: "TSLAasdfadsf".to_string(),
        proxy_addr: PROXY_ADDR_2.to_string(),
    };
    let err = execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap_err();
    assert_eq!(err, ContractError::SymbolNotRegistered {});
}

#[test]
fn test_update_priority() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    whitelist_proxy(&mut deps, PROXY_ADDR_1).unwrap();
    whitelist_proxy(&mut deps, PROXY_ADDR_2).unwrap();

    register_source(&mut deps, "TSLA", PROXY_ADDR_2, None).unwrap();
    register_source(&mut deps, "TSLA", PROXY_ADDR_1, None).unwrap();

    // check initial state
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::SourcesBySymbol {
            symbol: "TSLA".to_string(),
        },
    )
    .unwrap();
    let sources: SourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        SourcesResponse {
            symbol: "TSLA".to_string(),
            proxies: vec![
                (10u8, PROXY_ADDR_2.to_string()),
                (10u8, PROXY_ADDR_1.to_string())
            ]
        }
    );

    let msg = ExecuteMsg::UpdateSourcePriorityList {
        symbol: "TSLA".to_string(),
        priority_list: vec![(PROXY_ADDR_1.to_string(), 2), (PROXY_ADDR_2.to_string(), 3)],
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // successfull attempt
    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

    // check updated state
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::SourcesBySymbol {
            symbol: "TSLA".to_string(),
        },
    )
    .unwrap();
    let sources: SourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        SourcesResponse {
            symbol: "TSLA".to_string(),
            proxies: vec![
                (2u8, PROXY_ADDR_1.to_string()),
                (3u8, PROXY_ADDR_2.to_string())
            ]
        }
    );

    // try to update not registered symbol
    let msg = ExecuteMsg::UpdateSourcePriorityList {
        symbol: "TSLAssssss".to_string(),
        priority_list: vec![(PROXY_ADDR_1.to_string(), 2), (PROXY_ADDR_2.to_string(), 3)],
    };

    let err = execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap_err();
    assert_eq!(err, ContractError::SymbolNotRegistered {});

    // try to update not registered proxy
    let msg = ExecuteMsg::UpdateSourcePriorityList {
        symbol: "TSLA".to_string(),
        priority_list: vec![
            (PROXY_ADDR_1.to_string(), 2),
            ("notaproxy123123".to_string(), 3),
        ],
    };
    let err = execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap_err();
    assert_eq!(err, ContractError::ProxyNotRegistered {});
}

#[test]
fn test_symbol_map() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    whitelist_proxy(&mut deps, PROXY_ADDR_1).unwrap();
    register_source(&mut deps, "TSLA", PROXY_ADDR_1, None).unwrap();

    let msg = ExecuteMsg::InsertAssetSymbolMap {
        map: vec![
            ("tsla0000".to_string(), "TSLA".to_string()),
            ("aapl0000".to_string(), "AAPL".to_string()),
        ],
    };

    // unauthorized attempt
    let info = mock_info("notowner0000", &[]);
    let err = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // successfull attempt
    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info.clone(), msg).unwrap();

    // check updated state
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AssetSymbolMap {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let map_res: AssetSymbolMapResponse = from_binary(&res).unwrap();
    assert_eq!(
        map_res,
        AssetSymbolMapResponse {
            map: vec![
                ("aapl0000".to_string(), "AAPL".to_string()),
                ("tsla0000".to_string(), "TSLA".to_string()),
            ]
        }
    );

    // try to add a new one, and override existing
    let msg = ExecuteMsg::InsertAssetSymbolMap {
        map: vec![
            ("aapl0000".to_string(), "AAPL2".to_string()), // update existring one
            ("amzon0000".to_string(), "AMZN".to_string()), // new one
        ],
    };

    execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    // check updated state
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AssetSymbolMap {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let map_res: AssetSymbolMapResponse = from_binary(&res).unwrap();
    assert_eq!(
        map_res,
        AssetSymbolMapResponse {
            map: vec![
                ("aapl0000".to_string(), "AAPL2".to_string()), // updated
                ("amzon0000".to_string(), "AMZN".to_string()), // new one
                ("tsla0000".to_string(), "TSLA".to_string()),
            ]
        }
    );

    // test that now we can query by asset token

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Sources {
            asset_token: "tsla0000".to_string(),
        },
    )
    .unwrap();
    let sources: SourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        SourcesResponse {
            symbol: "TSLA".to_string(),
            proxies: vec![(10u8, PROXY_ADDR_1.to_string()),]
        }
    );

    deps.querier
        .with_proxy_price(&[(&"TSLA".to_string(), &Decimal::one())]);

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            asset_token: "tsla0000".to_string(),
            timeframe: None,
        },
    )
    .unwrap();
    let price_res: PriceResponse = from_binary(&res).unwrap();
    assert_eq!(
        price_res,
        PriceResponse {
            rate: Decimal::one(),
            last_updated: 1000u64,
        }
    );

    // try to query asset that is not mapped
    let err = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Price {
            asset_token: "random0000".to_string(),
            timeframe: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::MappingNotFound {});
}

#[test]
fn test_query_pagination() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    whitelist_proxy(&mut deps, PROXY_ADDR_1).unwrap();
    whitelist_proxy(&mut deps, PROXY_ADDR_2).unwrap();

    register_source(&mut deps, "TSLA", PROXY_ADDR_1, None).unwrap();
    register_source(&mut deps, "AAPL", PROXY_ADDR_2, None).unwrap();
    register_source(&mut deps, "AMZN", PROXY_ADDR_1, None).unwrap();

    // without pagination
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AllSources {
            start_after: None,
            limit: None,
        },
    )
    .unwrap();
    let sources: AllSourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        AllSourcesResponse {
            list: vec![
                SourcesResponse {
                    symbol: "AAPL".to_string(),
                    proxies: vec![(10u8, PROXY_ADDR_2.to_string())]
                },
                SourcesResponse {
                    symbol: "AMZN".to_string(),
                    proxies: vec![(10u8, PROXY_ADDR_1.to_string())]
                },
                SourcesResponse {
                    symbol: "TSLA".to_string(),
                    proxies: vec![(10u8, PROXY_ADDR_1.to_string())]
                }
            ]
        }
    );

    // with pagination
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AllSources {
            start_after: None,
            limit: Some(1u32),
        },
    )
    .unwrap();
    let sources: AllSourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        AllSourcesResponse {
            list: vec![SourcesResponse {
                symbol: "AAPL".to_string(),
                proxies: vec![(10u8, PROXY_ADDR_2.to_string())]
            },]
        }
    );
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AllSources {
            start_after: Some("AAPL".to_string()),
            limit: Some(1u32),
        },
    )
    .unwrap();
    let sources: AllSourcesResponse = from_binary(&res).unwrap();
    assert_eq!(
        sources,
        AllSourcesResponse {
            list: vec![SourcesResponse {
                symbol: "AMZN".to_string(),
                proxies: vec![(10u8, PROXY_ADDR_1.to_string())]
            },]
        }
    );

    // regiter mapping
    let msg = ExecuteMsg::InsertAssetSymbolMap {
        map: vec![
            ("tsla0000".to_string(), "TSLA".to_string()),
            ("aapl0000".to_string(), "AAPL".to_string()),
            ("amzn0000".to_string(), "AMZN".to_string()),
        ],
    };

    // successfull attempt
    let owner_info = mock_info(OWNER_ADDR, &[]);
    execute(deps.as_mut(), mock_env(), owner_info, msg).unwrap();

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AssetSymbolMap {
            start_after: None,
            limit: Some(1u32),
        },
    )
    .unwrap();
    let map_res: AssetSymbolMapResponse = from_binary(&res).unwrap();
    assert_eq!(
        map_res,
        AssetSymbolMapResponse {
            map: vec![("aapl0000".to_string(), "AAPL".to_string()),]
        }
    );
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AssetSymbolMap {
            start_after: Some("aapl0000".to_string()),
            limit: Some(1u32),
        },
    )
    .unwrap();
    let map_res: AssetSymbolMapResponse = from_binary(&res).unwrap();
    assert_eq!(
        map_res,
        AssetSymbolMapResponse {
            map: vec![("amzn0000".to_string(), "AMZN".to_string()),]
        }
    );
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::AssetSymbolMap {
            start_after: Some("aapl0000".to_string()),
            limit: None,
        },
    )
    .unwrap();
    let map_res: AssetSymbolMapResponse = from_binary(&res).unwrap();
    assert_eq!(
        map_res,
        AssetSymbolMapResponse {
            map: vec![
                ("amzn0000".to_string(), "AMZN".to_string()),
                ("tsla0000".to_string(), "TSLA".to_string()),
            ]
        }
    );
}
