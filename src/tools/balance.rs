use crate::{
    config::Config,
    erc20::{format_units, Erc20Client},
    eth_client::EthClient,
    logging::info,
    token_registry::TokenRegistry,
    types::TokenInfo,
};
use ethers::prelude::*;
use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, ErrorData as McpError,
};
use std::sync::Arc;

/// GetBalance 工具的参数
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetBalanceArgs {
    /// 钱包地址(必需)
    pub address: String,
    /// ERC20 代币地址或符号(可选,不填则查询 ETH 余额)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_address: Option<String>,
}

/// GetBalance 工具的返回结果
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BalanceResult {
    pub address: String,
    pub token: TokenInfo,
    pub balance: String,
    pub decimals: u8,
    pub formatted_balance: String,
}

/// 获取以太坊地址余额(支持 ETH 和 ERC20)
#[tool(description = "获取以太坊地址余额(支持 ETH 和 ERC20 代币)")]
pub fn get_balance(
    config: &Arc<Config>,
    eth_client: &Arc<EthClient>,
    erc20_client: &Arc<Erc20Client>,
    token_registry: &Arc<TokenRegistry>,
    Parameters(args): Parameters<GetBalanceArgs>,
) -> Result<CallToolResult, McpError> {
    info!("收到 get_balance 请求");

    let wallet_address = &args.address;
    info!(address = %wallet_address, "查询地址余额");

    // 测试模式
    if config.server.test_mode {
        let token = if args.token_address.is_some() {
            TokenInfo {
                symbol: "TEST".to_string(),
                name: "Test Token".to_string(),
                address: args.token_address.clone().unwrap_or_default(),
                decimals: 18,
            }
        } else {
            TokenInfo::eth()
        };

        let result = BalanceResult {
            address: wallet_address.clone(),
            token,
            balance: "100000000000000000000".to_string(), // 100 in wei
            decimals: 18,
            formatted_balance: "100".to_string(),
        };

        let json_str = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        return Ok(CallToolResult::success(vec![Content::text(json_str)]));
    }

    // 真实模式:需要检查客户端可用性
    if !eth_client.is_available() {
        return Err(McpError::internal_error(
            "Ethereum 客户端不可用,请检查 RPC 配置",
            None,
        ));
    }

    // 解析钱包地址
    let wallet_addr: Address = wallet_address.parse().map_err(|_| {
        McpError::invalid_params(format!("无效的地址: {}", wallet_address), None)
    })?;

    // 查询余额
    let (token_info, balance, decimals) = if let Some(ref token_address) = args.token_address {
        // 查询 ERC20 余额
        let mut token_info = token_registry
            .resolve(token_address)
            .ok_or_else(|| {
                McpError::invalid_params(
                    format!("未知的代币: {}", token_address),
                    None,
                )
            })?;

        let token_addr: Address = token_info.address.parse().map_err(|_| {
            McpError::internal_error("无效的代币地址".to_string(), None)
        })?;

        // 🔍 动态查询未知代币信息
        if token_info.symbol == "UNKNOWN" && erc20_client.is_available() {
            let erc20_client_clone = erc20_client.clone();
            let real_info = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    erc20_client_clone.token_info(token_addr).await
                })
            })
            .map_err(|e| McpError::internal_error(format!("查询代币信息失败: {}", e), None))?;

            // 缓存到注册表
            token_registry.register(real_info.symbol.clone(), real_info.clone());
            token_info = real_info;
        }

        let erc20_client = erc20_client.clone();
        let decimals = token_info.decimals;

        let balance = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                erc20_client.balance_of(token_addr, wallet_addr).await
            })
        })
        .map_err(|e| McpError::internal_error(format!("查询 ERC20 余额失败: {}", e), None))?;

        (token_info, balance, decimals)
    } else {
        // 查询 ETH 余额
        let eth_client = eth_client.clone();
        let addr_str = wallet_address.clone();

        let balance_wei = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                eth_client.get_balance(&addr_str).await
            })
        })
        .map_err(|e| McpError::internal_error(format!("查询 ETH 余额失败: {}", e), None))?;

        (TokenInfo::eth(), balance_wei, 18)
    };

    // 格式化余额
    let formatted_balance = format_units(balance, decimals);

    let result = BalanceResult {
        address: wallet_address.clone(),
        token: token_info,
        balance: balance.to_string(),
        decimals,
        formatted_balance,
    };

    let json_str = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    info!("成功返回余额");

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_result_serialization() {
        let result = BalanceResult {
            address: "0x123".to_string(),
            token: TokenInfo::eth(),
            balance: "100000000000000000000".to_string(),
            decimals: 18,
            formatted_balance: "100".to_string(),
        };

        let json = serde_json::to_string(&result).expect("应该能序列化");
        assert!(json.contains("100"));
        assert!(json.contains("ETH"));
        assert!(json.contains("0x123"));
    }

    #[test]
    fn test_get_balance_args_deserialization() {
        // 测试带地址和代币的情况
        let json = r#"{"address":"0x123","token_address":"USDC"}"#;
        let args: GetBalanceArgs = serde_json::from_str(json).expect("应该能反序列化");
        assert_eq!(args.address, "0x123");
        assert_eq!(args.token_address, Some("USDC".to_string()));

        // 测试只有地址的情况
        let json = r#"{"address":"0x123"}"#;
        let args: GetBalanceArgs = serde_json::from_str(json).expect("应该能反序列化");
        assert_eq!(args.address, "0x123");
        assert_eq!(args.token_address, None);
    }
}
