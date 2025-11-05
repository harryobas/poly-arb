
use tokio::sync::broadcast;
use ethers::types::H256;
use std::sync::Arc;
use ethers::providers::Middleware;

use crate::{
    helpers::{compute_spread, handle_arb_opportunity, get_token_symbol},
    types::{DexPairConfig, PriceTracker},
};

/// ArbitrageWorker
/// - Listens to new block broadcasts
/// - Fetches latest prices for a given pair across all DEXes
/// - Computes spreads
/// - Executes profitable opportunities (> 0.2%)
pub struct ArbWorker<M> {
    receiver: broadcast::Receiver<H256>,
    tracker: Arc<PriceTracker>,
    pair: DexPairConfig,
    provider: Arc<M>,
}

impl<M: Middleware + 'static> ArbWorker<M> {
    pub fn new(
        receiver: broadcast::Receiver<H256>,
        tracker: Arc<PriceTracker>,
        pair: DexPairConfig,
        provider: Arc<M>,
    ) -> Self {
        Self {
            receiver,
            tracker,
            pair,
            provider,
        }
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        tracing::info!(
            "📡 Arbitrage worker started for pair {:?}/{:?}",
            self.pair.base,
            self.pair.quote
        );

        while let Ok(block_hash) = self.receiver.recv().await {
            tracing::debug!("🔹 New block received: {:?}", block_hash);

            // get all known DEX prices for the pair
            let prices = self
                .tracker
                .get_all_for_pair(self.pair.base.id, self.pair.quote.id)
                .await?;

            if prices.len() < 2 {
                tracing::debug!("Not enough DEX prices available for pair");
                continue;
            }

            // compute spread between best buy/sell
            match compute_spread(&prices) {
                Ok((spread, buy_key, sell_key)) => {
                    if spread >= 0.002 {
                        tracing::info!(
                            "💰 Profitable spread detected: {:.3}% between {:?} and {:?}",
                            spread * 100.0,
                            buy_key.dex_factory,
                            sell_key.dex_factory
                        );

                        if let Err(e) = handle_arb_opportunity(
                            buy_key,
                            sell_key,
                            spread,
                            &self.pair,
                            self.provider.clone(),
                        )
                        .await
                        {
                            tracing::warn!("Failed to handle arb opportunity: {:?}", e);
                        }
                    } else {
                        tracing::debug!(
                            "Spread {:.3}% below threshold for pair {:?}/{:?}",
                            spread * 100.0,
                            get_token_symbol(self.pair.base.id, self.provider.clone()).await,
                            get_token_symbol(self.pair.base.id, self.provider.clone()).await,
                        );
                    }
                }
                Err(e) => tracing::warn!("Failed to compute spread: {:?}", e),
            }
        }

        Ok(())
    }
}
