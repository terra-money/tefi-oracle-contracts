#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;

use tefi_oracle::hub::{HubExecuteMsg, HubQueryMsg, InstantiateMsg};

use crate::handle::{register_proxy, remove_proxy, update_owner, update_priority};
use crate::query::{query_config, query_legacy_price, query_price, query_proxy_list};
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
        HubExecuteMsg::RegisterProxy {
            asset_token,
            proxy_addr,
            priority,
        } => register_proxy(deps, info, asset_token, proxy_addr, priority),
        HubExecuteMsg::UpdatePriority {
            asset_token,
            proxy_addr,
            priority,
        } => update_priority(deps, info, asset_token, proxy_addr, priority),
        HubExecuteMsg::RemoveProxy {
            asset_token,
            proxy_addr,
        } => remove_proxy(deps, info, asset_token, proxy_addr),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: HubQueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        HubQueryMsg::Config {} => to_binary(&query_config(deps)?),
        HubQueryMsg::ProxyList { asset_token } => to_binary(&query_proxy_list(deps, asset_token)?),
        HubQueryMsg::Price {
            asset_token,
            timeframe,
        } => to_binary(&query_price(deps, env, asset_token, timeframe)?),
        HubQueryMsg::PriceList { asset_token } => to_binary(&query_proxy_list(deps, asset_token)?),
        HubQueryMsg::LegacyPrice { base, quote } => {
            to_binary(&query_legacy_price(deps, base, quote)?)
        }
    };

    res.map_err(|err| err.into())
}
