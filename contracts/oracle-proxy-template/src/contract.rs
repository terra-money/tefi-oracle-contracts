#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use cw2::set_contract_version;
use tefi_oracle::proxy::{ProxyPriceResponse, ProxyQueryMsg};

use crate::msg::{ConfigResponse, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "tefi-oracle-proxy-template";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        source_addr: deps.api.addr_validate(&msg.source_addr)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        // Any custom query msgs
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        // Implementation of the queries required by proxy contract standard
        QueryMsg::Base(proxy_msg) => match proxy_msg {
            ProxyQueryMsg::Price { symbol } => to_binary(&query_price(deps, env, symbol)?),
        },
    };

    res.map_err(|err| err.into())
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

pub fn query_price(_deps: Deps, _env: Env, _symbol: String) -> StdResult<ProxyPriceResponse> {
    // fetch the price from the corresponding source and convert to the standard format
    // pub struct ProxyPriceResponse {
    //     pub rate: Decimal, // rate denominated in base_denom
    //     pub last_updated: u64, // timestamp in seconds
    // }

    unimplemented!()
}
