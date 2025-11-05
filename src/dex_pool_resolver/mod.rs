use std::sync::Arc;

use ethers::{providers::Middleware, types::Address};

pub mod uniswapv2_resolver;
pub mod uniswapv3_resolver;
pub mod quickswapv3_resolver;

#[async_trait::async_trait]
pub trait DexPoolResolver<M: Middleware + 'static>: Send + Sync {
    async fn resolve_pool(
        factory: Address,
        base: Address,
        quote: Address,
        provider: Arc<M> 
    ) -> anyhow::Result<Address>;


}