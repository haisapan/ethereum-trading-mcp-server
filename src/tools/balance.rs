use rmcp::{
    model::{CallToolResult, Content},
    ErrorData as McpError,
};
use serde::{Deserialize, Serialize};

/// 查询余额请求参数
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct GetBalanceRequest {
    /// 钱包地址
    pub address: String,

    /// 可选的 ERC20 代币合约地址
    /// 如果不提供，则查询 ETH 余额
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_address: Option<String>,
}

/// 余额查询响应
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct BalanceResponse {
    /// 钱包地址
    pub address: String,

    /// 余额值（已格式化，带正确小数位）
    pub balance: String,

    /// 原始余额（wei 或最小单位）
    pub balance_raw: String,

    /// 代币符号（如 ETH, USDT, etc.）
    pub symbol: String,

    /// 代币小数位数
    pub decimals: u8,

    /// 代币名称（如果是 ERC20）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_name: Option<String>,

    /// 代币合约地址（如果是 ERC20）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_address: Option<String>,
}

/// 查询 ETH 或 ERC20 代币余额
///
/// # 参数
/// - `address`: 钱包地址
/// - `token_address`: 可选的 ERC20 代币合约地址，如果不提供则查询 ETH 余额
///
/// # 返回
/// - 余额信息，包含格式化后的余额和原始余额
pub async fn get_balance(request: GetBalanceRequest) -> Result<CallToolResult, McpError> {
    // TODO: 实现余额查询逻辑
    // 1. 验证地址格式
    // 2. 如果 token_address 为 None，查询 ETH 余额
    // 3. 如果 token_address 有值，查询 ERC20 余额
    // 4. 格式化余额（考虑小数位）
    // 5. 返回结果

    let response = BalanceResponse {
        address: request.address,
        balance: "0.0".to_string(),
        balance_raw: "0".to_string(),
        symbol: "ETH".to_string(),
        decimals: 18,
        token_name: None,
        token_address: request.token_address,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(CallToolResult::success(vec![Content::text(json)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_request_deserialization() {
        let json = r#"{"address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"}"#;
        let request: GetBalanceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(
            request.address,
            "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"
        );
        assert!(request.token_address.is_none());
    }

    #[test]
    fn test_balance_response_serialization() {
        let response = BalanceResponse {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            balance: "1.5".to_string(),
            balance_raw: "1500000000000000000".to_string(),
            symbol: "ETH".to_string(),
            decimals: 18,
            token_name: None,
            token_address: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("1.5"));
        assert!(json.contains("ETH"));
    }
}
