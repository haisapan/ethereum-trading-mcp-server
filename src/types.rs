use serde::{Deserialize, Serialize};

/// 代币信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub symbol: String,
    pub name: String,
    pub address: String,
    pub decimals: u8,
}

/// Gas 估算信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    pub gas_limit: u64,
    pub gas_price_gwei: String,
    pub total_cost_eth: String,
}

/// 交换路径信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapRoute {
    pub protocol: String,
    pub path: Vec<String>,
    pub pools: Vec<String>,
}

impl TokenInfo {
    /// 创建 ETH 代币信息
    pub fn eth() -> Self {
        Self {
            symbol: "ETH".to_string(),
            name: "Ether".to_string(),
            address: "0x0000000000000000000000000000000000000000".to_string(),
            decimals: 18,
        }
    }

    /// 判断是否为 ETH
    pub fn is_eth(&self) -> bool {
        self.address == "0x0000000000000000000000000000000000000000"
            || self.symbol == "ETH"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_token_info() {
        let eth = TokenInfo::eth();
        assert_eq!(eth.symbol, "ETH");
        assert_eq!(eth.decimals, 18);
        assert!(eth.is_eth());
    }

    #[test]
    fn test_token_info_serialization() {
        let token = TokenInfo {
            symbol: "USDC".to_string(),
            name: "USD Coin".to_string(),
            address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
            decimals: 6,
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("USDC"));
        assert!(json.contains("USD Coin"));

        let deserialized: TokenInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.symbol, token.symbol);
        assert_eq!(deserialized.decimals, token.decimals);
    }
}
