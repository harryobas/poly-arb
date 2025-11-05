
use crate::bindings::IUniswapV2Factory;
use super::{DexPoolResolver, Middleware, Address, Arc};
  

pub struct UniswapV2Resolver;

#[async_trait::async_trait]
impl<M: Middleware + 'static> DexPoolResolver<M> for UniswapV2Resolver {
    async fn resolve_pool(
        factory: Address,
        base: Address,
        quote: Address,
        provider: Arc<M>,
    ) -> anyhow::Result<Address> {
        let factory_contract = IUniswapV2Factory::new(factory, provider);
        let pair = factory_contract.get_pair(base, quote).call().await?;
        if pair == Address::zero() {
            anyhow::bail!("pair does not exist in UniswapV2 factory");
        }
        Ok(pair)
    }
}
