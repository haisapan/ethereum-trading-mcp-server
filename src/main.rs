use rmcp::{
    handler::server::{router::tool::ToolRouter, ServerHandler},
    model::{CallToolResult, Content},
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServiceExt,
};

/// Hello World MCP Server
/// 提供一个简单的问候工具
#[derive(Clone)]
pub struct HelloWorldServer {
    tool_router: ToolRouter<Self>,
}

/// 使用 tool_router 宏定义服务器工具
#[tool_router]
impl HelloWorldServer {
    /// 创建新的 HelloWorld 服务器实例
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// 简单的 Hello World 工具
    #[tool(description = "返回一个简单的问候消息")]
    async fn hello(&self) -> Result<CallToolResult, McpError> {
        let greeting = "Hello, World! 👋 欢迎使用 Rust MCP Server!";

        Ok(CallToolResult::success(vec![Content::text(
            greeting.to_string(),
        )]))
    }

    /// 获取服务器信息工具
    #[tool(description = "获取 MCP 服务器的基本信息")]
    async fn server_info(&self) -> Result<CallToolResult, McpError> {
        let info = r#"
🦀 Hello World MCP Server
━━━━━━━━━━━━━━━━━━━━━━━━━━━
版本: 0.1.0
语言: Rust
框架: rmcp (Model Context Protocol)
功能:
  - hello: 返回简单的问候消息
  - server_info: 获取服务器信息
━━━━━━━━━━━━━━━━━━━━━━━━━━━
        "#.trim();

        Ok(CallToolResult::success(vec![Content::text(
            info.to_string(),
        )]))
    }
}

/// 实现 ServerHandler trait 以处理 MCP 协议
#[tool_handler]
impl ServerHandler for HelloWorldServer {
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

    tracing::info!("🚀 启动 Hello World MCP Server...");

    // 创建服务器实例
    let server = HelloWorldServer::new();

    // 创建 stdio 传输层
    let transport = stdio();

    // 启动服务器
    tracing::info!("✅ MCP Server 已就绪，等待客户端连接...");
    let service = server.serve(transport).await?;

    // 等待服务器关闭
    let quit_reason = service.waiting().await?;
    tracing::info!("👋 MCP Server 关闭，原因: {:?}", quit_reason);

    Ok(())
}
