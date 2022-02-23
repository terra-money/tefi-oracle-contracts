use crate::{
    state::{Config, ProxyWhitelist, Sources, SOURCES, CONFIG, WHITELIST},
    ContractError,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};
use tefi_oracle::hub::{DEFAULT_PRIORITY, MAX_WHITELISTED_PROXIES};

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

/// @dev Updates the max_proxies_per_asset parameter
/// @param owner : New maximum number of proxies per asset
pub fn update_max_proxies(
    deps: DepsMut,
    info: MessageInfo,
    max_proxies_per_asset: u8,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    config.max_proxies_per_asset = max_proxies_per_asset;

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

/// @dev Registers a new price proxy contract for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
/// @param proxy_addr : Proxy contract address
/// @param priority : Priority number (lowest value has higher priority)
pub fn register_source(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    proxy_addr: String,
    priority: Option<u8>,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;
    let priority: u8 = priority.unwrap_or(DEFAULT_PRIORITY);

    let mut sources: Sources = SOURCES.load(deps.storage, &asset_token).unwrap_or(Sources {
        asset_token: asset_token.clone(),
        proxies: vec![],
    });

    if sources.proxies.len() >= config.max_proxies_per_asset as usize {
        return Err(ContractError::TooManyProxiesForAsset {
            max: config.max_proxies_per_asset,
        });
    }

    sources.proxies.push((priority, proxy_addr));
    // sort before storing
    sources.sort_by_priority();

    SOURCES.save(deps.storage, &asset_token, &sources)?;

    Ok(Response::default())
}

/// @dev Changes the priority value for an existing price proxy
/// @param asset_token : Asset token address. Native assets are not supported
/// @param proxy_addr : Proxy contract address
/// @param priority : New priority number (lowest value has higher priority)
pub fn update_source_priority(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    proxy_addr: String,
    priority: u8,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;

    let mut sources: Sources = SOURCES
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    sources.update_proxy_priority(&proxy_addr, priority)?;
    // sort before storing
    sources.sort_by_priority();

    SOURCES.save(deps.storage, &asset_token, &sources)?;

    Ok(Response::default())
}

/// @dev Removes an existing price proxy for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
/// @param proxy_addr : Proxy contract address
pub fn remove_source(
    deps: DepsMut,
    info: MessageInfo,
    asset_token: String,
    proxy_addr: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !config.is_owner(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let asset_token: Addr = deps.api.addr_validate(&asset_token)?;
    let proxy_addr: Addr = deps.api.addr_validate(&proxy_addr)?;

    let mut sources: Sources = SOURCES
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    sources.remove(&proxy_addr)?;

    SOURCES.save(deps.storage, &asset_token, &sources)?;

    Ok(Response::default())
}

///
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

///
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
