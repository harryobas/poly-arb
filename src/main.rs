
mod config;
mod constants;
mod types;
mod block_watcher;
mod bindings;
mod arb_worker;
mod helpers;
mod dex_price_listener;
mod dex_pool_resolver;

use crate::{
    arb_worker::ArbWorker, 
    block_watcher::BlockWatcher, 
    constants::PROVIDER,
    types::PriceTracker
};

use tokio::sync::broadcast;
use std::sync::Arc;
use ethers::types::H256;

use dotenv::dotenv;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    // Shared state
    let provider = PROVIDER.clone();
    let tracker = Arc::new(PriceTracker::new());

    // 1 Create broadcast channel for block hashes
    let (tx, _rx) = broadcast::channel::<H256>(16);

    // 2 Start BlockWatcher
    let block_watcher = BlockWatcher::new(provider.clone(), tx.clone());
    tokio::spawn(async move {
        if let Err(e) = block_watcher.start().await {
            tracing::error!("Block watcher failed: {:?}", e);
        }
    });

    // 3 Load target configs
    let dex_configs = config::build_target_configs(provider.clone()).await?;
    let pair_configs = helpers::extract_pair_configs(&dex_configs)?;

    helpers::start_all_listeners(dex_configs.clone(), provider.clone(), tracker.clone()).await?;

    // 5 Start Arbitrage Workers (block-triggered)
    for (_factory, pairs) in pair_configs.clone() {
        for pair in pairs {
            let tracker = tracker.clone();
            let rx = tx.subscribe(); // every worker gets its own receiver
            let pair = pair.clone();
            let provider = provider.clone();

            tokio::spawn(async move {
                let worker = ArbWorker::new(
                    rx, 
                    tracker, 
                    pair,
                    provider
                );
                if let Err(e) = worker.start().await {
                    tracing::error!("Arbitrage worker failed for : {:?}", e);
                }
            });
        }
    }

    tracing::info!("🚀 System initialized: block watcher, listeners, and workers running");

    // Prevent main from exiting
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down gracefully...");
    Ok(())
}
