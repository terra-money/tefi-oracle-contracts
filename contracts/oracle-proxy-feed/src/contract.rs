#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Attribute, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
};

use cw2::set_contract_version;
use tefi_oracle::proxy::{ProxyPriceResponse, ProxyQueryMsg};

use crate::msg::{ConfigResponse, ExecuteMsg, FeederResponse, InstantiateMsg, QueryMsg};
use crate::state::{Config, PriceInfo, CONFIG, FEEDERS, PRICES};
use crate::ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "tefi-oracle-proxy-feed";
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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => update_owner(deps, info, owner),
        ExecuteMsg::RegisterFeed {
            asset_token,
            feeder,
        } => register_feed(deps, info, asset_token, feeder),
        ExecuteMsg::FeedPrices { prices } => feed_prices(deps, env, info, prices),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Feeder { asset_token } => to_binary(&query_feeder(deps, asset_token)?),
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

/// @dev Registers a new feeder or updates an existing one
/// @param asset_token : Asset token address
/// @param feeder : Address authorized to feed prices for the asset_token
pub fn register_feed(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    feeder: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let feeder: Addr = deps.api.addr_validate(&feeder)?;

    // overwrite if exists
    FEEDERS.save(deps.storage, &asset_token, &feeder)?;

    Ok(Response::default())
}

/// @dev Feeder operation to feed prices to one or multiple asset tokens
/// @param prices : Array of price feeds (asset_token, price)
pub fn feed_prices(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    prices: Vec<(String, Decimal)>,
) -> Result<Response, ContractError> {
    let mut attributes: Vec<Attribute> = vec![attr("action", "price_feed")];
    for price in prices {
        attributes.push(attr("asset", price.0.to_string()));
        attributes.push(attr("price", price.1.to_string()));

        // Check feeder permission
        let asset_token: Addr = deps.api.addr_validate(&price.0)?;
        let registered_feeder: Addr =
            FEEDERS
                .load(deps.storage, &asset_token)
                .map_err(|_| ContractError::ProxyError {
                    reason: "There is no feeder registered for the provided asset".to_string(),
                })?;

        if registered_feeder.ne(&info.sender) {
            return Err(ContractError::Unauthorized {});
        }

        PRICES.save(
            deps.storage,
            &asset_token,
            &PriceInfo {
                price: price.1,
                last_updated_time: env.block.time.seconds(),
            },
        )?;
    }

    Ok(Response::new().add_attributes(attributes))
}

////////////////////////////////////////////////////////////////////////////////////////////////
/// Query implementations
////////////////////////////////////////////////////////////////////////////////////////////////

/// @dev Queries the contract configuration
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

/// @dev Queries the registered feeder for an asset_token
/// @param asset_token : Asset token address
pub fn query_feeder(deps: Deps, asset_token: String) -> Result<FeederResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;

    let registered_feeder: Addr =
        FEEDERS
            .load(deps.storage, &asset_token)
            .map_err(|_| ContractError::ProxyError {
                reason: "There is no feeder registered for the provided asset".to_string(),
            })?;

    Ok(FeederResponse {
        asset_token: asset_token.to_string(),
        feeder: registered_feeder.to_string(),
    })
}

/// @dev Queries last price feed for the asset_token
/// @param asset_token : Asset token address
pub fn query_price(deps: Deps, asset_token: String) -> Result<ProxyPriceResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;

    let price_info: PriceInfo =
        PRICES
            .load(deps.storage, &asset_token)
            .map_err(|_| ContractError::ProxyError {
                reason: "There is no price feed for the requested asset_token".to_string(),
            })?;

    Ok(price_info.as_res())
}
