use crate::bindings::quickswapv3::AlgebraFactory;

use super::{DexPoolResolver, Address, Middleware, Arc};

pub struct QuickSwapV3Resolver;

#[async_trait::async_trait]
impl<M: Middleware + 'static> DexPoolResolver<M> for QuickSwapV3Resolver {
    async fn resolve_pool(
        factory: Address,
        base: Address,
        quote: Address,
        provider: Arc<M>,
    ) -> anyhow::Result<Address> {
        let factory_contract = AlgebraFactory::new(factory, provider.clone());
        let pool = factory_contract.pool_by_pair(base, quote).call().await?;
        if pool == Address::zero() {
            anyhow::bail!("pair does not exist in QuickswapV3 factory");
        }
        Ok(pool)
    }
}
