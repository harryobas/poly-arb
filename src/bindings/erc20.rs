use ethers::contract::abigen;


abigen!(
    IERC20,
    r#"[
        function name() external view returns (string)
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
    ]"#
);