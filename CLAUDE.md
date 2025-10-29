# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

这是一个用 Rust 编写的 **Ethereum Trading MCP Server**,实现 Model Context Protocol (MCP) 协议,为 AI 代理提供以太坊区块链交互能力。

### 核心功能

- **余额查询** (`get_balance`): 查询 ETH 和 ERC20 代币余额
- **价格查询** (`get_token_price`): 获取代币在 Uniswap V2 上的价格(USD/ETH)
- **代币交换模拟** (`swap_tokens`): 模拟 Uniswap V2/V3 代币交换,返回预估输出和 Gas 成本

## Development Commands

### Build & Run

```bash
# 构建项目
cargo build

# 发布构建(优化版本)
cargo build --release

# 运行服务器
cargo run

# 发布运行
cargo run --release
```

### Testing

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_name

# 运行测试并显示输出
cargo test -- --nocapture

# 运行集成测试
cargo test --test integration_test
```

### Code Quality

```bash
# 代码格式化
cargo fmt

# 检查格式
cargo fmt -- --check

# Linting
cargo clippy

# 严格 Linting
cargo clippy -- -D warnings
```

## Environment Configuration

项目需要 `.env` 文件配置:

```bash
# 从模板创建配置文件
cp .env.example .env
```

### 关键配置项

- `ETHEREUM_RPC_URL`: 以太坊 RPC 节点地址(默认使用公共节点)
- `CHAIN_ID`: 链 ID (1=主网, 5=Goerli, 11155111=Sepolia)
- `ETH_PRIVATE_KEY`: 私钥(用于签名交易,可选)
- `DEFAULT_SLIPPAGE_BPS`: 默认滑点容差(50 = 0.5%)
- `LOG_LEVEL`: 日志级别(trace/debug/info/warn/error)

## Architecture

### Technology Stack

- **MCP 框架**: `rmcp` (Model Context Protocol Rust SDK)
- **Ethereum 客户端**: `ethers-rs` (支持 WebSocket 和 TLS)
- **异步运行时**: `tokio` (full features)
- **日志系统**: `tracing` + `tracing-subscriber`
- **精度计算**: `rust_decimal` (金融精度要求)

### Project Structure

```
ethereum-trading-mcp-server/
├── src/
│   └── main.rs              # 主入口(当前为 Hello World)
├── doc/
│   ├── requirement.txt      # 项目需求文档
│   └── design.md           # 架构设计(待完善)
├── .env.example            # 环境变量模板
├── Cargo.toml              # 依赖配置
└── CLAUDE.md              # 本文件
```

### Key Design Considerations

1. **实时数据**: 必须连接真实的以太坊 RPC 节点获取链上数据
2. **交易模拟**: 使用 `eth_call` 模拟 Uniswap 交换,不实际执行链上交易
3. **精度处理**: 使用 `rust_decimal` 处理代币金额,避免浮点精度问题
4. **异步架构**: 基于 `tokio` 实现高性能异步 I/O
5. **日志追踪**: 使用结构化日志便于调试和生产监控

## Development Notes

### 项目状态

⚠️ **当前阶段**: 项目刚初始化,`main.rs` 仅包含 Hello World 代码,核心功能尚未实现。

### 待实现功能

根据 `doc/requirement.txt`,需要实现:

1. MCP 服务器基础框架
2. 三个核心工具 (get_balance, get_token_price, swap_tokens)
3. Ethereum RPC 客户端封装
4. Uniswap V2/V3 交互逻辑
5. 钱包管理和交易签名
6. 错误处理和日志系统
7. 集成测试和文档

### Code Organization Principles

遵循 CLAUDE.md 架构指导原则:

- **文件行数限制**: Rust 文件不超过 350 行
- **文件夹组织**: 每个文件夹不超过 10 个文件,超过需拆分子目录
- **模块化设计**: 避免循环依赖、冗余代码和过度复杂性
- **清晰命名**: 保持代码意图明确,避免晦涩性

### Suggested Module Structure

```
src/
├── main.rs                 # 服务器启动和 MCP 初始化
├── config.rs              # 配置管理和环境变量
├── error.rs               # 统一错误类型定义
├── ethereum/
│   ├── mod.rs
│   ├── client.rs          # Ethereum RPC 客户端封装
│   ├── balance.rs         # 余额查询逻辑
│   └── swap.rs            # Uniswap 交换逻辑
├── mcp/
│   ├── mod.rs
│   ├── server.rs          # MCP 服务器实现
│   └── tools.rs           # MCP 工具定义
└── utils/
    ├── mod.rs
    ├── decimal.rs         # 精度计算工具
    └── logging.rs         # 日志配置
```

## Testing Guidelines

- **单元测试**: 测试独立函数逻辑(如精度转换、地址验证)
- **集成测试**: 测试 Ethereum RPC 交互(可使用测试网或 Mock)
- **MCP 工具测试**: 验证工具调用的 JSON-RPC 格式正确性

## Dependencies

核心依赖已配置在 `Cargo.toml`:

- `ethers = "2.0.14"` - Ethereum 交互库
- `rmcp = "0.8.3"` - MCP SDK
- `tokio = "1.48.0"` - 异步运行时
- `tracing = "0.1.41"` - 结构化日志
- `rust_decimal = "1.39.0"` - 高精度数值计算
- `serde = "1.0.228"` - JSON 序列化
- `dotenv = "0.15.0"` - 环境变量加载
- `anyhow = "1.0.100"` - 错误处理
- `thiserror = "2.0.17"` - 自定义错误类型

## Important Warnings

- **私钥安全**: 不要在代码中硬编码私钥,仅通过 `.env` 加载(已在 `.gitignore` 中排除)
- **RPC 限制**: 公共 RPC 节点有速率限制,生产环境建议使用 Infura/Alchemy
- **测试网建议**: 开发阶段使用 Sepolia 测试网避免真实资金风险
- **Gas 估算**: 交换模拟时需正确估算 Gas,避免交易失败

## Additional Resources

- [ethers-rs 文档](https://docs.rs/ethers/)
- [MCP 协议规范](https://spec.modelcontextprotocol.io/)
- [Uniswap V2 文档](https://docs.uniswap.org/contracts/v2/overview)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)
