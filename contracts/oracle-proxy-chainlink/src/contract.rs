use cosmwasm_bignumber::{Decimal256, Uint256};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, QueryRequest, Response,
    StdResult, WasmQuery,
};

use cw2::set_contract_version;
use tefi_oracle::de::deserialize_key;
use tefi_oracle::proxy::{ProxyPriceResponse, ProxyQueryMsg};

use crate::msg::{
    AggregatorQuery, AggregatorQueryMsg, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
    RoundDataResponse, SourcesResponse,
};
use crate::state::{Config, CONFIG, SOURCES};
use crate::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "tefi-oracle-proxy-chainlink";
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
        ExecuteMsg::SetSources { sources } => set_sources(deps, info, sources),
        ExecuteMsg::RemoveSource { asset_token } => remove_source(deps, info, asset_token),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Sources { asset_token } => to_binary(&query_sources(deps, asset_token)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
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

/// @dev Registers Chainlink price sources
/// @param sources : Array of mappings (asset_token, chainlink_source)
pub fn set_sources(
    deps: DepsMut,
    info: MessageInfo,
    sources: Vec<(String, String)>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    for (asset_token, source) in sources {
        let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
        let source: Addr = deps.api.addr_validate(&source)?;

        SOURCES.save(deps.storage, &asset_token, &source)?;
    }

    Ok(Response::default())
}

/// @dev Removes an existing Chainlink price source for an asset_token
/// @param asset_token : Address of the asset_token to remove
pub fn remove_source(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    SOURCES.remove(deps.storage, &asset_token);

    Ok(Response::default())
}

////////////////////////////////////////////////////////////////////////////////////////////////
/// Query implementations
////////////////////////////////////////////////////////////////////////////////////////////////

/// @dev Queries the contract configuration
pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

/// @dev Queries the registered Chainlink prices sources
/// @param asset_token : (Optional) Asset token address, if not provided, returns all sources
pub fn query_sources(
    deps: Deps,
    asset_token: Option<String>,
) -> Result<SourcesResponse, ContractError> {
    let sources: Vec<(String, String)> = match asset_token {
        Some(asset_token) => {
            let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
            let source = SOURCES.load(deps.storage, &asset_token).map_err(|_| {
                ContractError::ProxyError {
                    reason: "Price source not registered".to_string(),
                }
            })?;

            vec![(asset_token.to_string(), source.to_string())]
        }
        None => SOURCES
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (k, v) = item.unwrap();
                let asset_token = deserialize_key::<Addr>(k).unwrap();

                (asset_token.to_string(), v.to_string())
            })
            .collect(),
    };

    Ok(SourcesResponse { sources })
}

/// @dev Queries last price feed for the asset_token by fetching from Chainlink source and converts to standard format
/// @param asset_token : Asset token address
pub fn query_price(deps: Deps, asset_token: String) -> Result<ProxyPriceResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;

    let source: Addr =
        SOURCES
            .load(deps.storage, &asset_token)
            .map_err(|_| ContractError::ProxyError {
                reason: "Price source not registered".to_string(),
            })?;

    let res: RoundDataResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: source.to_string(),
        msg: to_binary(&AggregatorQuery {
            aggregator_query: AggregatorQueryMsg::GetLatestRoundData {},
        })?,
    }))?;

    if res.answer.is_none() || res.updated_at.is_none() {
        return Err(ContractError::ProxyError {
            reason: "Source did not return answer".to_string(),
        });
    }

    let parsed_rate: Decimal256 = Decimal256::from_ratio(
        Uint256::from(res.answer.unwrap()),
        Uint256::from(1e8 as u128),
    );

    Ok(ProxyPriceResponse {
        rate: parsed_rate.into(),
        last_updated: res.updated_at.unwrap(),
    })
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Uint128;

    use super::*;

    #[test]
    fn test_parse_chainlink() {
        let res = RoundDataResponse {
            round_id: 205531,
            answer: Some(Uint128::from_str("350456000000").unwrap()),
            started_at: Some(1637995747),
            updated_at: Some(1637995754),
            answered_in_round: 205531,
        };

        let parsed_rate: Decimal256 = Decimal256::from_ratio(
            Uint256::from(res.answer.unwrap()),
            Uint256::from(1e8 as u128),
        );

        assert_eq!(parsed_rate, Decimal256::from_str("3504.56000000").unwrap())
    }
}
