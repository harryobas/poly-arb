use ethers::{
    providers::{Middleware, PubsubClient}, 
    types::{Address, Bytes, H256 as TxHash, U256}
};
use crate::{
    bindings::IERC20, 
    constants::{FACTORY_ROUTER_MAP, TOKEN_SYMBOL_CACHE}, 
    dex_pool_resolver::DexPoolResolver,
    dex_price_listener::{
        DexPriceListener,
        uniswapv2_price_listener::UniswapV2Listener, 
        uniswapv3_price_listener::UniswapV3Listener,
        balancer_price_listener::BalancerListener
     },
    types::{
        ArbOpportunity,
        DexConfig, 
        DexPairConfig, 
        DexPairKey, 
        PriceInfo, 
        PriceTracker, 
        Token,
        DexType
    }
};

use std::sync::Arc;
use tokio::task::JoinHandle;

pub async fn make_pair<M, R>(
    factory: Address,
    base: Token,
    quote: Token,
    trade_size: U256,
    provider: Arc<M>,
) -> anyhow::Result<DexPairConfig>
where 
    M: Middleware + 'static,
    R: DexPoolResolver<M>

{
    let pair = R::resolve_pool(factory, base.id, quote.id, provider.clone()).await?;

    Ok(DexPairConfig {
        pair,
        base,
        quote,
        trade_size,
    
    })
}

pub fn create_arb_calldata(arb_opp: &ArbOpportunity) -> anyhow::Result<Bytes> {
    let tokens = vec![
        ethers::abi::Token::Uint(arb_opp.trade_amt),
        ethers::abi::Token::Address(arb_opp.quote_asset),
        ethers::abi::Token::Address(arb_opp.base_asset),
        ethers::abi::Token::Address(arb_opp.buy_dex),
        ethers::abi::Token::Address(arb_opp.sell_dex)
    ];
    let encoded = ethers::abi::encode(&tokens);
    Ok(Bytes::from(encoded))
    
}

/// Determine whether token0/token1 correspond to base/quote.
/// Returns (is_token0_base, is_token0_quote)
pub fn detect_token_ordering(
    token0: Address,
    token1: Address,
    base: Address,
    quote: Address,
    pair_address: Address,
) -> anyhow::Result<(bool, bool)> {
    let token0_is_base = token0 == base;
    let token1_is_base = token1 == base;
    let token0_is_quote = token0 == quote;
    let token1_is_quote = token1 == quote;

    if !token0_is_base && !token1_is_base {
        anyhow::bail!("Base token not found in pair {:?}", pair_address);
    }
    if !token0_is_quote && !token1_is_quote {
        anyhow::bail!("Quote token not found in pair {:?}", pair_address);
    }

    Ok((token0_is_base, token0_is_quote))
}

pub fn compute_price(
    amount0_in: f64,
    amount1_in: f64,
    amount0_out: f64,
    amount1_out: f64,
    token0_is_base: bool,
) -> Option<f64> {
    // Case 1: base = token0
    if token0_is_base {
        if amount0_in > 0.0 && amount1_out > 0.0 {
            // selling base -> quote per base = quote_out / base_in
            Some(amount1_out / amount0_in)
        } else if amount1_in > 0.0 && amount0_out > 0.0 {
            // buying base -> quote per base = quote_in / base_out
            Some(amount1_in / amount0_out)
        } else {
            None
        }
    } else {
        // Case 2: base = token1 (inverted direction)
        if amount1_in > 0.0 && amount0_out > 0.0 {
            // selling base -> quote per base = quote_out / base_in
            Some(amount0_out / amount1_in)
        } else if amount0_in > 0.0 && amount1_out > 0.0 {
            // buying base -> quote per base = quote_in / base_out
            Some(amount0_in / amount1_out)
        } else {
            None
        }
    }
}

pub fn to_f64_normalized(amount: U256, decimals: usize) -> anyhow::Result<f64> {
    let amt = ethers::utils::format_units(amount, decimals)?
        .parse::<f64>()?;

    Ok(amt)

}

pub fn extract_pair_configs(
    dex_configs: &[DexConfig]
) -> anyhow::Result<Vec<(Address, Vec<DexPairConfig>)>> {

    let mut pairs = vec![];

    dex_configs
        .iter()
        .cloned()
        .for_each(|p| pairs.push((p.factory, p.pairs)));

    Ok(pairs)

}


pub fn compute_spread(
    prices: &[(DexPairKey, PriceInfo)]
) -> anyhow::Result<(f64, DexPairKey, DexPairKey)> {
    if prices.len() < 2 {
        return Err(anyhow::anyhow!("Need at least 2 price sources to compute spread"));
    }

    // Track global min (best buy) and max (best sell)
    let mut min: Option<(DexPairKey, PriceInfo)> = None;
    let mut max: Option<(DexPairKey, PriceInfo)> = None;

    for (k, v) in prices {
        if v.price <= 0.0 {
            continue;
        }

        if min.as_ref().map_or(true, |(_, p)| v.price < p.price) {
            min = Some((*k, v.clone()));
        }
        if max.as_ref().map_or(true, |(_, p)| v.price > p.price) {
            max = Some((*k, v.clone()));
        }
    }

    let (buy_k, buy_info) = min
        .ok_or_else(|| anyhow::anyhow!("No valid min price"))?;

    let (sell_k, sell_info) = max
        .ok_or_else(|| anyhow::anyhow!("No valid max price"))?;

    if buy_info.price <= 0.0 {
        return Err(anyhow::anyhow!("Invalid buy price"));
    }

    let spread = (sell_info.price / buy_info.price) - 1.0;

    Ok((spread, buy_k, sell_k))
}

