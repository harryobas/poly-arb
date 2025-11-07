use ethers::types::{Address, U256};
use std::collections::HashMap;
use tokio::sync::RwLock;

use serde::{Serialize, Deserialize};


#[derive(Clone, Debug)]
pub struct PriceInfo {
    pub base: Address,
    pub quote: Address,
    pub price: f64, // canonical: quote per base
}

/// Unique key: (dex factory address, pair contract address)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub struct DexPairKey {
    pub dex_factory: Address,
    pub pair_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPairConfig {
    pub pair: Address,
    pub base: Token,
    pub quote: Token,
    pub trade_size: U256,

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Token {
    pub id: Address,
    pub decimals: usize 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DexType {
    V2,
    V3,
    QuickSwap
}
#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct BalancerPoolInfo {
    
}


#[derive(Debug, Clone)]
pub struct DexConfig {
    pub name: String,
    pub factory: Address,
    pub pairs: Vec<DexPairConfig>,
    pub dex_type: DexType
}

#[derive(Debug, Clone)]
pub struct ArbOpportunity {
    pub trade_amt: U256,
    pub quote_asset: Address,
    pub base_asset: Address,
    pub buy_dex: Address,
    pub sell_dex: Address,
}


/// Shared, async-friendly PriceTracker (Arc around this struct in main)
pub struct PriceTracker {
    inner: RwLock<HashMap<DexPairKey, PriceInfo>>,
}

impl PriceTracker {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// update canonical price (quote per base) for a dex/pair
    pub async fn update(
        &self, 
        dex_factory: Address, 
        pair_address: Address, 
        base: Address, 
        quote: Address, 
        price: f64
    ) -> anyhow::Result<()> {
        let key = DexPairKey { dex_factory, pair_address };
        let info = PriceInfo { base, quote, price };
        let mut map = self.inner.write().await;
        map.insert(key, info);

        Ok(())
    }

    /// get all price infos for a given token pair (base/quote) across DEXes
    pub async fn get_all_for_pair(
        &self, 
        base: Address, 
        quote: Address
    ) -> anyhow::Result<Vec<(DexPairKey, PriceInfo)>> {
        let map = self.inner.read().await;
        Ok(
            map.iter()
            .filter(|(_, info)| info.base == base && info.quote == quote)
            .map(|(k, v)| (*k, v.clone()))
            .collect()
        )
    }
}