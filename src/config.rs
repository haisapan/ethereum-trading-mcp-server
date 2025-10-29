use ethers::prelude::*;
use std::env;

/// 服务器配置结构体
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 服务器名称
    pub name: String,
    /// 服务器版本
    pub version: String,
    /// 日志级别
    pub log_level: String,
    /// 是否启用 JSON 格式日志
    pub log_json_format: bool,
    /// 是否启用测试模式
    pub test_mode: bool,
    /// 测试模式返回的余额值
    pub test_balance: f64,
}

/// 以太坊网络配置
#[derive(Debug, Clone)]
pub struct EthereumConfig {
    /// RPC 节点地址
    pub rpc_url: Option<String>,
    /// Chain ID
    pub chain_id: u64,
    /// 私钥（用于签名交易）
    pub private_key: Option<String>,
}

/// 交易配置
#[derive(Debug, Clone)]
pub struct TradingConfig {
    /// 默认滑点容差（基点，50 = 0.5%）
    pub default_slippage_bps: u32,
    /// Gas 价格策略
    pub gas_price_strategy: String,
    /// 最大 Gas 限制
    pub max_gas_limit: u64,
}

/// Uniswap 配置
#[derive(Debug, Clone)]
pub struct UniswapConfig {
    /// Uniswap V2 Router 地址
    pub v2_router: String,
    /// Uniswap V3 Router 地址
    pub v3_router: String,
}

/// API 密钥配置
#[derive(Debug, Clone)]
pub struct ApiKeysConfig {
    /// Alchemy API Key
    pub alchemy_api_key: Option<String>,
    /// Infura API Key
    pub infura_api_key: Option<String>,
    /// Etherscan API Key
    pub etherscan_api_key: Option<String>,
    /// CoinGecko API Key
    pub coingecko_api_key: Option<String>,
}

/// 性能配置
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// HTTP 请求超时时间（秒）
    pub http_timeout: u64,
    /// 最大并发请求数
    pub max_concurrent_requests: usize,
    /// RPC 重试次数
    pub rpc_retry_count: u32,
    /// 价格缓存时间（秒）
    pub price_cache_ttl: u64,
}

