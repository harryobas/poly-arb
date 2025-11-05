use ethers::{providers::Middleware, types::{Address, U256}};
use crate::{
    constants::*,
    dex_pool_resolver::{uniswapv2_resolver::UniswapV2Resolver, uniswapv3_resolver::UniswapV3Resolver},
    helpers::make_pair,
    types::{DexConfig, DexPairConfig, DexType, Token},
};
use std::sync::Arc;
use futures::future::join_all;
use anyhow::Result;

pub async fn build_target_configs<M: Middleware + 'static>(
    provider: Arc<M>,
) -> Result<Vec<DexConfig>> {
    // --- Factories  ---
    let quickswap_factory: Address = QUICKSWAP_FACTORY.parse()?;
    let sushiswapv2_factory: Address = SUSHISWAPV2_FACTORY.parse()?;
    let uniswapv3_factory: Address = UNISWAPV3_FACTORY.parse()?;
    let sushiswapv3_factory: Address = SUSHISWAPV3_FACTORY.parse()?;

    // --- Tokens ---
    let weth: Address = WETH.parse()?;
    let wbtc: Address = WBTC.parse()?;
    let dai: Address = DAI.parse()?;
    let usdc: Address = USDC.parse()?;
    let usdt: Address = USDT.parse()?;
    let maticx: Address = MATICX.parse()?;
    let wpol: Address = WPOL.parse()?;

    // --- Token Pairs ---
    let token_pairs = vec![
        (weth, usdc),
        (weth, usdt),
        (weth, dai),
        (wbtc, usdc),
        (wbtc, usdt),
        (wbtc, dai),
        (maticx, usdc),
        (maticx, usdt),
        (maticx, dai),
        (dai, usdc),
        (dai, usdt),
        (usdt, usdc),
        (wpol, usdc),
        (wpol, usdt)
    ]
    .into_iter()
    .map(|(base, quote)| {
        let base = Token { id: base, decimals: 18 };
        let quote_decimals = if quote == usdc || quote == usdt { 6 } else { 18 };
        let quote = Token { id: quote, decimals: quote_decimals };
        (base, quote)
    })
    .collect::<Vec<_>>();

    // --- Trade Sizes ---
    let usdc_trade_size = ethers::utils::parse_units(TRADE_SIZE, 6)?.into();
    let dai_trade_size = ethers::utils::parse_units(TRADE_SIZE, 18)?.into();

    // --- Helper closure for trade size ---
    let select_trade_size = |quote: Address| -> U256 {
        if quote == usdc || quote == usdt {
            usdc_trade_size
        } else {
            dai_trade_size
        }
    };

    // --- Build QuickSwap pairs in parallel ---
    let quick_pairs = join_all(token_pairs.iter().map(|(base, quote)| {
        let provider = provider.clone();
        async move {
            make_pair::<M, UniswapV2Resolver>(
                quickswap_factory,
                base.clone(),
                quote.clone(),
                select_trade_size(quote.id),
                provider.clone(),
            )
            .await
        }
    }))
    .await
    .into_iter()
    .filter_map(Result::ok) // skip failing pairs
    .collect::<Vec<DexPairConfig>>();

    // --- Build SushiSwap pairs in parallel ---
    let sushiv2_pairs = join_all(token_pairs.iter().map(|(base, quote)| {
        let provider = provider.clone();
        async move {
            make_pair::<M, UniswapV2Resolver>(
                sushiswapv2_factory,
                base.clone(),
                quote.clone(),
                select_trade_size(quote.id),
                provider.clone(),
            )
            .await
        }
    }))
    .await
    .into_iter()
    .filter_map(Result::ok)
    .collect::<Vec<DexPairConfig>>();

    let uniswap_pairs = join_all(token_pairs.iter().map(|(base, quote)| {
        let provider = provider.clone();
        async move {
            make_pair::<M, UniswapV3Resolver<3000>>(
                uniswapv3_factory, 
                base.clone(), 
                quote.clone(), 
                select_trade_size(quote.id), 
                provider
            )
            .await
        }
    }))
    .await
    .into_iter()
    .filter_map(Result::ok)
    .collect::<Vec<DexPairConfig>>();

    // --- Assemble into DexConfig structs ---
    let configs = vec![
        DexConfig {
            name: String::from("quickswapv2"),
            factory: quickswap_factory,
            pairs: quick_pairs,
            dex_type: DexType::V2
        },
        DexConfig {
            name: String::from("sushiswapv2"),
            factory: sushiswapv2_factory,
            pairs: sushiv2_pairs,
            dex_type: DexType::V2
        },

        DexConfig {
            name: String::from("uniswapv3"),
            factory: uniswapv3_factory,
            pairs: uniswap_pairs,
            dex_type: DexType::V3
        }
    ];

    Ok(configs)
}