pub async fn handle_arb_opportunity<M: Middleware + 'static>(
    buy_k: DexPairKey, 
    sell_k: DexPairKey, 
    spread: f64,
    pair: &DexPairConfig,
    provider: Arc<M>,
) -> anyhow::Result<()> {
    tracing::info!(
        "Detected arb between {:?} and {:?}, spread {:.3}%",
        buy_k.dex_factory, sell_k.dex_factory, spread * 100.0
    );

    let arb_opp = ArbOpportunity {
        trade_amt: pair.trade_size,
        base_asset: pair.base.id,
        quote_asset: pair.quote.id,

        buy_dex: *FACTORY_ROUTER_MAP
            .get(&buy_k.dex_factory)
            .ok_or(anyhow::anyhow!("Buy DEX router not found"))?,

        sell_dex: *FACTORY_ROUTER_MAP
            .get(&sell_k.dex_factory)
            .ok_or(anyhow::anyhow!("Sell DEX router not found"))?,
    };

    let arb_data = create_arb_calldata(&arb_opp)?;
    tracing::debug!("Prepared calldata for arb execution ({} bytes)", arb_data.len());

    match execute_arb_tx(arb_data, provider.clone()).await {
        Ok(tx_hash) => tracing::info!("✅ Executed arb tx: {:?}", tx_hash),
        Err(e) => tracing::warn!("❌ Failed to execute arb tx: {:?}", e),
    }

    Ok(())
}

pub async fn get_token_symbol<M: Middleware + 'static>(
    token: Address,
    provider: Arc<M>,
) -> anyhow::Result<String> {
    if let Some(sym) = TOKEN_SYMBOL_CACHE.get(&token) {
        return Ok(sym.value().clone());
    }

    let contract = IERC20::new(token, provider.clone());
    let result = contract.symbol().call().await;

    let symbol = match result {
        Ok(sym) if !sym.is_empty() => sym,
        _ => anyhow::bail!("symbol for token {:?} not found or unreadable", token),
    };

    
    TOKEN_SYMBOL_CACHE.insert(token, symbol.clone());
    Ok(symbol)
}

 async fn spawn_listener<M, L>(
    dex: DexConfig,
    pair: DexPairConfig,
    provider: Arc<M>,
    tracker: Arc<PriceTracker>,
) -> anyhow::Result<()>
where
    M: Middleware + 'static,
    L: DexPriceListener<M>,
    <M as Middleware>::Provider: PubsubClient
{
   L::start(dex.name, provider.clone(), tracker.clone(), dex.factory, pair).await?;

    Ok(())

}

pub async fn start_all_listeners<M>(
    dex_configs: Vec<DexConfig>,
    provider: Arc<M>,
    tracker: Arc<PriceTracker>,
) -> anyhow::Result<()>
where
    M: Middleware + 'static,
    <M as Middleware>::Provider: PubsubClient,
{
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for dex in dex_configs {
        for pair in dex.pairs.clone() {
            let provider = provider.clone();
            let tracker = tracker.clone();
            let dex_clone = dex.clone();

            let handle = match dex.dex_type {
                DexType::V2 => {
                    tokio::spawn(async move {
                        let _ = spawn_listener::<_, UniswapV2Listener>(
                            dex_clone, pair, provider, tracker
                        ).await;
                    })
                }
                DexType::V3 => {
                    tokio::spawn(async move {
                        let _ = spawn_listener::<_, UniswapV3Listener>(
                            dex_clone, pair, provider, tracker
                        ).await;
                    })
                }
                DexType::Balancer => {
                    tokio::spawn(async move {
                        let _ = spawn_listener::<_, BalancerListener>(
                            dex_clone, pair, provider, tracker
                        ).await;
                    })
                }
            };

            handles.push(handle);
        }
    }

    tracing::info!("✅ Spawned {} DEX listeners", handles.len());

    // Optional: keep them alive forever
    futures::future::join_all(handles).await;

    Ok(())
}


pub async fn execute_arb_tx<M: Middleware + 'static>(
    arb_data: Bytes, 
    provider: Arc<M>
) -> anyhow::Result<TxHash>{
    let executor = ArbExecutor::new(ARBITRAGE_CONTRACT, provider.clone());
    let tx = executor.execute(arb_data).gas(8_000_000u64).send().await?;
    let receipt = tx.await?.ok_or_else(|| anyhow::anyhow!("No receipt"))?;
    Ok(receipt.transaction_hash)
    
}

