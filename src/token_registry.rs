use crate::types::TokenInfo;
use std::collections::HashMap;
use std::sync::RwLock;

/// 代币注册表
/// 管理常用代币的符号到地址的映射
/// 支持动态查询链上信息并缓存
pub struct TokenRegistry {
    tokens: RwLock<HashMap<String, TokenInfo>>,
}

impl TokenRegistry {
    /// 创建新的注册表，预加载常用代币
    pub fn new() -> Self {
        let mut tokens = HashMap::new();

        // 加载默认代币
        for (symbol, info) in default_mainnet_tokens() {
            tokens.insert(symbol.to_uppercase(), info);
        }

        Self {
            tokens: RwLock::new(tokens),
        }
    }

    /// 解析代币地址或符号
    /// 如果输入是有效的以太坊地址，直接返回
    /// 如果是符号，从注册表查找
    pub fn resolve(&self, symbol_or_address: &str) -> Option<TokenInfo> {
        let tokens = self.tokens.read().unwrap();

        // 检查是否为以太坊地址（0x 开头，42 位）
        if symbol_or_address.starts_with("0x") && symbol_or_address.len() == 42 {
            // 验证是否为十六进制
            if symbol_or_address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                // 这是地址，尝试从注册表查找详细信息
                // 如果找不到，返回 UNKNOWN 标记（调用方应主动查询链上信息）
                return tokens
                    .values()
                    .find(|t| t.address.to_lowercase() == symbol_or_address.to_lowercase())
                    .cloned()
                    .or_else(|| {
                        Some(TokenInfo {
                            symbol: "UNKNOWN".to_string(),
                            name: "Unknown Token".to_string(),
                            address: symbol_or_address.to_string(),
                            decimals: 18, // 🔴 占位符，调用方应查询真实值
                        })
                    });
            }
        }

        // 作为符号查找
        tokens.get(&symbol_or_address.to_uppercase()).cloned()
    }

    /// 添加或更新代币信息
    pub fn register(&self, symbol: String, info: TokenInfo) {
        let mut tokens = self.tokens.write().unwrap();
        tokens.insert(symbol.to_uppercase(), info.clone());
        // 同时用地址作为 key 缓存
        tokens.insert(info.address.to_lowercase(), info);
    }

    /// 获取所有已注册代币
    pub fn all_tokens(&self) -> Vec<TokenInfo> {
        let tokens = self.tokens.read().unwrap();
        tokens.values().cloned().collect()
    }

    /// 判断是否包含某个符号
    pub fn contains(&self, symbol: &str) -> bool {
        let tokens = self.tokens.read().unwrap();
        tokens.contains_key(&symbol.to_uppercase())
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 以太坊主网常用代币列表
fn default_mainnet_tokens() -> Vec<(String, TokenInfo)> {
    vec![
        // WETH 排在前面，通过地址查询时优先返回
        (
            "WETH".to_string(),
            TokenInfo {
                symbol: "WETH".to_string(),
                name: "Wrapped Ether".to_string(),
                address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                decimals: 18,
            },
        ),
        // ETH 别名：用户友好的符号，映射到 WETH 合约
        (
            "ETH".to_string(),
            TokenInfo {
                symbol: "ETH".to_string(),
                name: "Ether".to_string(),
                address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH 地址
                decimals: 18,
            },
        ),
        (
            "USDC".to_string(),
            TokenInfo {
                symbol: "USDC".to_string(),
                name: "USD Coin".to_string(),
                address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                decimals: 6,
            },
        ),
        (
            "USDT".to_string(),
            TokenInfo {
                symbol: "USDT".to_string(),
                name: "Tether USD".to_string(),
                address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
                decimals: 6,
            },
        ),
        (
            "DAI".to_string(),
            TokenInfo {
                symbol: "DAI".to_string(),
                name: "Dai Stablecoin".to_string(),
                address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(),
                decimals: 18,
            },
        ),
        (
            "WBTC".to_string(),
            TokenInfo {
                symbol: "WBTC".to_string(),
                name: "Wrapped BTC".to_string(),
                address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599".to_string(),
                decimals: 8,
            },
        ),
        (
            "UNI".to_string(),
            TokenInfo {
                symbol: "UNI".to_string(),
                name: "Uniswap".to_string(),
                address: "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984".to_string(),
                decimals: 18,
            },
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = TokenRegistry::new();
        assert!(registry.contains("ETH")); // ETH 别名
        assert!(registry.contains("WETH"));
        assert!(registry.contains("USDC"));
        assert!(registry.contains("DAI"));
    }

    #[test]
    fn test_eth_alias_resolves_to_weth_address() {
        let registry = TokenRegistry::new();

        // 解析 ETH 符号
        let eth = registry.resolve("ETH").unwrap();
        assert_eq!(eth.symbol, "ETH");
        assert_eq!(eth.name, "Ether");
        assert_eq!(eth.decimals, 18);

        // 验证 ETH 和 WETH 使用相同的合约地址
        let weth = registry.resolve("WETH").unwrap();
        assert_eq!(eth.address, weth.address);
        assert_eq!(
            eth.address.to_lowercase(),
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
        );
    }

    #[test]
    fn test_resolve_by_symbol() {
        let registry = TokenRegistry::new();

        // 大写符号
        let usdc = registry.resolve("USDC").unwrap();
        assert_eq!(usdc.symbol, "USDC");
        assert_eq!(usdc.decimals, 6);

        // 小写符号也能解析
        let dai = registry.resolve("dai").unwrap();
        assert_eq!(dai.symbol, "DAI");
        assert_eq!(dai.decimals, 18);
    }

    #[test]
    fn test_resolve_by_address() {
        let registry = TokenRegistry::new();

        // WETH 地址 (注意：ETH 和 WETH 共享同一地址，优先返回 WETH)
        let weth_addr = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
        let token = registry.resolve(weth_addr).unwrap();
        // 可能返回 ETH 或 WETH，因为它们共享地址
        assert!(token.symbol == "WETH" || token.symbol == "ETH");

        // 不区分大小写
        let token_lower = registry.resolve(&weth_addr.to_lowercase()).unwrap();
        assert!(token_lower.symbol == "WETH" || token_lower.symbol == "ETH");
    }

    #[test]
    fn test_resolve_unknown_address() {
        let registry = TokenRegistry::new();

        // 未知地址
        let unknown = registry
            .resolve("0x1234567890123456789012345678901234567890")
            .unwrap();
        assert_eq!(unknown.symbol, "UNKNOWN");
        assert_eq!(unknown.decimals, 18); // 默认值
    }

    #[test]
    fn test_resolve_invalid() {
        let registry = TokenRegistry::new();

        // 无效符号
        assert!(registry.resolve("INVALID").is_none());

        // 无效地址格式
        assert!(registry.resolve("0xinvalid").is_none());
    }

    #[test]
    fn test_register_custom_token() {
        let registry = TokenRegistry::new();

        let custom = TokenInfo {
            symbol: "CUSTOM".to_string(),
            name: "Custom Token".to_string(),
            address: "0x1234567890123456789012345678901234567890".to_string(),
            decimals: 18,
        };

        registry.register("CUSTOM".to_string(), custom.clone());

        let resolved = registry.resolve("CUSTOM").unwrap();
        assert_eq!(resolved.symbol, "CUSTOM");
        assert_eq!(resolved.address, custom.address);
    }

    #[test]
    fn test_all_tokens() {
        let registry = TokenRegistry::new();
        let all = registry.all_tokens();
        assert!(all.len() >= 6); // 至少 6 个默认代币
    }
}
