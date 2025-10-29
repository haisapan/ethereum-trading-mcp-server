use rmcp::{
    handler::server::{
        router::tool::ToolRouter,
        wrapper::Parameters,  // ← 正确的导入路径！
        ServerHandler,
    },
    model::{CallToolResult, Content},
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServiceExt,
};

// 导入工具模块
mod tools;
use tools::{
    balance::{get_balance, GetBalanceRequest},
    price::{get_token_price, GetTokenPriceRequest},
    swap::{swap_tokens, SwapTokensRequest},
};

/// Ethereum Trading MCP Server
/// 提供以太坊交易相关的 MCP 工具
#[derive(Clone)]
pub struct EthereumTradingServer {
    tool_router: ToolRouter<Self>,
    // TODO: 添加依赖注入字段
    // config: Arc<Config>,
    // eth_client: Arc<EthClient>,
    // erc20_client: Arc<Erc20Client>,
    // token_registry: Arc<TokenRegistry>,
}

/// 使用 tool_router 宏定义服务器工具
#[tool_router]
impl EthereumTradingServer {
    /// 创建新的服务器实例
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            // TODO: 初始化依赖
        }
    }

    // ==================== 以太坊交易工具 ====================

    /// 查询 ETH 或 ERC20 代币余额
    #[tool(description = "查询以太坊地址的 ETH 余额或 ERC20 代币余额。如果不提供 token_address，则查询 ETH 余额；否则查询指定 ERC20 代币的余额。")]
    async fn get_balance(
        &self,
        Parameters(request): Parameters<GetBalanceRequest>,
    ) -> Result<CallToolResult, McpError> {
        // TODO: 从 self 访问依赖
        // let config = &self.config;
        // let eth_client = &self.eth_client;
        get_balance(request).await
    }

    /// 获取代币价格
    #[tool(description = "获取代币在 Uniswap V2 上的当前价格。支持 USD 和 ETH 报价。可以使用代币符号（如 USDT）或合约地址。")]
    async fn get_token_price(
        &self,
        Parameters(request): Parameters<GetTokenPriceRequest>,
    ) -> Result<CallToolResult, McpError> {
        // TODO: 从 self 访问依赖
        get_token_price(request).await
    }

    /// 模拟代币交换
    #[tool(description = "在 Uniswap V2/V3 上模拟代币交换。构造真实的交换交易并使用 eth_call 进行链上模拟（不实际执行）。返回预估输出、价格影响和 Gas 费用。")]
    async fn swap_tokens(
        &self,
        Parameters(request): Parameters<SwapTokensRequest>,
    ) -> Result<CallToolResult, McpError> {
        // TODO: 从 self 访问依赖
        swap_tokens(request).await
    }

    // ==================== 辅助工具 ====================

    /// 简单的 Hello World 工具（用于测试）
    #[tool(description = "返回一个简单的问候消息（测试工具）")]
    async fn hello(&self) -> Result<CallToolResult, McpError> {
        let greeting = "Hello, World! 👋 欢迎使用 Ethereum Trading MCP Server!";

        Ok(CallToolResult::success(vec![Content::text(
            greeting.to_string(),
        )]))
    }

    /// 获取服务器信息
    #[tool(description = "获取 MCP 服务器的基本信息，包括版本、功能列表等")]
    async fn server_info(&self) -> Result<CallToolResult, McpError> {
        let info = r#"
🦀 Ethereum Trading MCP Server
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
版本: 0.1.0
语言: Rust
框架: rmcp 0.8.3 (Model Context Protocol)

📋 可用工具:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🔹 核心功能:
  • get_balance      - 查询 ETH 和 ERC20 代币余额
  • get_token_price  - 获取代币价格 (USD/ETH)
  • swap_tokens      - 模拟 Uniswap 代币交换

🔹 辅助工具:
  • hello           - 测试连接
  • server_info     - 显示本信息

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📦 技术栈:
  - ethers-rs 2.0.14 (Ethereum 客户端)
  - rmcp 0.8.3 (MCP SDK)
  - tokio 1.48.0 (异步运行时)

🔗 支持的协议:
  - Ethereum Mainnet, Goerli, Sepolia
  - Uniswap V2/V3

⚠️  注意: swap_tokens 仅进行模拟，不会实际执行交易
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        "#.trim();

        Ok(CallToolResult::success(vec![Content::text(
            info.to_string(),
        )]))
    }
}

/// 实现 ServerHandler trait 以处理 MCP 协议
#[tool_handler]
impl ServerHandler for EthereumTradingServer {
    // 使用默认实现即可，tool_router 会自动处理工具调用
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("🚀 启动 Ethereum Trading MCP Server...");

    // 创建服务器实例
    let server = EthereumTradingServer::new();

    // 创建 stdio 传输层
    let transport = stdio();

    // 启动服务器
    tracing::info!("✅ MCP Server 已就绪，等待客户端连接...");
    tracing::info!("📋 可用工具: get_balance, get_token_price, swap_tokens, hello, server_info");

    let service = server.serve(transport).await?;

    // 等待服务器关闭
    let quit_reason = service.waiting().await?;
    tracing::info!("👋 MCP Server 关闭，原因: {:?}", quit_reason);

    Ok(())
}
