use ethers::contract::abigen;

abigen!(
    IUniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address)
    ]"#
);

abigen! (
    UniswapV3Pool,
    r#"[
        function token0() external view returns (address)
        function token1() external view returns (address)
        event Swap(address indexed sender, address indexed recipient, int256 amount0,int256 amount1,uint160 sqrtPriceX96,uint128 liquidity,int24 tick)

    ]"#
);