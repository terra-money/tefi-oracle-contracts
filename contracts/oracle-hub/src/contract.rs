#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use tefi_oracle::hub::{HubExecuteMsg, HubQueryMsg, InstantiateMsg};

use crate::handle::{
    insert_asset_symbol_map, register_source, remove_proxy, remove_source, update_max_proxies,
    update_owner, update_source_priority, whitelist_proxy,
};
use crate::query::{
    query_asset_symbol_map, query_config, query_price, query_price_list, query_proxy_whitelist,
    query_sources,
};
use crate::state::{Config, CONFIG};
use crate::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "tefi-oracle-hub";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        base_denom: msg.base_denom,
        max_proxies_per_symbol: msg.max_proxies_per_symbol,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: HubExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        HubExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, owner),
        HubExecuteMsg::UpdateMaxProxies {
            max_proxies_per_symbol,
        } => update_max_proxies(deps, info, max_proxies_per_symbol),
        HubExecuteMsg::RegisterSource {
            symbol,
            proxy_addr,
            priority,
        } => register_source(deps, info, symbol, proxy_addr, priority),
        HubExecuteMsg::UpdateSourcePriority {
            symbol,
            proxy_addr,
            priority,
        } => update_source_priority(deps, info, symbol, proxy_addr, priority),
        HubExecuteMsg::RemoveSource { symbol, proxy_addr } => {
            remove_source(deps, info, symbol, proxy_addr)
        }
        HubExecuteMsg::WhitelistProxy { proxy_addr } => whitelist_proxy(deps, info, proxy_addr),
        HubExecuteMsg::RemoveProxy { proxy_addr } => remove_proxy(deps, info, proxy_addr),
        HubExecuteMsg::InsertAssetSymbolMap { items } => insert_asset_symbol_map(deps, info, items),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: HubQueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        HubQueryMsg::Config {} => to_binary(&query_config(deps)?),
        HubQueryMsg::ProxyWhitelist {} => to_binary(&query_proxy_whitelist(deps)?),
        HubQueryMsg::Sources { asset_token } => {
            to_binary(&query_sources(deps, Some(asset_token), None)?)
        }
        HubQueryMsg::SourcesBySymbol { symbol } => {
            to_binary(&query_sources(deps, None, Some(symbol))?)
        }
        HubQueryMsg::Price {
            asset_token,
            timeframe,
        } => to_binary(&query_price(deps, env, Some(asset_token), None, timeframe)?),
        HubQueryMsg::PriceBySymbol { symbol, timeframe } => {
            to_binary(&query_price(deps, env, None, Some(symbol), timeframe)?)
        }
        HubQueryMsg::PriceList { asset_token } => {
            to_binary(&query_price_list(deps, Some(asset_token), None)?)
        }
        HubQueryMsg::PriceListBySymbol { symbol } => {
            to_binary(&query_price_list(deps, None, Some(symbol))?)
        }
        HubQueryMsg::AssetSymbolMap { start_after, limit } => {
            to_binary(&query_asset_symbol_map(deps, start_after, limit)?)
        }
    };

    res.map_err(|err| err.into())
}
