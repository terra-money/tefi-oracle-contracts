use cosmwasm_std::{Addr, Deps, Env};
use tefi_oracle::{
    errors::ContractError,
    hub::{
        ConfigResponse, PriceListResponse, PriceQueryResult, PriceResponse,
        ProxyWhitelistResponse, SourcesResponse,
    },
    proxy::ProxyPriceResponse,
    querier::query_proxy_asset_price,
};

use crate::state::{Config, ProxyWhitelist, Sources, SOURCES, CONFIG, WHITELIST};

/// @dev Queries the contract configuration
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

///
pub fn query_proxy_whitelist(deps: Deps) -> Result<ProxyWhitelistResponse, ContractError> {
    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;

    Ok(whitelist.as_res())
}

/// @dev Queries the list of registered proxies for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
pub fn query_sources(deps: Deps, asset_token: String) -> Result<SourcesResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let sources_list: Sources = SOURCES
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    Ok(sources_list.as_res())
}

/// @dev Queries the available price with highest priority
/// @param asset_token : Asset token address. Native assets are not supported
/// @param timeframe : Valid price timeframe in seconds
pub fn query_price(
    deps: Deps,
    env: Env,
    asset_token: String,
    timeframe: Option<u64>,
) -> Result<PriceResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let sources: Sources = SOURCES
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    let time_threshold = match timeframe {
        Some(v) => env.block.time.minus_seconds(v).seconds(),
        None => 0u64,
    };

    for (_prio, proxy_addr) in sources.proxies {
        let proxy_price: ProxyPriceResponse =
            match query_proxy_asset_price(&deps.querier, &proxy_addr, &asset_token) {
                Ok(res) => res,
                Err(..) => continue,
            };

        // if time_threshold is 0, always false
        if proxy_price.last_updated < time_threshold {
            continue;
        }

        return Ok(proxy_price.into());
    }

    Err(ContractError::PriceNotAvailable {})
}

/// @dev Queries prices from all registered proxies for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
pub fn query_price_list(
    deps: Deps,
    asset_token: String,
) -> Result<PriceListResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let sources: Sources = SOURCES
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    let price_list: Vec<(u8, PriceQueryResult)> = sources
        .proxies
        .iter()
        .map(|item| {
            let res = match query_proxy_asset_price(&deps.querier, &item.1, &asset_token) {
                Ok(price_res) => PriceQueryResult::Success(price_res.into()),
                Err(..) => PriceQueryResult::Fail,
            };

            (item.0, res)
        })
        .collect();

    Ok(PriceListResponse { price_list })
}
