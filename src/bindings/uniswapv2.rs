use ethers::contract::abigen;


abigen!(
    IUniswapV2Factory,
    r#"[
        function getPair(address tokenA, address tokenB) external view returns (address pair)
    ]"#
);

abigen!(
    UniswapV2Pair,
    r#"[
        function token0() external view returns (address)
        function token1() external view returns (address)
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        event Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
    ]"#
);