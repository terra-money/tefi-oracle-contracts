use crate::contract::{execute, instantiate, query};
use crate::state::is_existing_pair;
use astroport::asset::{Asset, AssetInfo};
use astroport::pair::{
    Cw20HookMsg as PairCw20HookMsg, ExecuteMsg as PairExecuteMsg, ReverseSimulationResponse,
    SimulationResponse,
};
use cosmwasm_std::testing::{mock_env, mock_info, MockApi};
use cosmwasm_std::{
    from_binary, to_binary, Addr, Attribute, BankMsg, Coin, CosmosMsg, Decimal, MemoryStorage,
    OwnedDeps, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use std::str::FromStr;

use super::mock_querier::{mock_dependencies, WasmMockQuerier};
use prism_protocol::limit_order::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, LastOrderIdResponse, OrderBy, OrderResponse,
    OrdersResponse, QueryMsg,
};

const OWNER_ADDR: &str = "owner_0001";
const USER1_ADDR: &str = "user_0001";
const USER2_ADDR: &str = "user_0002";
const EXECUTOR_ADDR: &str = "executor_0001";
const PRISM_ADDR: &str = "prism_0001";
const PLUNA_ADDR: &str = "pluna_0001";
const YLUNA_ADDR: &str = "yluna_0001";
const PRISM_UST_PAIR_ADDR: &str = "prism_ust_pair_0001";
const FEE_COLLECTOR_ADDR: &str = "fee_coll_0001";
const EXCESS_COLLECTOR_ADDR: &str = "excess_coll_0001";

// helper to successfully init
pub fn init(deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>) -> StdResult<Response> {
    let msg = InstantiateMsg {
        base_denom: "uusd".to_string(),
        prism_token: PRISM_ADDR.to_string(),
        fee_collector_addr: FEE_COLLECTOR_ADDR.to_string(),
        prism_ust_pair: PRISM_UST_PAIR_ADDR.to_string(),
        order_fee: Decimal::from_str("0.05").unwrap(),
        min_fee_value: Uint128::from(100u128),
        executor_fee_portion: Decimal::from_str("0.25").unwrap(),
        excess_collector_addr: EXCESS_COLLECTOR_ADDR.to_string(),
    };
    let info = mock_info(OWNER_ADDR, &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg)
}

// helper to build a pair addr from underlying assets
pub fn get_pair_addr(asset_infos: &[AssetInfo; 2]) -> String {
    format!("{}_{}", asset_infos[0], asset_infos[1])
}

// helper to add a trading pair
pub fn add_trading_pair(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>,
    asset_infos: &[AssetInfo; 2],
    owner_addr: &str,
) -> StdResult<Response> {
    let msg = ExecuteMsg::AddPair {
        asset_infos: asset_infos.clone(),
        pair_addr: get_pair_addr(&asset_infos),
    };
    let info = mock_info(owner_addr, &[]);
    execute(deps.as_mut(), mock_env(), info.clone(), msg.clone())
}

// helper to submit an order
pub fn submit_order(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, WasmMockQuerier>,
    assets: &[Asset; 2],
    user_addr: &str,
    funds: &Vec<Coin>,
) -> StdResult<Response> {
    let msg = ExecuteMsg::SubmitOrder {
        offer_asset: assets[0].clone(),
        ask_asset: assets[1].clone(),
    };
    let info = mock_info(user_addr, &funds);
    execute(deps.as_mut(), mock_env(), info.clone(), msg)
}

