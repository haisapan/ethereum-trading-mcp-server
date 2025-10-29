# 环境变量配置指南

本文档详细说明 Ethereum Trading MCP Server 支持的所有环境变量配置。

## 快速开始

1. 复制示例配置文件：

```bash
cp .env.example .env
```

2. 编辑 `.env` 文件，填入你的配置值

3. 启动服务器（会自动加载 .env 文件）：

```bash
cargo run --release
```

## 配置分类

### 🖥️ 服务器配置

#### `SERVER_NAME`

- **类型**: String
- **默认值**: `ethereum-trading-server`
- **说明**: 服务器名称
- **示例**:
  ```bash
  SERVER_NAME=my-eth-server
  ```

#### `SERVER_VERSION`

- **类型**: String
- **默认值**: `0.1.0`
- **说明**: 服务器版本号
- **示例**:
  ```bash
  SERVER_VERSION=1.0.0
  ```

#### `SERVER_DESCRIPTION`

- **类型**: String
- **默认值**: `以太坊交易 MCP 服务器 - 提供余额查询等功能`
- **说明**: 服务器描述信息
- **示例**:
  ```bash
  SERVER_DESCRIPTION=我的以太坊交易服务器
  ```

#### `LOG_LEVEL`

- **类型**: String (trace | debug | info | warn | error)
- **默认值**: `info`
- **说明**: 日志级别
- **示例**:
  ```bash
  LOG_LEVEL=debug
  ```

---

### 🧪 测试模式配置

#### `TEST_MODE`

- **类型**: Boolean
- **默认值**: `true`
- **说明**: 是否启用测试模式。测试模式下返回固定的测试值，不连接真实网络
- **示例**:
  ```bash
  TEST_MODE=true
  ```

#### `TEST_BALANCE`

- **类型**: Float
- **默认值**: `100.0`
- **说明**: 测试模式下返回的余额值（单位：ETH）
- **示例**:
  ```bash
  TEST_BALANCE=200.5
  ```

---

### ⛓️ 以太坊网络配置

#### `ETH_RPC_URL`

- **类型**: String (URL)
- **默认值**: 空
- **必需**: 非测试模式下必需
- **说明**: 以太坊 RPC 节点地址
- **示例**:

  ```bash
  # Alchemy
  ETH_RPC_URL=https://eth.llamarpc.com

  # Infura
  ETH_RPC_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID

  # 本地节点
  ETH_RPC_URL=http://localhost:8545
  ```

#### `ETH_NETWORK_ID`

- **类型**: Integer
- **默认值**: 空
- **说明**: 以太坊网络 ID
- **常用值**:
  - `1` - 主网 (Mainnet)
  - `5` - Goerli 测试网
  - `11155111` - Sepolia 测试网
- **示例**:
  ```bash
  ETH_NETWORK_ID=1
  ```

#### `ETH_CHAIN_ID`

- **类型**: Integer
- **默认值**: 空
- **说明**: 链 ID，用于签名验证
- **示例**:
  ```bash
  ETH_CHAIN_ID=1
  ```

---

### 🔑 API 密钥配置

#### `ALCHEMY_API_KEY`

- **类型**: String
- **默认值**: 空
- **说明**: Alchemy API 密钥
- **获取方式**: https://www.alchemy.com/
- **示例**:
  ```bash
  ALCHEMY_API_KEY=your_alchemy_api_key_here
  ```

#### `INFURA_API_KEY`

- **类型**: String
- **默认值**: 空
- **说明**: Infura API 密钥
- **获取方式**: https://infura.io/
- **示例**:
  ```bash
  INFURA_API_KEY=your_infura_api_key_here
  ```

#### `ETHERSCAN_API_KEY`

- **类型**: String
- **默认值**: 空
- **说明**: Etherscan API 密钥（用于交易验证）
- **获取方式**: https://etherscan.io/apis
- **示例**:
  ```bash
  ETHERSCAN_API_KEY=your_etherscan_api_key_here
  ```

---

### 🔐 钱包配置（未来功能）

⚠️ **安全警告**: 生产环境不要直接在 .env 文件中存储私钥或助记词！

#### `PRIVATE_KEY`

