use ethers::{
    contract::Event,
    providers::{Middleware, PubsubClient},
    types::Address
};
use futures::StreamExt;
use std::sync::Arc;

use crate::{
    bindings::{SwapFilter, UniswapV2Pair},
    dex_price_listener::{DexPairConfig, DexPriceListener, PriceTracker},
    helpers::{compute_price, detect_token_ordering, get_token_symbol, to_f64_normalized},
};

pub struct UniswapV2Listener;

#[async_trait::async_trait]
impl<M> DexPriceListener<M> for UniswapV2Listener
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
        let pair_contract = UniswapV2Pair::new(pair.pair, provider.clone());

        let token0: Address = pair_contract.token_0().call().await?;
        let token1: Address = pair_contract.token_1().call().await?;

        let (token0_is_base, _) = detect_token_ordering(
            token0,
            token1,
            pair.base.id,
            pair.quote.id,
            pair.pair,
        )?;

        let (token0_decimals, token1_decimals) = if token0_is_base {
            (pair.base.decimals, pair.quote.decimals)
        } else {
            (pair.quote.decimals, pair.base.decimals)
        };

        // Subscribe to Swap events
        let swap_events: Event<Arc<M>, M, SwapFilter> = pair_contract.swap_filter();
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
                    let a0_in = match to_f64_normalized(parsed.amount_0_in, token0_decimals) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let a1_in = match to_f64_normalized(parsed.amount_1_in, token1_decimals) {
                        Ok(v) => v,
                        Err(_) => continue,

                    };
                        
                    let a0_out = match to_f64_normalized(parsed.amount_0_out, token0_decimals) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let a1_out = match to_f64_normalized(parsed.amount_1_out, token1_decimals) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                        

                    if let Some(price) =
                        compute_price(a0_in, a1_in, a0_out, a1_out, token0_is_base)
                    {
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