// helper to verify the messages returned from a limit order being executed
pub fn verify_execute_response(
    res: &Response,
    pair_addr: &String,
    offer_asset: &Asset,
    ask_asset: &Asset,
    return_asset: &Asset,
    executor_fee_amount: &Uint128,
    protocol_fee_amount: &Uint128,
    order_id: u32,
) {
    let excess_amount = return_asset.amount.checked_sub(ask_asset.amount).unwrap();
    let mut num_msgs = 3;
    if excess_amount > Uint128::zero() {
        num_msgs = num_msgs + 1;
    }
    if *protocol_fee_amount > Uint128::zero() {
        num_msgs = num_msgs + 1;
    }
    assert_eq!(res.messages.len(), num_msgs);

    let mut idx = 0;
    match offer_asset.clone().info {
        AssetInfo::Token { contract_addr } => {
            assert_eq!(
                res.messages[idx],
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Send {
                        contract: pair_addr.to_string(),
                        amount: offer_asset.amount,
                        msg: to_binary(&PairCw20HookMsg::Swap {
                            to: None,
                            belief_price: None,
                            max_spread: None,
                        })
                        .unwrap(),
                    })
                    .unwrap(),
                }))
            );
        }
        AssetInfo::NativeToken { denom } => {
            assert_eq!(
                res.messages[idx],
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: pair_addr.to_string(),
                    funds: vec![Coin {
                        denom,
                        amount: offer_asset.amount
                    }],
                    msg: to_binary(&PairExecuteMsg::Swap {
                        offer_asset: offer_asset.clone(),
                        to: None,
                        belief_price: None,
                        max_spread: None,
                    })
                    .unwrap(),
                }))
            );
        }
    }
    idx = idx + 1;

    // send requested ask to bidder
    assert_eq!(
        res.messages[idx],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: ask_asset.info.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: USER1_ADDR.to_string().clone(),
                amount: ask_asset.amount,
            })
            .unwrap(),
        }))
    );
    idx = idx + 1;

    // send excess ask to excess collector
    if excess_amount > Uint128::zero() {
        assert_eq!(
            res.messages[idx],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ask_asset.info.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: EXCESS_COLLECTOR_ADDR.to_string().clone(),
                    amount: excess_amount,
                })
                .unwrap(),
            }))
        );
        idx = idx + 1;
    }

    // send prism executor fee to executor
    assert_eq!(
        res.messages[idx],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: PRISM_ADDR.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: EXECUTOR_ADDR.to_string().clone(),
                amount: *executor_fee_amount,
            })
            .unwrap(),
        }))
    );
    idx = idx + 1;

    // send prism protocol fee to fee collector
    if *protocol_fee_amount > Uint128::zero() {
        assert_eq!(
            res.messages[idx],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: PRISM_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: FEE_COLLECTOR_ADDR.to_string().clone(),
                    amount: *protocol_fee_amount,
                })
                .unwrap(),
            }))
        );
    }

    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "execute_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: order_id.to_string()
            },
            Attribute {
                key: "executor_fee_amount".to_string(),
                value: executor_fee_amount.to_string()
            },
            Attribute {
                key: "protocol_fee_amount".to_string(),
                value: protocol_fee_amount.to_string()
            },
            Attribute {
                key: "excess_amount".to_string(),
                value: excess_amount.to_string()
            }
        ]
    );
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
            prism_token: PRISM_ADDR.to_string(),
            fee_collector_addr: FEE_COLLECTOR_ADDR.to_string(),
            prism_ust_pair: PRISM_UST_PAIR_ADDR.to_string(),
            order_fee: Decimal::from_str("0.05").unwrap(),
            min_fee_value: Uint128::from(100u128),
            executor_fee_portion: Decimal::from_str("0.25").unwrap(),
            excess_collector_addr: EXCESS_COLLECTOR_ADDR.to_string(),
        }
    );
}

#[test]
fn test_add_pairs() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(YLUNA_ADDR),
        },
    ];

    // unauthorized
    let res = add_trading_pair(&mut deps, &asset_infos, "not_the_owner_addr").unwrap_err();
    assert_eq!(res, StdError::generic_err("unauthorized"));

    // successful add pair
    let res = add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![Attribute {
            key: "action".to_string(),
            value: "add_pair".to_string()
        }]
    );

    // verify pair exists in storage
    let res = is_existing_pair(&deps.storage, &asset_infos);
    assert!(res);

    // verify pair exists when queried in reverse order
    let asset_infos_rev = [asset_infos[1].clone(), asset_infos[0].clone()];
    let res = is_existing_pair(&deps.storage, &asset_infos_rev);
    assert!(res);

    // error - pair already exists
    let res = add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap_err();
    assert_eq!(res, StdError::generic_err("pair already exists"));

    // error - neither asset is PRISM
    let asset_infos = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PLUNA_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(YLUNA_ADDR),
        },
    ];
    let res = add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("one of the assets has to be PRISM token")
    );
}