- **类型**: String
- **默认值**: 空
- **说明**: 钱包私钥（仅用于开发测试）
- **安全建议**: 使用硬件钱包或密钥管理服务

#### `MNEMONIC`

- **类型**: String
- **默认值**: 空
- **说明**: 钱包助记词（仅用于开发测试）
- **安全建议**: 使用硬件钱包或密钥管理服务

---

### ⚡ 性能配置（未来功能）

#### `HTTP_TIMEOUT`

- **类型**: Integer
- **默认值**: `30`
- **说明**: HTTP 请求超时时间（秒）
- **示例**:
  ```bash
  HTTP_TIMEOUT=60
  ```

#### `MAX_CONCURRENT_REQUESTS`

- **类型**: Integer
- **默认值**: `10`
- **说明**: 最大并发请求数
- **示例**:
  ```bash
  MAX_CONCURRENT_REQUESTS=20
  ```

---

## 配置示例

### 开发环境（测试模式）

```bash
# .env
TEST_MODE=true
TEST_BALANCE=100.0
LOG_LEVEL=debug
```

### 生产环境（主网）

```bash
# .env
TEST_MODE=false
ETH_RPC_URL=https://eth.llamarpc.com
ETH_NETWORK_ID=1
ETH_CHAIN_ID=1
ALCHEMY_API_KEY=your_alchemy_api_key
LOG_LEVEL=info
```

### 生产环境（测试网）

```bash
# .env
TEST_MODE=false
ETH_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_API_KEY
ETH_NETWORK_ID=11155111
ETH_CHAIN_ID=11155111
ALCHEMY_API_KEY=your_alchemy_api_key
LOG_LEVEL=info
```

---

## 配置验证

服务器启动时会自动验证配置：

1. **测试模式检查**: 如果 `TEST_MODE=false`，必须配置 `ETH_RPC_URL`
2. **余额值检查**: `TEST_BALANCE` 不能为负数
3. **配置信息打印**: 启动时会打印配置信息（隐藏敏感数据）

---

## 安全最佳实践

### ✅ 推荐做法

1. **使用 .env 文件**: 将配置放在 `.env` 文件中，不要硬编码
2. **添加到 .gitignore**: 确保 `.env` 文件不会被提交到 Git
3. **使用环境变量**: 生产环境使用系统环境变量而不是 .env 文件
4. **定期轮换密钥**: 定期更换 API 密钥
5. **最小权限原则**: API 密钥只授予必要的权限

### ❌ 避免做法

1. ❌ 不要将 `.env` 文件提交到 Git
2. ❌ 不要在代码中硬编码密钥
3. ❌ 不要在 .env 中存储生产环境的私钥
4. ❌ 不要共享包含敏感信息的 .env 文件
5. ❌ 不要在日志中输出完整的 API 密钥

---

## 环境变量优先级

配置加载顺序（后者覆盖前者）：

1. 内置默认值
2. `.env` 文件
3. 系统环境变量

示例：

```bash
# .env 文件
TEST_BALANCE=100.0

# 命令行覆盖
TEST_BALANCE=200.0 cargo run
```

---

## 故障排查

### 问题：配置没有生效

**解决方案**:

1. 确认 `.env` 文件在项目根目录
2. 检查文件编码是否为 UTF-8
3. 确认环境变量名称拼写正确
4. 重启服务器

### 问题：非测试模式启动失败

**错误信息**: "非测试模式下必须配置 ETH_RPC_URL"

**解决方案**:

```bash
# 设置 RPC URL
echo "ETH_RPC_URL=https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY" >> .env

# 或者启用测试模式
echo "TEST_MODE=true" >> .env
```

### 问题：如何查看当前配置

服务器启动时会打印配置信息：

```
📋 配置信息:
  服务器名称: ethereum-trading-server
  服务器版本: 0.1.0
  测试模式: true
  测试余额: 100 ETH
```

---

## 参考资源

- [Alchemy 文档](https://docs.alchemy.com/)
- [Infura 文档](https://docs.infura.io/)
- [Etherscan API 文档](https://docs.etherscan.io/)
- [以太坊 JSON-RPC 文档](https://ethereum.org/en/developers/docs/apis/json-rpc/)

---

**最后更新**: 2025-10-28
**维护者**: Ethereum Trading MCP Server Team
