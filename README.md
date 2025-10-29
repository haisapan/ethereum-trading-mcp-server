# Ethereum Trading MCP Server

一个基于 Rust 和 MCP (Model Context Protocol) 的以太坊交易服务器。

## 项目简介

这是一个使用 Rust 实现的 MCP 服务器，提供以太坊区块链查询和 Uniswap V2 交易模拟功能。支持测试模式和真实模式。
当前实现使用 STDIO 传输类型（`type: "stdio"`），通过标准输入输出与 MCP 客户端进行通信。

## 功能特性

### 可用工具

- **get_balance**: 获取以太坊地址余额（支持 ETH 和 ERC20 代币）

  - 真实模式：连接以太坊主网查询实际余额
  - 测试模式：返回固定测试值
  - 使用 U256 保证精度，支持任意大额余额

- **swap_tokens**: 模拟 Uniswap V2 代币交换

  - 真实模式：
    - 通过 eth_call 调用 Uniswap V2 Router 模拟真实交易
    - 返回 Gas 估算和路由信息
    - 检测流动性、余额、授权等问题
    - 提供 revert 原因分析
  - 测试模式：返回模拟数据
  - 使用 rust_decimal 保证金额精度

- **get_token_price**: 查询代币价格（基于 Uniswap V2 储备量）

## 技术栈

- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio
- **MCP SDK**: rmcp 0.8
- **序列化**: serde, serde_json

## 安装和运行

### 前置要求

- Rust 1.70+
- Cargo

### 配置环境变量

1. 复制环境变量示例文件：

```bash
cp .env.example .env
```

2. 编辑 `.env` 文件，根据需要修改配置：

**测试模式**（用于开发和测试）：

```bash
TEST_MODE=true
TEST_BALANCE=100.0
```

**真实模式**（连接以太坊主网）：

```bash
TEST_MODE=false
ETHEREUM_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY
CHAIN_ID=1

# 可选：用于模拟的钱包私钥（不会发送实际交易）
# 如果提供，将从私钥派生地址用于模拟
# 如果不提供，使用默认的高余额地址（Vitalik 地址）
ETH_PRIVATE_KEY=0x...
```

完整的环境变量配置说明请查看 [ENV_CONFIG.md](./ENV_CONFIG.md)

### 编译项目

```bash
# Debug 模式
cargo build

# Release 模式
cargo build --release
```

### 运行服务器

```bash
# Debug 模式
cargo run

# Release 模式
cargo run --release
```

服务器启动后会监听 stdio，等待 MCP 客户端连接。

### 运行测试

项目包含完整的单元测试套件：

```bash
# 运行所有测试
cargo test

# 运行测试并显示详细输出
cargo test -- --nocapture

# 运行特定测试
cargo test test_get_balance_with_address
```

**测试覆盖**：

- ✅ 9 个单元测试
- ✅ 100% 通过率
- ✅ 包含并发测试
- ✅ 完整的功能验证

详细测试文档请查看 [TESTING.md](./TESTING.md)

### 使用 MCP Inspector 测试

你可以使用 MCP Inspector 来测试服务器：

```bash
npx @modelcontextprotocol/inspector cargo run --release
```

## 项目结构

```
.
├── Cargo.toml          # 项目配置和依赖
├── README.md           # 项目文档
├── src/                # 服务器核心代码
│   ├── main.rs         # 主程序入口
│   ├── config.rs       # 配置加载和校验
│   ├── tools/          # MCP 工具实现
│   └── …               # 其他模块（ERC20、Uniswap 等）
├── doc/                # 设计与需求文档
├── test-mcp.sh         # 简单的 MCP 测试脚本
├── .env.example        # 环境变量示例
└── .env                # 项目本地配置（可选）
```

## 代码架构

### 核心组件

1. **EthereumTradingServer**: 主服务器结构体

   - 使用 `#[tool_router]` 宏自动生成工具路由
   - 实现 `ServerHandler` trait 提供服务器元数据

2. **工具定义**

   - 使用 `#[tool]` 宏定义工具
   - 通过 `Parameters<T>` 接收参数
   - 返回 `CallToolResult` 类型

3. **传输层**
   - 使用 stdio 传输层与客户端通信
   - 支持标准输入输出的双向通信

## API 示例

### get_balance

**描述**: 获取以太坊地址余额

**参数**:

```json
{
  "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  "token_address": "USDC" // 可选，不填则查询 ETH 余额
}
```

**返回示例**:

```json
{
  "address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
  "token": {
    "symbol": "ETH",
    "name": "Ethereum",
    "address": "0x0000000000000000000000000000000000000000",
    "decimals": 18
  },
  "balance": "1234567890123456789",
  "decimals": 18,
  "formatted_balance": "1.234567890123456789"
}
```

### swap_tokens

**描述**: 模拟 Uniswap V2 代币交换