#[test]
fn test_submit_order() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env();
    init(&mut deps).unwrap();

    // successful submit, offering a token (prism)
    let asset_infos = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(YLUNA_ADDR),
        },
    ];
    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];

    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: PRISM_ADDR.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                owner: USER1_ADDR.to_string(),
                recipient: env.contract.address.to_string(),
                amount: Uint128::from(1_000u128),
            })
            .unwrap(),
        }))
    );
    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "submit_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: "1".to_string()
            },
            Attribute {
                key: "bidder_addr".to_string(),
                value: USER1_ADDR.to_string()
            },
            Attribute {
                key: "offer_asset".to_string(),
                value: "1000prism_0001".to_string()
            },
            Attribute {
                key: "ask_asset".to_string(),
                value: "1000yluna_0001".to_string()
            }
        ]
    );

    // successful submit, offering a native token (uluna)
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
    ];
    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();
    let funds = vec![Coin::new(1000, "uluna")];
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "submit_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: "2".to_string()
            },
            Attribute {
                key: "bidder_addr".to_string(),
                value: USER1_ADDR.to_string()
            },
            Attribute {
                key: "offer_asset".to_string(),
                value: "1000uluna".to_string()
            },
            Attribute {
                key: "ask_asset".to_string(),
                value: "1000prism_0001".to_string()
            }
        ]
    );

    // failed submit, native tokens not sent
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        )
    );

    // failed submit, wrong native qty sent
    let funds = vec![Coin::new(999, "uluna")];
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        )
    );

    // failed submit, wrong native denom sent
    let funds = vec![Coin::new(1000, "uusd")];
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err(
            "Native token balance mismatch between the argument and the transferred"
        )
    );

    // failed submit, unsupported pair
    let asset_infos = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PLUNA_ADDR),
        },
    ];
    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap_err();
    assert_eq!(
        res,
        StdError::generic_err("the 2 assets provided are not supported")
    );
}

#[test]
fn test_submit_order_with_inter_pair() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    // register 2 pairs
    //      1. ust -> prism
    //      2. prism -> yluna
    // then submit an order for ust -> yluna
    let asset_infos_1 = [
        AssetInfo::NativeToken {
            denom: "uusd".to_string(),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
    ];
    let pair_addr_1 = get_pair_addr(&asset_infos_1);
    add_trading_pair(&mut deps, &asset_infos_1, OWNER_ADDR).unwrap();

    let asset_infos_2 = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(YLUNA_ADDR),
        },
    ];
    let pair_addr_2 = get_pair_addr(&asset_infos_2);
    add_trading_pair(&mut deps, &asset_infos_2, OWNER_ADDR).unwrap();

    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos_1[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos_2[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];
    let funds = vec![Coin::new(1_000u128, "uusd")];
    let res = submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();
    assert!(res.messages.is_empty());
    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "submit_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: "1".to_string()
            },
            Attribute {
                key: "bidder_addr".to_string(),
                value: USER1_ADDR.to_string()
            },
            Attribute {
                key: "offer_asset".to_string(),
                value: "1000uusd".to_string()
            },
            Attribute {
                key: "ask_asset".to_string(),
                value: "1000yluna_0001".to_string()
            }
        ]
    );

    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Order { order_id: 1u64 },
    )
    .unwrap();
    let order_response = from_binary::<OrderResponse>(&res).unwrap();
    let expected_order_response = OrderResponse {
        order_id: 1u64,
        bidder_addr: USER1_ADDR.to_string(),
        pair_addr: pair_addr_2,
        offer_asset: assets[0].clone(),
        ask_asset: assets[1].clone(),
        inter_pair_addr: Some(pair_addr_1),
    };
    assert_eq!(order_response, expected_order_response);
}

