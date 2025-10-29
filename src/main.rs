mod config;
mod erc20;
mod eth_client;
mod logging;
mod token_registry;
mod tools;
mod types;
mod uniswap;

use config::Config;
use erc20::Erc20Client;
use eth_client::EthClient;
use ethers::prelude::*;
use logging::info;
use token_registry::TokenRegistry;
use tools::{
    balance::{get_balance, GetBalanceArgs},
    price::{get_token_price, GetTokenPriceArgs},
    swap::{swap_tokens, SwapTokensArgs},
};
use uniswap::UniswapV2Client;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    ErrorData as McpError,
    ServerHandler,
    ServiceExt,
};
use std::sync::Arc;

/// Ethereum Trading MCP Server
/// 提供以太坊交易相关的工具
#[derive(Clone)]
struct EthereumTradingServer {
    config: Arc<Config>,
    eth_client: Arc<EthClient>,
    erc20_client: Arc<Erc20Client>,
    uniswap_client: Arc<UniswapV2Client>,
    token_registry: Arc<TokenRegistry>,
    tool_router: ToolRouter<Self>,
}

#[rmcp::tool_router]
impl EthereumTradingServer {
    fn new(config: Config, eth_client: EthClient, provider: Option<Arc<Provider<Http>>>) -> Self {
        let erc20_client = Erc20Client::new(provider.clone());
        let uniswap_client = UniswapV2Client::new(provider);
        let token_registry = TokenRegistry::new();

        Self {
            config: Arc::new(config),
            eth_client: Arc::new(eth_client),
            erc20_client: Arc::new(erc20_client),
            uniswap_client: Arc::new(uniswap_client),
            token_registry: Arc::new(token_registry),
            tool_router: Self::tool_router(),
        }
    }

    /// 获取以太坊地址余额(支持 ETH 和 ERC20)
    #[rmcp::tool(description = "获取以太坊地址余额(支持 ETH 和 ERC20 代币)")]
    fn get_balance(
        &self,
        args: Parameters<GetBalanceArgs>,
    ) -> Result<CallToolResult, McpError> {
        get_balance(
            &self.config,
            &self.eth_client,
            &self.erc20_client,
            &self.token_registry,
            args,
        )
    }

    /// 获取代币价格(支持 USD 和 ETH 报价)
    #[rmcp::tool(description = "获取代币在 Uniswap V2 上的价格(支持 USD 和 ETH 报价)")]
    fn get_token_price(
        &self,
        args: Parameters<GetTokenPriceArgs>,
    ) -> Result<CallToolResult, McpError> {
        get_token_price(
            &self.config,
            &self.uniswap_client,
            &self.erc20_client,
            &self.token_registry,
            args,
        )
    }

    /// 模拟代币交换(Uniswap V2)
    #[rmcp::tool(description = "模拟 Uniswap V2 代币交换,返回预估输出和价格影响")]
    fn swap_tokens(
        &self,
        args: Parameters<SwapTokensArgs>,
    ) -> Result<CallToolResult, McpError> {
        swap_tokens(
            &self.config,
            &self.uniswap_client,
            &self.erc20_client,
            &self.token_registry,
            args,
        )
    }
}

