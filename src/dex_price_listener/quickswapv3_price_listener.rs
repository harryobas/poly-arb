use ethers::{
    contract::Event,
    providers::{Middleware, PubsubClient},
    types::{Address, U256},
};
use futures::StreamExt;
use std::sync::Arc;

use crate::{
    dex_price_listener::{DexPairConfig, DexPriceListener, PriceTracker},
    helpers::{
        detect_token_ordering,
        get_token_symbol, 
        sqrt_price_x96_to_price_f64
    },
    bindings::quickswapv3::{AlgebraPool, SwapFilter}
};

/// Algebra-based (QuickSwap V3) listener
pub struct QuickSwapV3Listener;

#[async_trait::async_trait]
impl<M> DexPriceListener<M> for QuickSwapV3Listener
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
        // --- Bind to the Algebra pool (QuickSwap V3 pool) ---
        let pool = AlgebraPool::new(pair.pair, provider.clone());

        let token0: Address = pool.token_0().call().await?;
        let token1: Address = pool.token_1().call().await?;

        let (token0_is_base, _) =
            detect_token_ordering(token0, token1, pair.base.id, pair.quote.id, pair.pair)?;

        // Determine decimals for normalization
        let (token0_decimals, token1_decimals) = if token0_is_base {
            (pair.base.decimals, pair.quote.decimals)
        } else {
            (pair.quote.decimals, pair.base.decimals)
        };

        // Subscribe to Algebra-style Swap events
        let swap_events: Event<Arc<M>, M, SwapFilter> = pool.swap_filter();
        let mut stream = swap_events.stream().await?;

        tracing::info!(
            "[{}] Listening to Algebra swaps for {}/{}",
            dex_name,
            get_token_symbol(pair.base.id, provider.clone()).await?,
            get_token_symbol(pair.quote.id, provider.clone()).await?
        );

        // --- Main event loop ---
        while let Some(event) = stream.next().await {
            match event {
                Ok(parsed) => {
                    // Normalize amounts
                    //let amount0 = to_f64_normalized(parsed.amount_0, token0_decimals)?;
                    //let amount1 = to_f64_normalized(parsed.amount_1, token1_decimals)?;

                    // Compute price from `price` (already full Q64.96)
                    let price_f = sqrt_price_x96_to_price_f64(
                        parsed.price,
                        token0_is_base,
                        token0_decimals as i32,
                        token1_decimals as i32,
                    )?;

                    // Update the global tracker
                    tracker
                        .update(
                            dex_factory,
                            pair.pair,
                            pair.base.id,
                            pair.quote.id,
                            price_f,
                        )
                        .await?;

                    tracing::info!(
                        "[{}] {:.6} for {}/{}",
                        dex_name,
                        price_f,
                        get_token_symbol(pair.base.id, provider.clone()).await?,
                        get_token_symbol(pair.quote.id, provider.clone()).await?
                    );
                }
                Err(e) => tracing::warn!(
                    "[{}] Failed to decode Algebra Swap for {:?}: {:?}",
                    dex_name,
                    pair.pair,
                    e
                ),
            }
        }

        Ok(())
    }
}