#[test]
fn test_cancel_order() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    // successful submit, offering a token (prism)
    let asset_infos = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(YLUNA_ADDR),
        },
    ];
    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];

    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // submit 3 orders
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful query order id 2
    query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Order { order_id: 2u64 },
    )
    .unwrap();

    // cancel order id 2
    let msg = ExecuteMsg::CancelOrder { order_id: 2 };
    let info = mock_info(USER1_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: PRISM_ADDR.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: USER1_ADDR.to_string(),
                amount: Uint128::from(1_000u128),
            })
            .unwrap(),
        }))
    );
    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "cancel_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: "2".to_string()
            },
            Attribute {
                key: "refunded_asset".to_string(),
                value: "1000prism_0001".to_string()
            },
        ]
    );

    // failed query order id 2
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Order { order_id: 2u64 },
    )
    .unwrap_err();
    assert!(matches!(res, StdError::NotFound { .. }));

    // failed cancel order id 2 again, doesn't exist anymore
    let msg = ExecuteMsg::CancelOrder { order_id: 2 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert!(matches!(res, StdError::NotFound { .. }));

    // unauthorized cancel order id 3
    let msg = ExecuteMsg::CancelOrder { order_id: 3 };
    let info = mock_info(USER2_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("unauthorized"));

    // submit using native token as offer asset
    // successful submit, offering a native token (uluna)
    let asset_infos = [
        AssetInfo::NativeToken {
            denom: "uluna".to_string(),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
    ];
    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();
    let funds = vec![Coin::new(1000, "uluna")];
    submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();

    // cancel order id 4
    let msg = ExecuteMsg::CancelOrder { order_id: 4 };
    let info = mock_info(USER1_ADDR, &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(BankMsg::Send {
            to_address: USER1_ADDR.to_string(),
            amount: funds,
        })
    );
    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "cancel_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: "4".to_string()
            },
            Attribute {
                key: "refunded_asset".to_string(),
                value: "1000uluna".to_string()
            },
        ]
    );

    // query remaining orders, expected oids: 1,3,5,6
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Orders {
            bidder_addr: Some(USER1_ADDR.to_string()),
            start_after: None,
            limit: None,
            order_by: Some(OrderBy::Asc),
        },
    )
    .unwrap();
    let res = from_binary::<OrdersResponse>(&res).unwrap();
    let order_ids: Vec<u64> = res.orders.iter().map(|x| x.order_id).collect();
    assert_eq!(order_ids, vec![1, 3, 5, 6]);
}