/// 完整配置
#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub ethereum: EthereumConfig,
    pub trading: TradingConfig,
    pub uniswap: UniswapConfig,
    pub api_keys: ApiKeysConfig,
    pub performance: PerformanceConfig,
    /// 代币注册表文件路径
    pub token_registry_path: Option<String>,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> anyhow::Result<Self> {
        // 尝试加载 .env 文件（如果存在）
        dotenv::dotenv().ok();

        let server = ServerConfig {
            name: env::var("SERVER_NAME")
                .unwrap_or_else(|_| "ethereum-trading-server".to_string()),
            version: env::var("SERVER_VERSION").unwrap_or_else(|_| "0.1.0".to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            log_json_format: env::var("LOG_JSON_FORMAT")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            test_mode: env::var("TEST_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            test_balance: env::var("TEST_BALANCE")
                .unwrap_or_else(|_| "100.0".to_string())
                .parse()
                .unwrap_or(100.0),
        };

        let ethereum = EthereumConfig {
            rpc_url: env::var("ETHEREUM_RPC_URL")
                .ok()
                .filter(|s| !s.is_empty())
                .or_else(|| Some("https://eth.llamarpc.com".to_string())),
            chain_id: env::var("CHAIN_ID")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1),
            private_key: env::var("ETH_PRIVATE_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
        };

        let trading = TradingConfig {
            default_slippage_bps: env::var("DEFAULT_SLIPPAGE_BPS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50),
            gas_price_strategy: env::var("GAS_PRICE_STRATEGY")
                .unwrap_or_else(|_| "standard".to_string()),
            max_gas_limit: env::var("MAX_GAS_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500000),
        };

        let uniswap = UniswapConfig {
            v2_router: env::var("UNISWAP_V2_ROUTER")
                .unwrap_or_else(|_| "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string()),
            v3_router: env::var("UNISWAP_V3_ROUTER")
                .unwrap_or_else(|_| "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
        };

        let api_keys = ApiKeysConfig {
            alchemy_api_key: env::var("ALCHEMY_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            infura_api_key: env::var("INFURA_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            etherscan_api_key: env::var("ETHERSCAN_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
            coingecko_api_key: env::var("COINGECKO_API_KEY")
                .ok()
                .filter(|s| !s.is_empty()),
        };

        let performance = PerformanceConfig {
            http_timeout: env::var("HTTP_TIMEOUT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            max_concurrent_requests: env::var("MAX_CONCURRENT_REQUESTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            rpc_retry_count: env::var("RPC_RETRY_COUNT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            price_cache_ttl: env::var("PRICE_CACHE_TTL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
        };

        let token_registry_path = env::var("TOKEN_REGISTRY_PATH")
            .ok()
            .filter(|s| !s.is_empty());

        Ok(Config {
            server,
            ethereum,
            trading,
            uniswap,
            api_keys,
            performance,
            token_registry_path,
        })
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> anyhow::Result<()> {
        // 如果不是测试模式，需要配置 RPC URL
        if !self.server.test_mode && self.ethereum.rpc_url.is_none() {
            anyhow::bail!("非测试模式下必须配置 ETHEREUM_RPC_URL");
        }

        // 验证测试余额值
        if self.server.test_balance < 0.0 {
            anyhow::bail!("TEST_BALANCE 不能为负数");
        }

        // 验证滑点范围（0-10000 基点，即 0-100%）
        if self.trading.default_slippage_bps > 10000 {
            anyhow::bail!("DEFAULT_SLIPPAGE_BPS 不能超过 10000（100%）");
        }

        // 验证 Gas 价格策略
        let valid_strategies = ["fast", "standard", "slow"];
        if !valid_strategies.contains(&self.trading.gas_price_strategy.as_str()) {
            anyhow::bail!(
                "GAS_PRICE_STRATEGY 必须是 fast、standard 或 slow 之一"
            );
        }

        // 验证 Chain ID
        let valid_chain_ids = [1, 5, 11155111]; // 主网、Goerli、Sepolia
        if !valid_chain_ids.contains(&self.ethereum.chain_id) {
            eprintln!(
                "⚠️  警告: Chain ID {} 不在常用网络列表中",
                self.ethereum.chain_id
            );
        }

        Ok(())
    }

    /// 获取用于模拟的钱包地址
    ///
    /// 优先级：
    /// 1. 从 private_key 派生地址
    /// 2. 使用知名的高余额地址（Vitalik 地址）作为默认模拟地址
    pub fn get_simulation_address(&self) -> Address {
        // 尝试从 private_key 派生地址
        if let Some(ref key_str) = self.ethereum.private_key {
            if let Ok(wallet) = key_str.parse::<LocalWallet>() {
                return wallet.address();
            }
        }

        // 使用 Vitalik 的地址作为默认模拟地址（已知有大量余额和代币）
        // 这个地址用于只读模拟，不会实际发送交易
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
            .parse()
            .expect("硬编码地址应该有效")
    }

    /// 打印配置信息（隐藏敏感信息）
    pub fn print_info(&self) {
        eprintln!("📋 配置信息:");
        eprintln!("  服务器名称: {}", self.server.name);
        eprintln!("  服务器版本: {}", self.server.version);
        eprintln!("  日志级别: {}", self.server.log_level);
        eprintln!("  JSON 日志: {}", self.server.log_json_format);
        eprintln!("  测试模式: {}", self.server.test_mode);

        if self.server.test_mode {
            eprintln!("  测试余额: {} ETH", self.server.test_balance);
        }

        eprintln!("\n🌐 以太坊网络:");
        if let Some(ref rpc_url) = self.ethereum.rpc_url {
            // 隐藏 API Key 部分
            let masked_url = if rpc_url.contains("?") {
                rpc_url.split('?').next().unwrap_or(rpc_url).to_string() + "?***"
            } else {
                rpc_url.clone()
            };
            eprintln!("  RPC 节点: {}", masked_url);
        }
        eprintln!("  Chain ID: {}", self.ethereum.chain_id);

        if self.ethereum.private_key.is_some() {
            eprintln!("  私钥: ✅ 已配置");
        } else {
            eprintln!("  私钥: ❌ 未配置（只读模式）");
        }

        eprintln!("\n💱 交易配置:");
        eprintln!(
            "  默认滑点: {} bps ({}%)",
            self.trading.default_slippage_bps,
            self.trading.default_slippage_bps as f64 / 100.0
        );
        eprintln!("  Gas 策略: {}", self.trading.gas_price_strategy);
        eprintln!("  最大 Gas: {}", self.trading.max_gas_limit);

        eprintln!("\n🦄 Uniswap:");
        eprintln!("  V2 Router: {}", self.uniswap.v2_router);
        eprintln!("  V3 Router: {}", self.uniswap.v3_router);

        eprintln!("\n🔑 API 密钥:");
        if self.api_keys.alchemy_api_key.is_some() {
            eprintln!("  Alchemy: ✅ 已配置");
        }
        if self.api_keys.infura_api_key.is_some() {
            eprintln!("  Infura: ✅ 已配置");
        }
        if self.api_keys.etherscan_api_key.is_some() {
            eprintln!("  Etherscan: ✅ 已配置");
        }
        if self.api_keys.coingecko_api_key.is_some() {
            eprintln!("  CoinGecko: ✅ 已配置");
        }

        eprintln!("\n⚡ 性能:");
        eprintln!("  HTTP 超时: {}s", self.performance.http_timeout);
        eprintln!(
            "  并发请求: {}",
            self.performance.max_concurrent_requests
        );
        eprintln!("  RPC 重试: {}", self.performance.rpc_retry_count);
        eprintln!("  价格缓存: {}s", self.performance.price_cache_ttl);

        if let Some(ref path) = self.token_registry_path {
            eprintln!("\n📄 代币注册表: {}", path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::from_env().expect("应该能创建默认配置");
        assert_eq!(config.server.name, "ethereum-trading-server");
        assert_eq!(config.ethereum.chain_id, 1);
        assert_eq!(config.trading.default_slippage_bps, 50);
    }

    #[test]
    fn test_config_validation() {
        let config = Config::from_env().expect("应该能创建配置");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_slippage_validation() {
        let mut config = Config::from_env().expect("应该能创建配置");

        // 正常滑点
        config.trading.default_slippage_bps = 100;
        assert!(config.validate().is_ok());

        // 超范围滑点
        config.trading.default_slippage_bps = 10001;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_gas_strategy_validation() {
        let mut config = Config::from_env().expect("应该能创建配置");

        // 有效策略
        config.trading.gas_price_strategy = "fast".to_string();
        assert!(config.validate().is_ok());

        config.trading.gas_price_strategy = "standard".to_string();
        assert!(config.validate().is_ok());

        config.trading.gas_price_strategy = "slow".to_string();
        assert!(config.validate().is_ok());

        // 无效策略
        config.trading.gas_price_strategy = "invalid".to_string();
        assert!(config.validate().is_err());
    }
}
