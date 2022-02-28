use crate::{
    state::{Config, ProxyWhitelist, Sources, ASSET_SYMBOL_MAP, CONFIG, SOURCES, WHITELIST},
    ContractError,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};
use tefi_oracle::hub::{DEFAULT_PRIORITY, MAX_WHITELISTED_PROXIES};

/// Updates the owner address
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

/// Updates the `max_proxies_per_asset` parameter
pub fn update_max_proxies(
    deps: DepsMut,
    info: MessageInfo,
    max_proxies_per_symbol: u8,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    config.max_proxies_per_symbol = max_proxies_per_symbol;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

/// Registers a new price proxy contract for a symbol
pub fn register_source(
    deps: DepsMut,
    info: MessageInfo,
    symbol: String,
    proxy_addr: String,
    priority: Option<u8>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;
    let priority: u8 = priority.unwrap_or(DEFAULT_PRIORITY);

    // check if the proxy is whitelisted
    let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.is_whitelisted(&proxy_addr) {
        return Err(ContractError::ProxyNotWhitelisted {});
    }

    let mut sources: Sources = SOURCES
        .load(deps.storage, symbol.as_bytes())
        .unwrap_or(Sources {
            symbol: symbol.clone(),
            proxies: vec![],
        });

    if sources.proxies.len() >= config.max_proxies_per_symbol as usize {
        return Err(ContractError::TooManyProxiesForSymbol {
            max: config.max_proxies_per_symbol,
        });
    }

    if sources.is_registered(&proxy_addr) {
        return Err(ContractError::ProxyAlreadyRegistered {});
    }

    sources.proxies.push((priority, proxy_addr));
    // sort before storing
    sources.sort_by_priority();

    SOURCES.save(deps.storage, symbol.as_bytes(), &sources)?;

    Ok(Response::default())
}

/// Registers a list of sources
pub fn bulk_register_source(
    deps: DepsMut,
    info: MessageInfo,
    sources: Vec<(String, String, Option<u8>)>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    for source in sources {
        let symbol: String = source.0;
        let proxy_addr: Addr = deps.api.addr_validate(&source.1)?;
        let priority: u8 = source.2.unwrap_or(DEFAULT_PRIORITY);

        // check if the proxy is whitelisted
        let whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;
        if !whitelist.is_whitelisted(&proxy_addr) {
            return Err(ContractError::ProxyNotWhitelisted {});
        }

        let mut sources: Sources =
            SOURCES
                .load(deps.storage, symbol.as_bytes())
                .unwrap_or(Sources {
                    symbol: symbol.clone(),
                    proxies: vec![],
                });

        if sources.proxies.len() >= config.max_proxies_per_symbol as usize {
            return Err(ContractError::TooManyProxiesForSymbol {
                max: config.max_proxies_per_symbol,
            });
        }

        if sources.is_registered(&proxy_addr) {
            return Err(ContractError::ProxyAlreadyRegistered {});
        }

        sources.proxies.push((priority, proxy_addr));
        // sort before storing
        sources.sort_by_priority();

        SOURCES.save(deps.storage, symbol.as_bytes(), &sources)?;
    }

    Ok(Response::default())
}

/// Changes the priority value for one or multiple registered proxies for a symbol
pub fn update_source_priority_list(
    deps: DepsMut,
    info: MessageInfo,
    symbol: String,
    priorities: Vec<(String, u8)>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let mut sources: Sources = SOURCES
        .load(deps.storage, symbol.as_bytes())
        .map_err(|_| ContractError::SymbolNotRegistered {})?;

    // check for duplicates in the input priority list
    let mut priorities_d = priorities.clone();
    priorities_d.sort();
    priorities_d.dedup_by(|item1, item2| item1.0.eq(&item2.0));
    if priorities_d.len() != priorities.len() {
        return Err(ContractError::InvalidPriorities {});
    }

    for item in priorities_d {
        let proxy_addr: Addr = deps.api.addr_validate(&item.0)?;
        // if it is not registered, this will return error
        sources.update_proxy_priority(&proxy_addr, item.1)?;
    }

    // sort before storing
    sources.sort_by_priority();

    SOURCES.save(deps.storage, symbol.as_bytes(), &sources)?;

    Ok(Response::default())
}

/// Removes an existing price proxy for an `asset_token`
pub fn remove_source(
    deps: DepsMut,
    info: MessageInfo,
    symbol: String,
    proxy_addr: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;

    let mut sources: Sources = SOURCES
        .load(deps.storage, symbol.as_bytes())
        .map_err(|_| ContractError::SymbolNotRegistered {})?;

    sources.remove(&proxy_addr)?;

    SOURCES.save(deps.storage, symbol.as_bytes(), &sources)?;

    Ok(Response::default())
}

/// Whitelist a new proxy. After a proxy is whitelisted it can be registered as
/// a source for a given symbol
pub fn whitelist_proxy(
    deps: DepsMut,
    info: MessageInfo,
    proxy_addr: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;
    let mut whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;

    if whitelist
        .proxies
        .len()
        .ge(&(MAX_WHITELISTED_PROXIES as usize))
    {
        return Err(ContractError::TooManyWhitelistedProxies {
            max: MAX_WHITELISTED_PROXIES,
        });
    }

    whitelist.proxies.push(proxy_addr);

    WHITELIST.save(deps.storage, &whitelist)?;

    Ok(Response::default())
}

/// Remove a proxy from the whitelist
pub fn remove_proxy(
    deps: DepsMut,
    info: MessageInfo,
    proxy_addr: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;
    let mut whitelist: ProxyWhitelist = WHITELIST.load(deps.storage)?;

    // returns error if it is not whitelisted
    whitelist.remove(&proxy_addr)?;

    WHITELIST.save(deps.storage, &whitelist)?;

    Ok(Response::default())
}

/// Update the map of `asset_token` => `symbol`
/// ## Params
/// * `items` - Array of (`asset_token`, `symbol`)
pub fn insert_asset_symbol_map(
    deps: DepsMut,
    info: MessageInfo,
    map: Vec<(String, String)>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    for item in map {
        ASSET_SYMBOL_MAP.save(deps.storage, item.0.as_bytes(), &item.1)?;
    }

    Ok(Response::default())
}
