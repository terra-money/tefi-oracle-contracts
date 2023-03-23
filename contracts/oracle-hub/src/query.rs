use cosmwasm_std::{Addr, Deps, Env, Order, StdError, StdResult};
use cw_storage_plus::Bound;
use tefi_oracle::{
    de::deserialize_key,
    errors::ContractError,
    hub::{
        AllSourcesResponse, AssetSymbolMapResponse, ConfigResponse, PriceListResponse,
        PriceQueryResult, PriceResponse, ProxyInfoResponse, ProxyWhitelistResponse,
        SourcesResponse,
    },
    proxy::ProxyPriceResponse,
    querier::query_proxy_symbol_price,
};

use crate::state::{
    Config, ProxyInfo, ProxyWhitelist, Sources, ASSET_SYMBOL_MAP, CONFIG, SOURCES, WHITELIST,
};

const DEFAULT_PAGINATION_LIMIT: u32 = 10u32;
const MAX_PAGINATION_LIMIT: u32 = 30u32;

/// Queries the contract configuration
pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

/// Queries the proxy whitelist
pub fn query_proxy_whitelist(deps: Deps) -> Result<ProxyWhitelistResponse, ContractError> {
    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;

    Ok(whitelist.as_res())
}

/// Queries the list of registered proxies for an `asset_token` or `symbol`
pub fn query_sources(
    deps: Deps,
    asset_token: Option<String>,
    symbol: Option<String>,
) -> Result<SourcesResponse, ContractError> {
    let symbol = match symbol {
        Some(v) => v,
        None => {
            if let Some(asset_token) = asset_token {
                ASSET_SYMBOL_MAP
                    .load(deps.storage, asset_token.into_bytes())
                    .map_err(|_| ContractError::MappingNotFound {})?
            } else {
                return Err(ContractError::Std(StdError::generic_err(
                    "symbol or asset_token must be provided",
                )));
            }
        }
    };

    let sources_list: Sources = SOURCES
        .load(deps.storage, symbol.into_bytes())
        .map_err(|_| ContractError::SymbolNotRegistered {})?;
    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;

    Ok(sources_list.as_res(&whitelist))
}

/// Queries the available price with highest priority.
/// `asset_token` or `symbol` must be provided
pub fn query_price(
    deps: Deps,
    env: Env,
    asset_token: Option<String>,
    symbol: Option<String>,
    timeframe: Option<u64>,
) -> Result<PriceResponse, ContractError> {
    let symbol = match symbol {
        Some(v) => v,
        None => {
            if let Some(asset_token) = asset_token {
                ASSET_SYMBOL_MAP
                    .load(deps.storage, asset_token.into_bytes())
                    .map_err(|_| ContractError::MappingNotFound {})?
            } else {
                // internal error, should never happen
                return Err(ContractError::Std(StdError::generic_err(
                    "symbol or asset_token must be provided",
                )));
            }
        }
    };

    let sources: Sources = SOURCES
        .load(deps.storage, symbol.clone().into_bytes())
        .map_err(|_| ContractError::SymbolNotRegistered {})?;

    let time_threshold = match timeframe {
        Some(v) => env.block.time.minus_seconds(v).seconds(),
        None => 0u64,
    };

    for (_prio, proxy_addr) in sources.proxies {
        let proxy_price: ProxyPriceResponse =
            match query_proxy_symbol_price(&deps.querier, &proxy_addr, symbol.clone()) {
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

///  Queries prices from all registered proxies for an `asset_token` or `symbol`
pub fn query_price_list(
    deps: Deps,
    asset_token: Option<String>,
    symbol: Option<String>,
) -> Result<PriceListResponse, ContractError> {
    let symbol = match symbol {
        Some(v) => v,
        None => {
            if let Some(asset_token) = asset_token {
                ASSET_SYMBOL_MAP.load(deps.storage, asset_token.into_bytes())?
            } else {
                return Err(ContractError::Std(StdError::generic_err(
                    "symbol or asset_token must be provided",
                )));
            }
        }
    };

    let sources: Sources = SOURCES
        .load(deps.storage, symbol.clone().into_bytes())
        .map_err(|_| ContractError::SymbolNotRegistered {})?;
    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;

    let price_list: Vec<(u8, ProxyInfoResponse, PriceQueryResult)> = sources
        .proxies
        .iter()
        .map(|item| {
            let res = match query_proxy_symbol_price(&deps.querier, &item.1, symbol.clone()) {
                Ok(price_res) => PriceQueryResult::Success(price_res.into()),
                Err(..) => PriceQueryResult::Fail,
            };
            let proxy_info = whitelist.find_by_addr(&item.1).unwrap_or(ProxyInfo {
                address: item.1.clone(),
                provider_name: "No longer whitelisted".to_string(),
            });

            (item.0, proxy_info.as_res(), res)
        })
        .collect();

    Ok(PriceListResponse { price_list })
}

/// Query the map of `asset_token`s to `symbol`s
pub fn query_asset_symbol_map(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<AssetSymbolMapResponse, ContractError> {
    let limit = limit
        .unwrap_or(DEFAULT_PAGINATION_LIMIT)
        .min(MAX_PAGINATION_LIMIT) as usize;
    let start = start_after.map(|addr| Bound::exclusive(addr.into_bytes()));

    let map: Vec<(String, String)> = ASSET_SYMBOL_MAP
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, symbol) = item?;
            let address = deserialize_key::<String>(k).unwrap();
            Ok((address, symbol))
        })
        .collect::<StdResult<Vec<(String, String)>>>()?;

    Ok(AssetSymbolMapResponse { map })
}

/// Queries all sources for all symbols
pub fn query_all_sources(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> Result<AllSourcesResponse, ContractError> {
    let limit = limit
        .unwrap_or(DEFAULT_PAGINATION_LIMIT)
        .min(MAX_PAGINATION_LIMIT) as usize;
    let start = start_after.map(|symbol| Bound::exclusive(symbol.into_bytes()));

    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;
    let list: Vec<SourcesResponse> = SOURCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, sources) = item?;
            Ok(sources.as_res(&whitelist))
        })
        .collect::<StdResult<Vec<SourcesResponse>>>()?;

    Ok(AllSourcesResponse { list })
}

pub fn query_check_source(
    deps: Deps,
    proxy_addr: String,
    symbol: String,
) -> Result<PriceResponse, ContractError> {
    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;

    // check if the proxy is whitelisted
    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.is_whitelisted(&proxy_addr) {
        return Err(ContractError::ProxyNotWhitelisted {});
    }

    // attempt to query price from proxy
    let price_res: ProxyPriceResponse =
        query_proxy_symbol_price(&deps.querier, &proxy_addr, symbol)
            .map_err(|_| ContractError::PriceNotAvailable {})?;

    Ok(price_res.into())
}