#[test]
fn test_query_orders() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();

    let asset_infos = [
        AssetInfo::Token {
            contract_addr: Addr::unchecked(PRISM_ADDR),
        },
        AssetInfo::Token {
            contract_addr: Addr::unchecked(YLUNA_ADDR),
        },
    ];
    let assets: [Asset; 2] = [
        Asset {
            info: asset_infos[0].clone(),
            amount: Uint128::from(1_000u128),
        },
        Asset {
            info: asset_infos[1].clone(),
            amount: Uint128::from(1_000u128),
        },
    ];

    let pair_addr = get_pair_addr(&asset_infos);
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // query order we just added, order_id=1
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Order { order_id: 1u64 },
    )
    .unwrap();
    let order_response = from_binary::<OrderResponse>(&res).unwrap();
    let expected_order_response = OrderResponse {
        order_id: 1u64,
        bidder_addr: USER1_ADDR.to_string(),
        pair_addr: pair_addr,
        offer_asset: assets[0].clone(),
        ask_asset: assets[1].clone(),
        inter_pair_addr: None,
    };
    assert_eq!(order_response, expected_order_response);

    // query non-existent order
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Order { order_id: 2u64 },
    )
    .unwrap_err();
    assert!(matches!(res, StdError::NotFound { .. }));

    // query orders for USER1_ADDR
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Orders {
            bidder_addr: Some(USER1_ADDR.to_string()),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let orders_response = from_binary::<OrdersResponse>(&res).unwrap();
    let expected_orders_response = OrdersResponse {
        orders: vec![expected_order_response],
    };
    assert_eq!(orders_response, expected_orders_response);

    // query orders for random user, returns empty vector
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Orders {
            bidder_addr: Some("random_addr".to_string()),
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let orders_response = from_binary::<OrdersResponse>(&res).unwrap();
    let expected_orders_response = OrdersResponse { orders: vec![] };
    assert_eq!(orders_response, expected_orders_response);

    // submit 4 more orders for user 1
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // submit 2 orders for user 2
    submit_order(&mut deps, &assets, USER2_ADDR, &vec![]).unwrap();
    submit_order(&mut deps, &assets, USER2_ADDR, &vec![]).unwrap();

    // read all orders, should be 7 orders now
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Orders {
            bidder_addr: None,
            start_after: None,
            limit: None,
            order_by: None,
        },
    )
    .unwrap();
    let res = from_binary::<OrdersResponse>(&res).unwrap();
    assert_eq!(res.orders.len(), 7);

    // query last order id
    let res = query(deps.as_ref(), mock_env(), QueryMsg::LastOrderId {}).unwrap();
    let response = from_binary::<LastOrderIdResponse>(&res).unwrap();
    let expected_response = LastOrderIdResponse { last_order_id: 7 };
    assert_eq!(response, expected_response);

    // test pagination, read all orders for USER1_ADDR, two at a time, ascending
    let mut start_after: Option<u64> = None;
    let limit = Some(2u32);
    loop {
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Orders {
                bidder_addr: Some(USER1_ADDR.to_string()),
                start_after,
                limit,
                order_by: Some(OrderBy::Asc),
            },
        )
        .unwrap();
        let res = from_binary::<OrdersResponse>(&res).unwrap();
        if (res.orders.len() as u32) < limit.unwrap() {
            assert_eq!(res.orders.len(), 1);
            assert_eq!(res.orders[0].order_id, 5); // last oid for user1 is 5
            break;
        }
        start_after = Some(res.orders.last().unwrap().order_id);
    }
    assert_eq!(start_after.unwrap(), 4);

    // test pagination, read all orders for USER1_ADDR, two at a time, descending
    let mut start_after: Option<u64> = None;
    let limit = Some(2u32);
    loop {
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Orders {
                bidder_addr: Some(USER1_ADDR.to_string()),
                start_after,
                limit,
                order_by: Some(OrderBy::Desc),
            },
        )
        .unwrap();
        let res = from_binary::<OrdersResponse>(&res).unwrap();
        if (res.orders.len() as u32) < limit.unwrap() {
            assert_eq!(res.orders.len(), 1);
            assert_eq!(res.orders[0].order_id, 1);
            break;
        }
        start_after = Some(res.orders.last().unwrap().order_id);
    }
    assert_eq!(start_after.unwrap(), 2);

    // test pagination, read all orders for all users, two at a time, ascending
    let mut start_after: Option<u64> = None;
    let limit = Some(2u32);
    loop {
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Orders {
                bidder_addr: None,
                start_after,
                limit,
                order_by: Some(OrderBy::Asc),
            },
        )
        .unwrap();
        let res = from_binary::<OrdersResponse>(&res).unwrap();
        if (res.orders.len() as u32) < limit.unwrap() {
            assert_eq!(res.orders.len(), 1);
            assert_eq!(res.orders[0].order_id, 7); // last oid for user2 is 7
            break;
        }
        start_after = Some(res.orders.last().unwrap().order_id);
    }
    assert_eq!(start_after.unwrap(), 6);

    // test pagination, read all orders for all users, two at a time, descending
    let mut start_after: Option<u64> = None;
    let limit = Some(2u32);
    loop {
        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Orders {
                bidder_addr: None,
                start_after,
                limit,
                order_by: Some(OrderBy::Desc),
            },
        )
        .unwrap();
        let res = from_binary::<OrdersResponse>(&res).unwrap();
        if (res.orders.len() as u32) < limit.unwrap() {
            assert_eq!(res.orders.len(), 1);
            assert_eq!(res.orders[0].order_id, 1);
            break;
        }
        start_after = Some(res.orders.last().unwrap().order_id);
    }
    assert_eq!(start_after.unwrap(), 2);
}

/*
test_execute_yluna_prism
    - offer 1000 yluna, asking for 1000 prism
    - 1200 prism received during astroport swap
    - total prism fee = 1200 * .05 = 60
    - astroport converts 60 prism to min_fee_value uusd, all good
    - returned asset after fees = 1200 - 60 = 1140
    - executor_fee = 60 * .25 = 15
    - protocol_fee = 60 - 15 = 45
*/
#[test]
pub fn test_execute_yluna_prism() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };

    let astro_swap_return = Uint128::from(1200u128);
    let astro_fee_uusd = config.min_fee_value;

    let prism_fee_amount = astro_swap_return * config.order_fee;
    let return_asset_after_fees = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return - prism_fee_amount,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // configure astroport to return 1200 for the offered luna
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // configure astroport to return sufficient prism->uusd conversion to meet fee min
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ask_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset,
        &ask_asset,
        &return_asset_after_fees,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );

    // verify order was removed
    let res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::Order { order_id: 1u64 },
    )
    .unwrap_err();
    assert!(matches!(res, StdError::NotFound { .. }));
}

