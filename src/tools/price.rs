use rmcp::{
    model::{CallToolResult, Content},
    ErrorData as McpError,
};
use serde::{Deserialize, Serialize};

/// 查询代币价格请求参数
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetTokenPriceRequest {
    /// 代币地址或符号（如 "USDT", "0xdac17f958d2ee523a2206206994597c13d831ec7"）
    pub token: String,

    /// 报价货币（"USD" 或 "ETH"）
    /// 默认为 "USD"
    #[serde(default = "default_quote_currency")]
    pub quote_currency: String,
}

fn default_quote_currency() -> String {
    "USD".to_string()
}

/// 代币价格响应
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TokenPriceResponse {
    /// 代币符号
    pub symbol: String,

    /// 代币名称
    pub name: String,

    /// 代币合约地址
    pub address: String,

    /// 当前价格
    pub price: String,

    /// 报价货币（USD 或 ETH）
    pub quote_currency: String,

    /// 24小时价格变化百分比（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_change_24h: Option<String>,

    /// 24小时交易量（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_24h: Option<String>,

    /// 市值（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<String>,

    /// 数据来源（如 "Uniswap V2", "CoinGecko" 等）
    pub data_source: String,

    /// 数据更新时间戳
    pub timestamp: u64,
}

/// 获取代币在 Uniswap V2 上的价格
///
/// # 参数
/// - `token`: 代币地址或符号
/// - `quote_currency`: 报价货币（USD 或 ETH），默认 USD
///
/// # 返回
/// - 代币价格信息，包括当前价格、24小时变化等
///
/// # 实现说明
/// - 优先从 Uniswap V2 查询实时价格
/// - 对于 USD 报价，需要先获取 ETH/USD 价格，然后计算
/// - 可以缓存价格数据以提高性能
pub async fn get_token_price(request: GetTokenPriceRequest) -> Result<CallToolResult, McpError> {
    // TODO: 实现价格查询逻辑
    // 1. 验证 token 地址或符号
    // 2. 如果是符号，解析为合约地址
    // 3. 从 Uniswap V2 查询 token/WETH 价格
    // 4. 如果报价货币是 USD，获取 ETH/USD 价格并转换
    // 5. 可选：获取 24小时价格变化、交易量等数据
    // 6. 返回格式化的价格信息

    let response = TokenPriceResponse {
        symbol: "UNKNOWN".to_string(),
        name: "Unknown Token".to_string(),
        address: request.token,
        price: "0.0".to_string(),
        quote_currency: request.quote_currency,
        price_change_24h: None,
        volume_24h: None,
        market_cap: None,
        data_source: "Uniswap V2".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_request_deserialization() {
        let json = r#"{"token": "USDT"}"#;
        let request: GetTokenPriceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.token, "USDT");
        assert_eq!(request.quote_currency, "USD");
    }

    #[test]
    fn test_price_request_with_eth_quote() {
        let json = r#"{"token": "USDT", "quote_currency": "ETH"}"#;
        let request: GetTokenPriceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.quote_currency, "ETH");
    }

    #[test]
    fn test_price_response_serialization() {
        let response = TokenPriceResponse {
            symbol: "USDT".to_string(),
            name: "Tether USD".to_string(),
            address: "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
            price: "1.0".to_string(),
            quote_currency: "USD".to_string(),
            price_change_24h: Some("-0.05".to_string()),
            volume_24h: Some("1000000000".to_string()),
            market_cap: None,
            data_source: "Uniswap V2".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("USDT"));
        assert!(json.contains("1.0"));
    }
}
