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

abigen!(
    IERC20,
    r#"[
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
    ]"#,
);

abigen!(
    IUniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address)
    ]"#
);

abigen!(
    AlgebraPool,
    r#"[
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 price,uint128 liquidity,int24 tick)
        function token0() external view returns (address)
        function token1() external view returns (address)
    ]"#
);

abigen!(
    AlgebraFactory,
    r#"[
        function poolByPair(address,address) external view returns (address)
        function createPool(address,address) external returns (address)
    ]"#
);