/*
test_execute_yluna_prism_reverse_sim
    - offer 1000 yluna, asking for 1000 prism
    - 1200 prism received during astroport swap
    - total prism fee = 1200 * .05 = 60
    - astroport converts 60 prism to (min_fee_value-1) uusd, no good
    - reverse sim results in 80 prism needed to meet fee rquirement
    - returned asset after fees = 1200 - 80 = 1120
    - executor_fee = 80 * .25 = 20
    - protocol_fee = 80 - 20 = 60
*/
#[test]
pub fn test_execute_yluna_prism_reverse_sim() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ust_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    let astro_swap_return = Uint128::from(1200u128);
    let astro_fee_uusd = config.min_fee_value - Uint128::from(1u128);
    let astro_prism_required_for_fee = Uint128::from(80u128);

    let prism_fee_amount = astro_prism_required_for_fee;
    let return_asset = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return - prism_fee_amount,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // return 1200 for the offered luna
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // return insufficient prism->uusd conversion, will require running reverse sim
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ask_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // reverse sim, reverse sim requires 80 prism to generate the required fee
    deps.querier.with_astroport_reverse_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ust_asset_info,
        ReverseSimulationResponse {
            offer_amount: Uint128::from(astro_prism_required_for_fee),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit, offering a token (prism)
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset,
        &ask_asset,
        &return_asset,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );
}

/*
test_execute_yluna_prism_insufficient_return_amount
    - offer 1000 yluna, asking for 1000 prism
    - 1200 prism received during astroport swap
    - total prism fee = 1200 * .05 = 60
    - astroport converts 60 prism to (min_fee_value-1) uusd, no good
    - reverse sim results in 250 prism needed to meet fee rquirement
    - returned asset after fees = 1200 - 250 = 950
    - 950 < ask amount (1000), return failure
*/
#[test]
pub fn test_execute_yluna_prism_insufficient_return_amount() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ust_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    let astro_swap_return = Uint128::from(1200u128);
    let astro_fee_uusd = config.min_fee_value - Uint128::from(1u128);
    let astro_prism_required_for_fee = Uint128::from(250u128);

    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // return 1200 for the offered luna
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // return insufficient prism->uusd conversion, will require running reverse sim
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ask_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // reverse sim, reverse sim requires 250 prism to generate the required fee
    deps.querier.with_astroport_reverse_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ust_asset_info,
        ReverseSimulationResponse {
            offer_amount: Uint128::from(astro_prism_required_for_fee),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit, offering a token (prism)
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // failed execution, insufficient return amount
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(res, StdError::generic_err("insufficient return amount"));
}

/*
test_execute_prism_yluna
    - offer 1000 prism, asking for 1000 luna
    - 1200 luna received during astroport swap
    - total prism fee = 1000 * .05 = 50
    - astroport converts 50 prism to min_fee_value uusd, all good
    - returned asset after fees = 1200 - 50 = 1150
    - executor_fee = 50 * .25 = 12
    - protocol_fee = 50 - 12 = 38
*/
#[test]
pub fn test_execute_prism_yluna() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };

    let astro_swap_return = Uint128::from(1200u128);
    let astro_fee_uusd = config.min_fee_value;

    let prism_fee_amount = offer_asset.amount * config.order_fee;
    let offer_asset_after_fees = Asset {
        info: offer_asset_info.clone(),
        amount: offer_asset.amount - prism_fee_amount,
    };
    let return_asset = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // return 1200 luna for the offered prism
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // configure astroport to return sufficient prism->uusd conversion to meet fee min
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset_after_fees,
        &ask_asset,
        &return_asset,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );
}

/*
test_execute_prism_yluna_reverse_sim
    - offer 1000 prism, asking for 1000 luna
    - 1200 luna received during astroport swap
    - total prism fee = 1000 * .05 = 50
    - astroport converts 50 prism to (min_fee_value-1) uusd, no good
    - reverse sim results in 80 luna needed to meet fee rquirement
    - returned asset after fees = 1200 - 80 = 1120
    - executor_fee = 80 * .25 = 20
    - protocol_fee = 80 - 20 = 60
*/
#[test]
pub fn test_execute_prism_yluna_reverse_sim() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ust_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };

    let astro_swap_return = Uint128::from(1200u128);
    let astro_fee_uusd = config.min_fee_value - Uint128::from(1u128);
    let astro_prism_required_for_fee = Uint128::from(80u128);

    let prism_fee_amount = astro_prism_required_for_fee;
    let offer_asset_after_fees = Asset {
        info: offer_asset_info.clone(),
        amount: offer_asset.amount - prism_fee_amount,
    };
    let return_asset = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // return 1200 yluna for the offered prism
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // return insufficient prism->uusd conversion, will require running reverse sim
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // reverse sim, reverse sim requires 80 prism to generate the required fee
    deps.querier.with_astroport_reverse_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ust_asset_info,
        ReverseSimulationResponse {
            offer_amount: Uint128::from(astro_prism_required_for_fee),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit, offering a token (prism)
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset_after_fees,
        &ask_asset,
        &return_asset,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );
}

