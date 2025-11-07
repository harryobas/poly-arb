use ethers::contract::abigen;


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