use std::sync::Arc;
use ethers::{providers::Middleware, types::H256};
use futures::StreamExt;
use tokio::sync::broadcast;

pub struct BlockWatcher<M> {
    provider: Arc<M>,
    sender: broadcast::Sender<H256>,
}

impl<M: Middleware + 'static> BlockWatcher<M> {
    pub fn new(provider: Arc<M>, sender: broadcast::Sender<H256>) -> Self {
        Self { provider, sender }
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let mut stream = self.provider.watch_blocks().await?;
        tracing::info!("🟢 Block watcher started");

        while let Some(block_hash) = stream.next().await {
            if let Err(_) = self.sender.send(block_hash) {
                tracing::warn!("⚠️ No active workers; block watcher idle");
            }
        }

        Ok(())
    }
}