#[rmcp::tool_handler]
impl ServerHandler for EthereumTradingServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "以太坊交易 MCP 服务器 - 提供余额查询、价格查询和交换模拟功能。\n\
                 可用工具:\n\
                 - get_balance: 获取以太坊地址余额(支持 ETH 和 ERC20)\n\
                 - get_token_price: 获取代币在 Uniswap V2 上的价格(支持 USD 和 ETH 报价)\n\
                 - swap_tokens: 模拟 Uniswap V2 代币交换(返回预估输出和价格影响)"
                    .to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("🚀 启动 Ethereum Trading MCP Server...");
    eprintln!();

    // 加载配置
    let config = Config::from_env()?;

    // 初始化日志系统
    logging::init_logging(&config.server.log_level, config.server.log_json_format)?;
    info!("日志系统已初始化");

    // 验证配置
    config.validate()?;

    // 打印配置信息
    config.print_info();
    eprintln!();

    // 创建 Ethereum 客户端和 Provider
    let rpc_url = if config.server.test_mode {
        None
    } else {
        config.ethereum.rpc_url.as_deref()
    };

    let provider = if let Some(url) = rpc_url {
        match Provider::<Http>::try_from(url) {
            Ok(provider) => Some(Arc::new(provider)),
            Err(e) => {
                eprintln!("⚠️  无法创建 Provider: {}", e);
                None
            }
        }
    } else {
        None
    };

    let eth_client = EthClient::new(rpc_url, Some(config.ethereum.chain_id)).await?;

    if eth_client.is_available() {
        info!("Ethereum 客户端已连接");
    } else {
        info!("运行在离线模式(未连接到 Ethereum 网络)");
    }

    // 创建服务器实例
    let server = EthereumTradingServer::new(config, eth_client, provider);

    eprintln!("🔧 可用工具:");
    eprintln!("   - get_balance: 获取以太坊地址余额");
    eprintln!("   - get_token_price: 获取代币价格");
    eprintln!("   - swap_tokens: 模拟代币交换");
    eprintln!();

    eprintln!("✅ 服务器已准备就绪,等待连接...");
    eprintln!();

    // 使用 stdio 传输层启动服务器
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::balance::BalanceResult;
    use crate::types::TokenInfo;
    use rmcp::handler::server::wrapper::Parameters;

    /// 创建测试用 EthClient
    async fn create_test_eth_client() -> EthClient {
        EthClient::new(None, None)
            .await
            .expect("应该能创建测试客户端")
    }

    /// 创建测试配置(强制测试模式)
    fn create_test_config() -> Config {
        let mut config = Config::from_env().expect("应该能加载配置");
        config.server.test_mode = true;
        config.server.test_balance = 100.0;
        config
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = create_test_config();
        let eth_client = create_test_eth_client().await;
        let server = EthereumTradingServer::new(config, eth_client, None);
        assert!(server.config.server.test_mode);
    }

    #[tokio::test]
    async fn test_get_balance_eth() {
        let config = create_test_config();
        let eth_client = create_test_eth_client().await;
        let server = EthereumTradingServer::new(config, eth_client, None);

        let args = GetBalanceArgs {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            token_address: None,
        };

        let result = server.get_balance(Parameters(args));
        assert!(result.is_ok(), "get_balance 应该成功返回");

        let call_result = result.unwrap();
        assert!(!call_result.content.is_empty(), "返回内容不应为空");
        assert!(
            call_result.is_error.is_none() || !call_result.is_error.unwrap(),
            "不应该是错误状态"
        );
    }

    #[tokio::test]
    async fn test_get_balance_erc20() {
        let config = create_test_config();
        let eth_client = create_test_eth_client().await;
        let server = EthereumTradingServer::new(config, eth_client, None);

        let args = GetBalanceArgs {
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
            token_address: Some("USDC".to_string()),
        };

        let result = server.get_balance(Parameters(args));
        assert!(result.is_ok(), "get_balance 应该成功返回");
    }

    #[tokio::test]
    async fn test_server_info() {
        let config = create_test_config();
        let eth_client = create_test_eth_client().await;
        let server = EthereumTradingServer::new(config, eth_client, None);
        let info = server.get_info();

        assert_eq!(info.protocol_version, ProtocolVersion::V_2024_11_05);
        assert!(info.capabilities.tools.is_some());
        assert!(info.instructions.is_some());
    }

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
        let json = r#"{"address":"0x123","token_address":"USDC"}"#;
        let args: GetBalanceArgs = serde_json::from_str(json).expect("应该能反序列化");
        assert_eq!(args.address, "0x123");
        assert_eq!(args.token_address, Some("USDC".to_string()));

        let json = r#"{"address":"0x123"}"#;
        let args: GetBalanceArgs = serde_json::from_str(json).expect("应该能反序列化");
        assert_eq!(args.address, "0x123");
        assert_eq!(args.token_address, None);
    }

    #[tokio::test]
    async fn test_concurrent_balance_queries() {
        let config = create_test_config();
        let eth_client = create_test_eth_client().await;
        let server = EthereumTradingServer::new(config, eth_client, None);

        let mut handles = vec![];
        for i in 0..5 {
            let server_clone = server.clone();
            let handle = tokio::spawn(async move {
                let args = GetBalanceArgs {
                    address: format!("0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb{}", i),
                    token_address: None,
                };
                server_clone.get_balance(Parameters(args))
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.expect("任务应该成功完成");
            assert!(result.is_ok());
        }
    }
}
