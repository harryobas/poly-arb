use std::sync::Arc;

use ethers::{providers::{Middleware, PubsubClient}, types::Address};

use crate::types::{DexPairConfig, PriceTracker};


pub mod uniswapv2_price_listener;
pub mod uniswapv3_price_listener;
pub mod quickswapv3_price_listener;


#[async_trait::async_trait]
 pub trait DexPriceListener<M>: Send + Sync 
where
    M: Middleware,
    <M as Middleware>::Provider: PubsubClient
{
    async fn start(
        dex_name: String,
        provider: Arc<M>,
        tracker: Arc<PriceTracker>,
        dex_factory: Address,
        pair: DexPairConfig

    ) -> anyhow::Result<()>;
    
}