/*
test_execute_yluna_prism
    - offer 1000 luna, asking for 1000 prism
    - 1200 prism received during astroport swap
    - total prism fee = 1200 * .05 = 60
    - astroport converts 60 prism to min_fee_value uusd, all good
    - returned asset after fees = 1200 - 60 = 1140
    - executor_fee = 60 * .25 = 15
    - protocol_fee = 60 - 15 = 45
*/
#[test]
pub fn test_execute_luna_prism() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::NativeToken {
        denom: "uluna".to_string(),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };

    let astro_swap_return = Uint128::from(1200u128);
    let astro_fee_uusd = config.min_fee_value;

    let prism_fee_amount = astro_swap_return * config.order_fee;
    let return_asset = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return - prism_fee_amount,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ask_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit, offering uluna
    let funds = vec![Coin::new(1000, "uluna")];
    submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset,
        &ask_asset,
        &return_asset,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );
}

#[test]
pub fn test_execute_order_not_found() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // failed execution, order id doesn't exist
    let msg = ExecuteMsg::ExecuteOrder { order_id: 2 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert!(matches!(res, StdError::NotFound { .. }));
}

/*
test_execute_yluna_prism_no_excess:
    - no excess fee available
*/
#[test]
pub fn test_execute_yluna_prism_no_excess() {
    let mut deps = mock_dependencies(&[]);
    init(&mut deps).unwrap();
    let info = mock_info(EXECUTOR_ADDR, &[]);

    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };

    let astro_swap_return = Uint128::from(1052u128);
    let astro_fee_uusd = config.min_fee_value;

    let prism_fee_amount = astro_swap_return * config.order_fee;
    let return_asset_after_fees = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return - prism_fee_amount,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // configure astroport to return 1200 for the offered luna
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // configure astroport to return sufficient prism->uusd conversion to meet fee min
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ask_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset,
        &ask_asset,
        &return_asset_after_fees,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );
}

/*
test_execute_yluna_prism_no_protocol_fee:
    - no protocol fee available to send to fee collector
*/
#[test]
pub fn test_execute_yluna_prism_no_protocol_fee() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(OWNER_ADDR, &[]);
    let msg = InstantiateMsg {
        base_denom: "uusd".to_string(),
        prism_token: PRISM_ADDR.to_string(),
        fee_collector_addr: FEE_COLLECTOR_ADDR.to_string(),
        prism_ust_pair: PRISM_UST_PAIR_ADDR.to_string(),
        order_fee: Decimal::from_str("0.05").unwrap(),
        min_fee_value: Uint128::from(100u128),
        executor_fee_portion: Decimal::one(),
        excess_collector_addr: EXCESS_COLLECTOR_ADDR.to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info(EXECUTOR_ADDR, &[]);
    // query the config to obtain the current config values
    let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();

    let offer_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };

    let astro_swap_return = Uint128::from(1060u128);
    let astro_fee_uusd = config.min_fee_value;

    let prism_fee_amount = astro_swap_return * config.order_fee;
    let return_asset_after_fees = Asset {
        info: ask_asset_info.clone(),
        amount: astro_swap_return - prism_fee_amount,
    };
    let executor_fee_amount = prism_fee_amount * config.executor_fee_portion;
    let protocol_fee_amount = prism_fee_amount - executor_fee_amount;
    let asset_infos = [offer_asset_info.clone(), ask_asset_info.clone()];
    let assets = [offer_asset.clone(), ask_asset.clone()];
    let pair_addr = get_pair_addr(&asset_infos);

    // add trading pair
    add_trading_pair(&mut deps, &asset_infos, OWNER_ADDR).unwrap();

    // configure astroport to return 1200 for the offered luna
    deps.querier.with_astroport_sim_response(
        &pair_addr,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_swap_return),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // configure astroport to return sufficient prism->uusd conversion to meet fee min
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &ask_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(astro_fee_uusd),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit
    submit_order(&mut deps, &assets, USER1_ADDR, &vec![]).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    verify_execute_response(
        &res,
        &pair_addr,
        &offer_asset,
        &ask_asset,
        &return_asset_after_fees,
        &executor_fee_amount,
        &protocol_fee_amount,
        1u32,
    );
}

