use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use crate::ContractError;
use tefi_oracle::hub::{ConfigResponse, ProxyWhitelistResponse, SourcesResponse};

pub const CONFIG: Item<Config> = Item::new("config");
// set price sources for each symbol
pub const SOURCES: Map<&[u8], Sources> = Map::new("sources");
// whitelist of proxies that can be added as sources
pub const WHITELIST: Item<ProxyWhitelist> = Item::new("whitelist");
// map of asset cw20 contract addresses to symbol
pub const ASSET_SYMBOL_MAP: Map<&[u8], String> = Map::new("asset_symbol_map");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub owner: Addr,
    // base denom has no utility in the contract, only for information purpose
    // e.g only proxies compatible with the base_denom should be registered
    pub base_denom: String,
    pub max_proxies_per_symbol: u8,
}

impl Config {
    pub fn as_res(&self) -> ConfigResponse {
        ConfigResponse {
            owner: self.owner.to_string(),
            base_denom: self.base_denom.to_string(),
            max_proxies_per_symbol: self.max_proxies_per_symbol,
        }
    }

    /// Checks if the provided `addr` is owner
    pub fn is_owner(&self, addr: &Addr) -> bool {
        self.owner.eq(addr)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ProxyWhitelist {
    pub proxies: Vec<Addr>,
}

impl ProxyWhitelist {
    pub fn as_res(&self) -> ProxyWhitelistResponse {
        ProxyWhitelistResponse {
            proxies: self.proxies.iter().map(|addr| addr.to_string()).collect(),
        }
    }

    pub fn is_whitelisted(&self, proxy_addr: &Addr) -> bool {
        self.proxies.iter().any(|item| item.eq(proxy_addr))
    }

    pub fn remove(&mut self, proxy_addr: &Addr) -> Result<(), ContractError> {
        match self.proxies.iter().position(|item| item.eq(proxy_addr)) {
            Some(position) => {
                self.proxies.remove(position);
                Ok(())
            }
            None => Err(ContractError::ProxyNotWhitelisted {}),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Sources {
    pub symbol: String,
    pub proxies: Vec<(u8, Addr)>,
}

impl Sources {
    /// Sorts the proxy list by priority
    pub fn sort_by_priority(&mut self) {
        self.proxies.sort_by_key(|item| item.0);
    }

    /// Checks if the provided proxy address is already registered
    pub fn is_registered(&self, proxy_addr: &Addr) -> bool {
        self.proxies.iter().any(|item| item.1.eq(proxy_addr))
    }

    pub fn as_res(&self) -> SourcesResponse {
        SourcesResponse {
            symbol: self.symbol.to_string(),
            proxies: self
                .proxies
                .iter()
                .map(|item| (item.0, item.1.to_string()))
                .collect(),
        }
    }

    pub fn remove(&mut self, proxy_addr: &Addr) -> Result<(), ContractError> {
        match self.proxies.iter().position(|item| item.1 == *proxy_addr) {
            Some(position) => {
                self.proxies.remove(position);
                Ok(())
            }
            None => Err(ContractError::ProxyNotRegistered {}),
        }
    }

    /// Updates the priority of the provided proxy address
    pub fn update_proxy_priority(
        &mut self,
        proxy_addr: &Addr,
        priority: u8,
    ) -> Result<(), ContractError> {
        match self.proxies.iter().position(|item| item.1.eq(proxy_addr)) {
            Some(position) => {
                self.proxies[position] = (priority, proxy_addr.clone());

                Ok(())
            }
            None => Err(ContractError::ProxyNotRegistered {}),
        }
    }
}
