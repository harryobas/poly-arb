use ethers::{
    contract::Event,
    providers::{Middleware, PubsubClient},
    types::{Address, U256},
};
use futures::StreamExt;
use std::sync::Arc;

use crate::{
    bindings::uniswapv3::{SwapFilter, UniswapV3Pool},
    dex_price_listener::{DexPairConfig, DexPriceListener, PriceTracker},
    helpers::{detect_token_ordering, get_token_symbol, sqrt_price_x96_to_price_f64},
};

pub struct UniswapV3Listener;

#[async_trait::async_trait]
impl<M> DexPriceListener<M> for UniswapV3Listener
where
    M: Middleware + 'static,
    <M as Middleware>::Provider: PubsubClient,
{
    async fn start(
        dex_name: String,
        provider: Arc<M>,
        tracker: Arc<PriceTracker>,
        dex_factory: Address,
        pair: DexPairConfig,
    ) -> anyhow::Result<()> {
        let pool_contract = UniswapV3Pool::new(pair.pair, provider.clone());

        let token0: Address = pool_contract.token_0().call().await?;
        let token1: Address = pool_contract.token_1().call().await?;

        let (token0_is_base, _) = detect_token_ordering(
            token0,
            token1,
            pair.base.id,
            pair.quote.id,
            pair.pair,
        )?;

        let (base_decimals, quote_decimals) = if token0_is_base {
            (pair.base.decimals, pair.quote.decimals)
        } else {
            (pair.quote.decimals, pair.base.decimals)
        };

        // Subscribe to Swap events
        let swap_events: Event<Arc<M>, M, SwapFilter> = pool_contract.swap_filter();
        let mut stream = swap_events.stream().await?;

        tracing::info!(
            "[{}] Listening to swaps for {}/{}",
            dex_name,
            get_token_symbol(pair.base.id, provider.clone()).await?,
            get_token_symbol(pair.quote.id, provider.clone()).await?
        );

        while let Some(event) = stream.next().await {
            match event {
                Ok(parsed) => {
                    let sqrt_price_x96: U256 = parsed.sqrt_price_x96;

                    // Convert sqrtPriceX96 to a human quote/base price
                    let price = match sqrt_price_x96_to_price_f64(
                        sqrt_price_x96,
                        token0_is_base,
                        base_decimals as i32,
                        quote_decimals as i32,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::warn!(
                                "[{}] Failed to compute price from sqrtPriceX96: {:?}",
                                dex_name,
                                e
                            );
                            continue;
                        }
                    };

                    tracker
                        .update(
                            dex_factory,
                            pair.pair,
                            pair.base.id,
                            pair.quote.id,
                            price,
                        )
                        .await?;

                    tracing::info!(
                        "[{}] {:.6} for {}/{}",
                        dex_name,
                        price,
                        get_token_symbol(pair.base.id, provider.clone()).await?,
                        get_token_symbol(pair.quote.id, provider.clone()).await?
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "[{}] Failed to decode Swap event for {:?}: {:?}",
                        dex_name,
                        pair.pair,
                        e
                    );
                }
            }
        }

        Ok(())
    }
}