**参数**:

```json
{
  "from_token": "ETH",
  "to_token": "USDC",
  "amount": "1.5",
  "slippage_bps": 50, // 0.5%
  "wallet_address": "0xYourAddress" // 可选，用于 Gas 估算
}
```

**返回示例**:

```json
{
  "from_token": {
    "symbol": "ETH",
    "name": "Ethereum",
    "address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    "decimals": 18
  },
  "to_token": {
    "symbol": "USDC",
    "name": "USD Coin",
    "address": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "decimals": 6
  },
  "input_amount": "1.5",
  "estimated_output": "3500.123456",
  "minimum_output": "3482.622839",
  "price_impact": "0.15%",
  "route": {
    "protocol": "Uniswap V2",
    "path": [
      "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
      "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
    ],
    "pools": ["0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"]
  },
  "simulation_success": true,
  "gas_estimate": "150000"
}
```

#### 测试提示

在 MCP Inspector 或 Claude Desktop 中可以直接使用以下参数验证 `swap_tokens` 工具的错误处理逻辑：

```json
{
  "from_token": "ETH",
  "to_token": "USDC",
  "amount": "0.1",
  "slippage_bps": 50,
  "wallet_address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
}
```

该示例会模拟 Vitalik 地址从 ETH 兑换 USDC。由于该地址未对 Uniswap Router 进行授权，预期返回 `simulation_success: false`，并给出 `TransferHelper: TRANSFER_FROM_FAILED` 的 revert 原因，以帮助测试端到端的错误提示。

## 核心特性

### 精度保证

- **余额查询**：使用 U256 存储，支持任意大额（无 f64 溢出问题）
- **金额解析**：使用 rust_decimal + 字符串操作，完全避免浮点运算
- **支持范围**：
  - ETH 余额：支持 > 18.4 ETH（u64 限制已移除）
  - 代币金额：支持任意大额和高精度小数
  - **高精度代币**：支持 decimals ≥ 20 的代币（避免 10^n 溢出）

### 真实交易模拟

- **Router 集成**：通过 eth_call 调用 Uniswap V2 Router 合约
- **智能地址选择**：
  - 优先使用用户提供的 wallet_address 参数
  - 如果未提供，从配置的 ETH_PRIVATE_KEY 派生地址
  - 如果没有私钥，使用知名高余额地址（Vitalik）作为默认模拟地址
  - 避免零地址导致的 ERC-20 transfer 失败
- **错误检测**：
  - 流动性不足
  - 代币授权失败
  - 余额不足
  - 滑点保护触发
- **Gas 估算**：提供真实的 Gas 消耗预估
- **Revert 分析**：解析并返回交易失败原因

### 已知限制

- **仅支持 Uniswap V2**：暂不支持 V3 和其他 DEX
- **只读模式**：不支持实际交易签名和发送
- **主网限制**：仅支持以太坊主网（Chain ID: 1）
- **路由简化**：仅支持直接路径或通过 WETH 的两跳路径

## 开发计划

- [x] 支持 Uniswap V3
- [x] 添加交易签名和发送功能
- [x] 支持多链（BSC, Polygon, Arbitrum 等）
- [x] 添加价格预言机集成
- [x] 实现复杂多跳路由
- [x] 添加钱包管理功能

## 配置说明

### 与 Claude Desktop 集成

编辑 `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ethereum-trading": {
      "command": "/path/to/ethereum-trading-server",
      "args": []
    }
  }
}
```

如果你已经构建好了 release 版本的二进制，并需要为 Claude Desktop / Claude Code 提供环境变量，可以使用类似下面的配置（路径请根据实际情况调整）：

```json
{
  "mcpServers": {
    "ethereum-trading": {
      "command": "/path/to/ethereum-trading-mcp-server/target/release/ethereum-trading-server",
      "args": [],
      "env": {
        "TEST_MODE": "false",
        "ETHEREUM_RPC_URL": "https://eth.llamarpc.com",
        "ETHEREUM_CHAIN_ID": "1",
        "LOG_LEVEL": "info",
        "LOG_JSON_FORMAT": "false"
      }
    }
  }
}
```

## 注意事项

⚠️ **重要提示**:

1. **测试模式 vs 真实模式**：

   - 测试模式：返回固定数据，无需 RPC 连接
   - 真实模式：连接主网，消耗 RPC 配额

2. **只读操作**：

   - 当前版本仅支持查询和模拟
   - 不会发送实际交易到区块链
   - 未来版本将支持交易签名和发送

3. **RPC 配额管理**：

   - 建议使用 Alchemy、Infura 等服务
   - 注意 API 调用频率限制
   - 真实模式下每次查询都会调用 RPC

4. **安全性**：
   - 不要在配置文件中存储私钥
   - 使用环境变量管理敏感信息
   - 模拟结果不保证实际交易成功
