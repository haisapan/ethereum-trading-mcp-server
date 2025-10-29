use rmcp::{
    model::{CallToolResult, Content},
    ErrorData as McpError,
};
use serde::{Deserialize, Serialize};

/// 代币交换请求参数
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SwapTokensRequest {
    /// 源代币地址或符号
    pub from_token: String,

    /// 目标代币地址或符号
    pub to_token: String,

    /// 交易数量（以源代币为单位）
    pub amount: String,

    /// 滑点容差（基点，例如 50 = 0.5%）
    /// 默认为 50
    #[serde(default = "default_slippage")]
    pub slippage_bps: u32,

    /// 钱包地址（用于 Gas 估算，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
}

fn default_slippage() -> u32 {
    50 // 0.5%
}

/// 代币交换响应
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SwapResponse {
    /// 源代币信息
    pub from_token: TokenInfo,

    /// 目标代币信息
    pub to_token: TokenInfo,

    /// 输入数量
    pub amount_in: String,

    /// 输入数量（原始值）
    pub amount_in_raw: String,

    /// 预估输出数量
    pub amount_out: String,

    /// 预估输出数量（原始值）
    pub amount_out_raw: String,

    /// 最小输出数量（考虑滑点）
    pub amount_out_min: String,

    /// 最小输出数量（原始值）
    pub amount_out_min_raw: String,

    /// 交易路径（代币地址数组）
    pub path: Vec<String>,

    /// 价格影响百分比
    pub price_impact: String,

    /// Gas 估算
    pub gas_estimate: GasEstimate,

    /// 交换率（1 from_token = ? to_token）
    pub exchange_rate: String,

    /// 反向交换率（1 to_token = ? from_token）
    pub inverse_rate: String,

    /// 使用的 DEX 协议
    pub protocol: String,

    /// 交易模拟是否成功
    pub simulation_success: bool,

    /// 警告信息（如果有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

/// 代币信息
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TokenInfo {
    /// 代币符号
    pub symbol: String,

    /// 代币名称
    pub name: String,

    /// 代币合约地址
    pub address: String,

    /// 小数位数
    pub decimals: u8,
}

/// Gas 估算信息
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GasEstimate {
    /// Gas 限制
    pub gas_limit: String,

    /// Gas 价格（Gwei）
    pub gas_price: String,

    /// 总 Gas 费用（ETH）
    pub total_gas_fee_eth: String,

    /// 总 Gas 费用（USD，可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_gas_fee_usd: Option<String>,
}

/// 在 Uniswap V2/V3 上执行代币交换模拟
///
/// # 参数
/// - `from_token`: 源代币地址或符号
/// - `to_token`: 目标代币地址或符号
/// - `amount`: 交易数量（以源代币为单位）
/// - `slippage_bps`: 滑点容差（基点，50 = 0.5%）
/// - `wallet_address`: 钱包地址（用于 Gas 估算）
///
/// # 返回
/// - 交换模拟结果，包括预估输出、价格影响、Gas 费用等
///
/// # 实现说明
/// - 构造真实的 Uniswap 交易
/// - 使用 `eth_call` 进行链上模拟（不实际执行）
/// - 计算价格影响和滑点
/// - 估算 Gas 费用
pub async fn swap_tokens(request: SwapTokensRequest) -> Result<CallToolResult, McpError> {
    // TODO: 实现代币交换模拟逻辑
    // 1. 验证代币地址或符号
    // 2. 解析代币信息（符号、小数位等）
    // 3. 构造 Uniswap V2/V3 交换交易
    // 4. 使用 eth_call 模拟交易执行
    // 5. 计算输出数量、价格影响
    // 6. 计算考虑滑点的最小输出
    // 7. 估算 Gas 费用
    // 8. 检查交易可行性并生成警告
    // 9. 返回完整的模拟结果

    let response = SwapResponse {
        from_token: TokenInfo {
            symbol: "UNKNOWN".to_string(),
            name: "Unknown Token".to_string(),
            address: request.from_token,
            decimals: 18,
        },
        to_token: TokenInfo {
            symbol: "UNKNOWN".to_string(),
            name: "Unknown Token".to_string(),
            address: request.to_token,
            decimals: 18,
        },
        amount_in: request.amount.clone(),
        amount_in_raw: "0".to_string(),
        amount_out: "0.0".to_string(),
        amount_out_raw: "0".to_string(),
        amount_out_min: "0.0".to_string(),
        amount_out_min_raw: "0".to_string(),
        path: vec![],
        price_impact: "0.0".to_string(),
        gas_estimate: GasEstimate {
            gas_limit: "0".to_string(),
            gas_price: "0".to_string(),
            total_gas_fee_eth: "0.0".to_string(),
            total_gas_fee_usd: None,
        },
        exchange_rate: "0.0".to_string(),
        inverse_rate: "0.0".to_string(),
        protocol: "Uniswap V2".to_string(),
        simulation_success: false,
        warnings: Some(vec!["功能未实现".to_string()]),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_request_deserialization() {
        let json = r#"{
            "from_token": "USDT",
            "to_token": "USDC",
            "amount": "100.0"
        }"#;
        let request: SwapTokensRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.from_token, "USDT");
        assert_eq!(request.to_token, "USDC");
        assert_eq!(request.amount, "100.0");
        assert_eq!(request.slippage_bps, 50); // 默认值
    }

    #[test]
    fn test_swap_request_with_custom_slippage() {
        let json = r#"{
            "from_token": "USDT",
            "to_token": "USDC",
            "amount": "100.0",
            "slippage_bps": 100
        }"#;
        let request: SwapTokensRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.slippage_bps, 100);
    }

    #[test]
    fn test_swap_response_serialization() {
        let response = SwapResponse {
            from_token: TokenInfo {
                symbol: "USDT".to_string(),
                name: "Tether USD".to_string(),
                address: "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
                decimals: 6,
            },
            to_token: TokenInfo {
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
                decimals: 6,
            },
            amount_in: "100.0".to_string(),
            amount_in_raw: "100000000".to_string(),
            amount_out: "99.5".to_string(),
            amount_out_raw: "99500000".to_string(),
            amount_out_min: "99.0".to_string(),
            amount_out_min_raw: "99000000".to_string(),
            path: vec![
                "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
                "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
            ],
            price_impact: "0.5".to_string(),
            gas_estimate: GasEstimate {
                gas_limit: "150000".to_string(),
                gas_price: "30".to_string(),
                total_gas_fee_eth: "0.0045".to_string(),
                total_gas_fee_usd: Some("15.0".to_string()),
            },
            exchange_rate: "0.995".to_string(),
            inverse_rate: "1.005".to_string(),
            protocol: "Uniswap V2".to_string(),
            simulation_success: true,
            warnings: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("USDT"));
        assert!(json.contains("USDC"));
        assert!(json.contains("99.5"));
    }
}
