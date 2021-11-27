use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, QueryRequest, Response,
    WasmQuery,
};

use cw2::set_contract_version;
use tefi_oracle::de::deserialize_key;
use tefi_oracle::proxy::{ProxyPriceResponse, ProxyQueryMsg};

use crate::msg::{
    BandResponse, ConfigResponse, ExecuteMsg, GetReferenceData, InstantiateMsg, QueryMsg,
    SymbolMapResponse,
};
use crate::state::{Config, CONFIG, SYMBOLS};
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
        ExecuteMsg::SetSymbolMapping {
            asset_token,
            symbol,
        } => set_symbol_mapping(deps, info, asset_token, symbol),
        ExecuteMsg::RemoveSymbolMapping { asset_token } => {
            remove_symbol_mapping(deps, info, asset_token)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::SymbolMap { asset_token } => to_binary(&query_symbol_map(deps, asset_token)?),
        // Implementation of the queries required by proxy contract standard
        QueryMsg::Base(proxy_msg) => match proxy_msg {
            ProxyQueryMsg::Price { asset_token } => to_binary(&query_price(deps, asset_token)?),
        },
    };

    res.map_err(|err| err.into())
}

////////////////////////////////////////////////////////////////////////////////////////////////
/// Execute implementations
////////////////////////////////////////////////////////////////////////////////////////////////

/// @dev Updates the owner addres
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

/// @dev Registers a new asset_token/symbol mapping
/// @param asset_token : Asset token address
/// @param symbol : Symbol that identifies the asset on Band Protocol price source
pub fn set_symbol_mapping(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    symbol: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;

    // overwrite if exists
    SYMBOLS.save(deps.storage, &asset_token, &symbol)?;

    Ok(Response::default())
}

/// @dev Removes an existing asset_token/symbol mapping
/// @param asset_token : Asset token address to remove
pub fn remove_symbol_mapping(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;

    SYMBOLS.remove(deps.storage, &asset_token);

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

/// @dev Queries the asset_token/symbol map
/// @param asset_token : (Optional) To query the map for a single asset
pub fn query_symbol_map(
    deps: Deps,
    asset_token: Option<String>,
) -> Result<SymbolMapResponse, ContractError> {
    let map: Vec<(String, String)> = match asset_token {
        Some(asset_token) => {
            let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
            let symbol = SYMBOLS.load(deps.storage, &asset_token).map_err(|_| {
                ContractError::ProxyError {
                    reason: "Symbol not registered".to_string(),
                }
            })?;

            vec![(asset_token.to_string(), symbol)]
        }
        None => SYMBOLS
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (k, v) = item.unwrap();
                let asset_token = deserialize_key::<Addr>(k).unwrap();

                (asset_token.to_string(), v)
            })
            .collect(),
    };

    Ok(SymbolMapResponse { map })
}

/// @dev Queries the price by fetching it from band source using the previously mapped symbol and converts to the standard
/// @param asset_token : Asset token address
pub fn query_price(deps: Deps, asset_token: String) -> Result<ProxyPriceResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;

    let symbol: String =
        SYMBOLS
            .load(deps.storage, &asset_token)
            .map_err(|_| ContractError::ProxyError {
                reason: "Symbol not registered".to_string(),
            })?;

    let res: BandResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.source_addr.to_string(),
        msg: to_binary(&GetReferenceData {
            base_symbol: symbol,
            quote_symbol: config.quote_symbol,
        })?,
    }))?;

    let parsed_rate: Decimal256 = Decimal256::from_ratio(res.rate, Uint256::from(1e18 as u128));

    Ok(ProxyPriceResponse {
        rate: parsed_rate.into(),
        last_updated: res.last_updated_base as u64,
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_band() {
        let band_res = BandResponse {
            rate: Uint256::from_str("1082780049999000000000").unwrap(),
            last_updated_base: 1637951384,
            last_updated_quote: 18446744073709552000,
        };

        let parsed_rate: Decimal256 =
            Decimal256::from_ratio(band_res.rate, Uint256::from(1e18 as u128));

        assert_eq!(
            parsed_rate,
            Decimal256::from_str("1082.780049999000000000").unwrap()
        )
    }
}