#[test]
pub fn test_execute_with_inter_pair() {
    let mut deps = mock_dependencies(&[]);
    let info = mock_info(OWNER_ADDR, &[]);
    let msg = InstantiateMsg {
        base_denom: "uusd".to_string(),
        prism_token: PRISM_ADDR.to_string(),
        fee_collector_addr: FEE_COLLECTOR_ADDR.to_string(),
        prism_ust_pair: PRISM_UST_PAIR_ADDR.to_string(),
        order_fee: Decimal::from_str("0.05").unwrap(),
        min_fee_value: Uint128::from(100u128),
        executor_fee_portion: Decimal::one(),
        excess_collector_addr: EXCESS_COLLECTOR_ADDR.to_string(),
    };
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = mock_info(EXECUTOR_ADDR, &[]);

    let offer_asset_info = AssetInfo::NativeToken {
        denom: "uusd".to_string(),
    };
    let ask_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(YLUNA_ADDR),
    };
    let prism_asset_info = AssetInfo::Token {
        contract_addr: Addr::unchecked(PRISM_ADDR),
    };
    let offer_asset = Asset {
        info: offer_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };
    let ask_asset = Asset {
        info: ask_asset_info.clone(),
        amount: Uint128::from(1000u128),
    };

    let assets = [offer_asset.clone(), ask_asset.clone()];

    let asset_infos_1 = [offer_asset_info.clone(), prism_asset_info.clone()];
    let asset_infos_2 = [prism_asset_info.clone(), ask_asset_info.clone()];

    // add trading pairs
    add_trading_pair(&mut deps, &asset_infos_1, OWNER_ADDR).unwrap();
    add_trading_pair(&mut deps, &asset_infos_2, OWNER_ADDR).unwrap();

    let pair_1 = get_pair_addr(&asset_infos_1);
    let pair_2 = get_pair_addr(&asset_infos_2);
    // configure astroport to return 500 for the offered UST
    deps.querier.with_astroport_sim_response(
        &pair_1,
        &offer_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(500u128),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // configure astroport to return sufficient fee value
    deps.querier.with_astroport_sim_response(
        PRISM_UST_PAIR_ADDR,
        &prism_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(100u128),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // configure astroport to return sufficient final swap value with 10 excess
    deps.querier.with_astroport_sim_response(
        &pair_2,
        &prism_asset_info,
        SimulationResponse {
            return_amount: Uint128::from(1000u128),
            spread_amount: Uint128::zero(),
            commission_amount: Uint128::zero(),
        },
    );

    // successful submit
    let funds = vec![Coin::new(1000, "uusd")];
    submit_order(&mut deps, &assets, USER1_ADDR, &funds).unwrap();

    // successful execution
    let msg = ExecuteMsg::ExecuteOrder { order_id: 1 };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            Attribute {
                key: "action".to_string(),
                value: "execute_order".to_string()
            },
            Attribute {
                key: "order_id".to_string(),
                value: "1".to_string()
            },
            Attribute {
                key: "executor_fee_amount".to_string(),
                value: "25".to_string()
            },
            Attribute {
                key: "protocol_fee_amount".to_string(),
                value: "0".to_string()
            },
            Attribute {
                key: "excess_amount".to_string(),
                value: "0".to_string()
            }
        ]
    );
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: pair_1.to_string(),
                funds: vec![Coin {
                    denom: "uusd".to_string(),
                    amount: Uint128::from(1000u128),
                }],
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: offer_asset.clone(),
                    to: None,
                    belief_price: None,
                    max_spread: None,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: PRISM_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: pair_2.to_string(),
                    amount: Uint128::from(475u128), // 500 - 25
                    msg: to_binary(&PairCw20HookMsg::Swap {
                        to: None,
                        belief_price: None,
                        max_spread: None,
                    })
                    .unwrap(),
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: ask_asset.info.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: USER1_ADDR.to_string(),
                    amount: ask_asset.amount,
                })
                .unwrap(),
            })),
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: PRISM_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: EXECUTOR_ADDR.to_string().clone(),
                    amount: Uint128::from(25u128),
                })
                .unwrap(),
            }))
        ]
    )
}
