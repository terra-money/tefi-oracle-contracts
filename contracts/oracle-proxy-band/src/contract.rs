#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response,
    Uint128, WasmQuery,
};

use cw2::set_contract_version;
use tefi_oracle::proxy::{ProxyPriceResponse, ProxyQueryMsg};

use crate::msg::{BandMsg, BandResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "tefi-oracle-proxy-band";
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
        owner: deps.api.addr_validate(&msg.owner)?,
        source_addr: deps.api.addr_validate(&msg.source_addr)?,
        quote_symbol: msg.quote_symbol,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, owner),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        // Implementation of the queries required by proxy contract standard
        QueryMsg::Base(proxy_msg) => match proxy_msg {
            ProxyQueryMsg::Price { symbol } => to_binary(&query_price(deps, symbol)?),
        },
    };

    res.map_err(|err| err.into())
}

////////////////////////////////////////////////////////////////////////////////////////////////
/// Execute implementations
////////////////////////////////////////////////////////////////////////////////////////////////

/// @dev Updates the owner address
/// @param owner : New owner address
pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let owner_addr: Addr = deps.api.addr_validate(&owner)?;
    config.owner = owner_addr;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

////////////////////////////////////////////////////////////////////////////////////////////////
/// Query implementations
////////////////////////////////////////////////////////////////////////////////////////////////

/// @dev Queries the contract configuration
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

/// @dev Queries the price by fetching it from Band source using the previously mapped symbol and converts to the standard format
/// @param asset_token : Asset token address
pub fn query_price(deps: Deps, symbol: String) -> Result<ProxyPriceResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    let res: BandResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.source_addr.to_string(),
        msg: to_binary(&BandMsg::GetReferenceData {
            base_symbol: symbol,
            quote_symbol: config.quote_symbol,
        })?,
    }))?;

    let parsed_rate: Decimal = Decimal::from_ratio(res.rate, Uint128::from(1e18 as u128));

    Ok(ProxyPriceResponse {
        rate: parsed_rate,
        last_updated: res.last_updated_base as u64,
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::{Decimal, Uint128};

    use super::*;

    #[test]
    fn test_parse_band() {
        let band_res = BandResponse {
            rate: Uint128::from_str("1082780049999000000000").unwrap(),
            last_updated_base: 1637951384,
            last_updated_quote: u64::MAX,
        };

        let parsed_rate: Decimal = Decimal::from_ratio(band_res.rate, Uint128::from(1e18 as u128));

        assert_eq!(
            parsed_rate,
            Decimal::from_str("1082.780049999000000000").unwrap()
        )
    }
}
