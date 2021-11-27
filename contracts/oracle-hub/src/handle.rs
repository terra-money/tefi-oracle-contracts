use crate::{
    state::{Config, ProxyList, ASSETS, CONFIG},
    ContractError,
};
use cosmwasm_std::{Addr, DepsMut, MessageInfo, Response};

const DEFAULT_PRIORITY: u8 = 10;

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

/// @dev Registers a new price proxy contract for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
/// @param proxy_addr : Proxy contract address
/// @param priority : Priority number (lowest value has higher priority)
pub fn register_proxy(
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

    let mut proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .unwrap_or(ProxyList {
            asset_token: asset_token.clone(),
            proxies: vec![],
        });

    proxy_list.proxies.push((priority, proxy_addr));

    ASSETS.save(deps.storage, &asset_token, &proxy_list)?;

    Ok(Response::default())
}

/// @dev Changes the priority value for an existing price proxy
/// @param asset_token : Asset token address. Native assets are not supported
/// @param proxy_addr : Proxy contract address
/// @param priority : New priority number (lowest value has higher priority)
pub fn update_priority(
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

    let mut proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    proxy_list.update_proxy_priority(&proxy_addr, priority)?;

    ASSETS.save(deps.storage, &asset_token, &proxy_list)?;

    Ok(Response::default())
}

/// @dev Remvoes an existing price proxy for an asset_token
/// @param asset_token : Asset token address. Native assets are not supported
/// @param proxy_addr : Proxy contract address
pub fn remove_proxy(
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

    let mut proxy_list: ProxyList = ASSETS
        .load(deps.storage, &asset_token)
        .map_err(|_| ContractError::AssetNotRegistered {})?;

    proxy_list.remove(&proxy_addr)?;

    ASSETS.save(deps.storage, &asset_token, &proxy_list)?;

    Ok(Response::default())
}
