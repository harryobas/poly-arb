use std::collections::HashMap;
use ethers::{
    signers::{LocalWallet, Signer}, 
    types::Address,
    providers::{Provider, Ws},
    middleware::SignerMiddleware
};
use dashmap::DashMap;

use crate::helpers::{load_private_key, load_rpc_url};

use once_cell::sync::Lazy;
use std::sync::Arc;

pub const WETH: &str = "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619";
pub const WBTC: &str = "0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6";
pub const USDC: &str = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359";

pub const USDT: &str = "0xc2132D05D31c914a87C6611C10748AEb04B58e8F";                    
pub const DAI: &str = "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063";
pub const MATICX: &str = "0xfa68FB4628DFF1028CFEc22b4162FCcd0d45efb6";
pub const WPOL: &str = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270";

pub const TRADE_SIZE: f64 = 15000.0;
pub const SPREAD_THRESHOLD: f64 = 0.025;
pub const SLIPPAGE_BPS: u64 = 30;

pub const QUICKSWAP_FACTORY: &str = "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32";
pub const SUSHISWAPV2_FACTORY: &str = "0xc35dadb65012ec5796536bd9864ed8773abc74c4";
pub const SUSHISWAPV3_FACTORY: &str = "0x917933899c6a5F8E37F31E19f92CdBFF7e8FF0e2";
pub const UNISWAPV3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";

pub const QUICKSWAP_ROUTER: &str = "0xa5E0829CaCED8fFDD4De3c43696c57F7D7A678ff";
pub const SUSHISWAPV2_ROUTER: &str = "0x1b02da8cb0d097eb8d57a175b88c7d8b47997506";
pub const UNISWAPV3_ROUTER: &str = "";
pub const SUSHISWAPV3_ROUTER: &str = "";


pub const FLASH_ARBITRAGEUR: &str = "";

pub const CHAIN_ID: u64 = 137;

pub static  FACTORY_ROUTER_MAP: Lazy<HashMap<Address, Address>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert(
        SUSHISWAPV2_FACTORY.parse::<Address>().unwrap(), 
        SUSHISWAPV2_ROUTER.parse::<Address>().unwrap()
     );

    map.insert(
        QUICKSWAP_FACTORY.parse::<Address>().unwrap(),
         QUICKSWAP_ROUTER.parse::<Address>().unwrap()
        );

    map.insert(
        UNISWAPV3_FACTORY.parse::<Address>().unwrap(), 
        UNISWAPV3_ROUTER.parse::<Address>().unwrap()
    );

    map.insert(
        SUSHISWAPV3_FACTORY.parse::<Address>().unwrap(), 
        SUSHISWAPV3_ROUTER.parse::<Address>().unwrap()
    );

    map

});

pub static TOKEN_SYMBOL_CACHE: Lazy<DashMap<Address, String>> = Lazy::new(|| {DashMap::new()});

pub static PRIVATE_KEY: Lazy<String> = Lazy::new(|| {
    load_private_key()
});

pub static RPC_URL: Lazy<String> = Lazy::new(|| {
    load_rpc_url()
});

pub static WALLET: Lazy<LocalWallet> = Lazy::new(|| {
    PRIVATE_KEY.parse::<LocalWallet>().unwrap().with_chain_id(CHAIN_ID)
});

pub static PROVIDER: Lazy<Arc<SignerMiddleware<Provider<Ws>, LocalWallet>>> = Lazy::new(|| {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let ws = Ws::connect(&*RPC_URL).await.unwrap();
        let provider = Provider::new(ws);
        let signer = WALLET.clone();
        Arc::new(SignerMiddleware::new(provider, signer))
    })
});