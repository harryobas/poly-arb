use crate::bindings::uniswapv3::IUniswapV3Factory;

use super::{DexPoolResolver, Address, Middleware, Arc};

pub struct UniswapV3Resolver<const FEE: u32>;

#[async_trait::async_trait]
impl<M: Middleware + 'static, const FEE: u32> DexPoolResolver<M> for UniswapV3Resolver<FEE> {
    async fn resolve_pool(
        factory: Address,
        base: Address,
        quote: Address,
        provider: Arc<M>,
    ) -> anyhow::Result<Address> {
        let factory_contract = IUniswapV3Factory::new(factory, provider);
        let pool = factory_contract.get_pool(base, quote, FEE).call().await?;
        if pool == Address::zero() {
            anyhow::bail!("pool does not exist in UniswapV3 factory (fee {})", FEE);
        }
        Ok(pool)
    }
}
