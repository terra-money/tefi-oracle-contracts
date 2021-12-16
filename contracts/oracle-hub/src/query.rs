use cosmwasm_std::{Addr, Deps, Env};
use tefi_oracle::{
    errors::ContractError,
    hub::{
        ConfigResponse, LegacyPriceResponse, PriceListResponse, PriceQueryResult, PriceResponse,
        ProxyListResponse,
    },
    proxy::ProxyPriceResponse,
    querier::query_proxy_asset_price,
};

use crate::state::{Config, ProxyList, ASSETS, CONFIG};

/// @dev Queries the contract configuration
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

/// @dev Queries the list of registered proxies for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
pub fn query_proxy_list(
    deps: Deps,
    asset_token: String,
) -> Result<ProxyListResponse, ContractError> {
    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    Ok(proxy_list.as_res())
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
    let proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    let time_threshold = match timeframe {
        Some(v) => env.block.time.minus_seconds(v).seconds(),
        None => 0u64,
    };

    for (_prio, proxy_addr) in proxy_list.proxies {
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
    let proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    let price_list: Vec<(u8, PriceQueryResult)> = proxy_list
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

/// @dev Queries the price with highest priority and returns it in legacy struct format
/// @param quote : Ignored, should always be base_denom
/// @param base : Asset token address. Native assets are not supported
pub fn query_legacy_price(
    deps: Deps,
    base: String,
    quote: String,
) -> Result<LegacyPriceResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if quote.ne(&config.base_denom) {
        return Err(ContractError::InvalidQuote {});
    }

    let asset_token: Addr = deps.api.addr_validate(&base)?;
    let proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    // TODO: instead of taking highest priority proxy, set a default valid timeframe
    let highest_prio_proxy: Addr = proxy_list.proxies.first().unwrap().1.clone();

    let proxy_price: ProxyPriceResponse =
        query_proxy_asset_price(&deps.querier, &highest_prio_proxy, &asset_token)?;

    Ok(LegacyPriceResponse {
        rate: proxy_price.rate.into(),
        last_updated_base: proxy_price.last_updated,
        last_updated_quote: u64::MAX,
    })
}
