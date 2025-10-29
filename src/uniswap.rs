use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Uniswap 错误类型
#[derive(Debug, thiserror::Error)]
pub enum UniswapError {
    #[error("提供者错误: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("未找到交易对")]
    PairNotFound,

    #[error("流动性不足")]
    InsufficientLiquidity,

    #[error("ABI 编码/解码错误: {0}")]
    AbiError(String),

    #[error("Provider 不可用")]
    ProviderUnavailable,

    #[error("无效的数量")]
    InvalidAmount,

    #[error("其他错误: {0}")]
    Other(String),
}

/// Uniswap V2 客户端
#[derive(Clone)]
pub struct UniswapV2Client {
    provider: Option<Arc<Provider<Http>>>,
    factory_address: Address,
    router_address: Address,
}

impl UniswapV2Client {
    /// 创建新的 Uniswap V2 客户端（主网地址）
    pub fn new(provider: Option<Arc<Provider<Http>>>) -> Self {
        Self {
            provider,
            // Uniswap V2 Factory
            factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
                .parse()
                .unwrap(),
            // Uniswap V2 Router02
            router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
                .parse()
                .unwrap(),
        }
    }

    /// 检查客户端是否可用
    pub fn is_available(&self) -> bool {
        self.provider.is_some()
    }

    /// 获取交易对地址
    /// getPair(address tokenA, address tokenB) -> address pair
    #[instrument(skip(self))]
    pub async fn get_pair(
        &self,
        token_a: Address,
        token_b: Address,
    ) -> Result<Address, UniswapError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(UniswapError::ProviderUnavailable)?;

        debug!(
            token_a = %token_a,
            token_b = %token_b,
            factory = %self.factory_address,
            "查询 Uniswap V2 交易对"
        );

        // getPair(address,address) selector: 0xe6a43905
        let mut data = vec![0xe6, 0xa4, 0x39, 0x05];
        // tokenA (32 bytes)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(token_a.as_bytes());
        // tokenB (32 bytes)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(token_b.as_bytes());

        let tx = Eip1559TransactionRequest::new()
            .to(self.factory_address)
            .data(Bytes::from(data));

        let result = provider.call(&tx.into(), None).await?;

        if result.len() != 32 {
            return Err(UniswapError::AbiError(format!(
                "期望 32 字节返回值，实际 {} 字节",
                result.len()
            )));
        }

        let pair_address = Address::from_slice(&result[12..32]);

        // 检查是否为零地址（表示交易对不存在）
        if pair_address == Address::zero() {
            return Err(UniswapError::PairNotFound);
        }

        debug!(pair_address = %pair_address, "找到交易对");
        Ok(pair_address)
    }

    /// 获取储备量
    /// getReserves() -> (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
    #[instrument(skip(self))]
    pub async fn get_reserves(&self, pair: Address) -> Result<(U256, U256), UniswapError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(UniswapError::ProviderUnavailable)?;

        debug!(pair_address = %pair, "查询储备量");

        // getReserves() selector: 0x0902f1ac
        let data = vec![0x09, 0x02, 0xf1, 0xac];

        let tx = Eip1559TransactionRequest::new()
            .to(pair)
            .data(Bytes::from(data));

        let result = provider.call(&tx.into(), None).await?;

        if result.len() < 64 {
            return Err(UniswapError::AbiError(format!(
                "期望至少 64 字节返回值，实际 {} 字节",
                result.len()
            )));
        }

        // reserve0 (uint112, 但存储在 32 字节中)
        let reserve0 = U256::from_big_endian(&result[0..32]);
        // reserve1 (uint112, 但存储在 32 字节中)
        let reserve1 = U256::from_big_endian(&result[32..64]);

        // 检查流动性
        if reserve0.is_zero() || reserve1.is_zero() {
            return Err(UniswapError::InsufficientLiquidity);
        }

        debug!(
            reserve0 = %reserve0,
            reserve1 = %reserve1,
            "获取到储备量"
        );

        Ok((reserve0, reserve1))
    }

    /// 计算输出数量（含 0.3% 手续费）
    /// 使用 Uniswap V2 公式: amountOut = (amountIn * 997 * reserveOut) / (reserveIn * 1000 + amountIn * 997)
    pub fn calculate_amount_out(
        &self,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256, UniswapError> {
        if amount_in.is_zero() {
            return Err(UniswapError::InvalidAmount);
        }

        if reserve_in.is_zero() || reserve_out.is_zero() {
            return Err(UniswapError::InsufficientLiquidity);
        }

        let amount_in_with_fee = amount_in
            .checked_mul(U256::from(997))
            .ok_or(UniswapError::InvalidAmount)?;
        let numerator = amount_in_with_fee
            .checked_mul(reserve_out)
            .ok_or(UniswapError::InvalidAmount)?;
        let denominator = reserve_in
            .checked_mul(U256::from(1000))
            .ok_or(UniswapError::InvalidAmount)?
            .checked_add(amount_in_with_fee)
            .ok_or(UniswapError::InvalidAmount)?;

        Ok(numerator / denominator)
    }

    /// 计算价格影响（百分比）
    /// 使用 checked_mul 避免溢出
    pub fn calculate_price_impact(
        &self,
        amount_in: U256,
        reserve_in: U256,
    ) -> Result<f64, UniswapError> {
        if reserve_in.is_zero() {
            return Err(UniswapError::InsufficientLiquidity);
        }

        // 🔒 使用 checked_mul 防止溢出
        // impact = (amount_in * 10000) / reserve_in  (保留 2 位小数的百分比)
        let impact_scaled = amount_in
            .checked_mul(U256::from(10000))
            .ok_or(UniswapError::InvalidAmount)?
            / reserve_in;

        // 转换为 f64 显示（除以 100 得到百分比）
        let impact_u128 = impact_scaled.as_u128();
        Ok((impact_u128 as f64) / 100.0)
    }

    /// 获取路径对应的储备量和 pair 地址
    /// 返回 (Vec<(reserve_in, reserve_out)>, Vec<pair_addresses>)
    #[instrument(skip(self))]
    pub async fn get_reserves_for_path(
        &self,
        path: &[Address],
    ) -> Result<(Vec<(U256, U256)>, Vec<Address>), UniswapError> {
        if path.len() < 2 {
            return Err(UniswapError::AbiError(
                "路径至少需要 2 个代币".to_string(),
            ));
        }

        let mut reserves = Vec::new();
        let mut pair_addresses = Vec::new();

        for i in 0..path.len() - 1 {
            let token_a = path[i];
            let token_b = path[i + 1];

            // 获取交易对
            let pair = self.get_pair(token_a, token_b).await?;
            pair_addresses.push(pair);

            // 获取储备量
            let (reserve0, reserve1) = self.get_reserves(pair).await?;

            // Uniswap V2 按地址排序确定 token0/token1
            // token0 < token1 (按地址字典序)
            let (reserve_in, reserve_out) = if token_a < token_b {
                (reserve0, reserve1)
            } else {
                (reserve1, reserve0)
            };

            reserves.push((reserve_in, reserve_out));
        }

        Ok((reserves, pair_addresses))
    }

    /// 计算路径的输出数量
    pub fn calculate_amounts_out(
        &self,
        amount_in: U256,
        reserves: &[(U256, U256)],
    ) -> Result<Vec<U256>, UniswapError> {
        let mut amounts = vec![amount_in];

        for (reserve_in, reserve_out) in reserves {
            let amount_out = self.calculate_amount_out(*amounts.last().unwrap(), *reserve_in, *reserve_out)?;
            amounts.push(amount_out);
        }

        Ok(amounts)
    }

    /// 计算交换的详细信息（用于价格查询和交换模拟）
    #[instrument(skip(self))]
    pub async fn quote_swap(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<SwapQuote, UniswapError> {
        // 构建路径（直接或通过 WETH）
        let weth: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            .parse()
            .unwrap();

        let path = if token_in == weth || token_out == weth {
            // 直接路径
            vec![token_in, token_out]
        } else {
            // 通过 WETH
            vec![token_in, weth, token_out]
        };

        debug!(path_length = path.len(), "构建交换路径");

        // 获取所有储备量和 pair 地址
        let (reserves, pair_addresses) = self.get_reserves_for_path(&path).await?;

        // 计算所有中间输出
        let amounts = self.calculate_amounts_out(amount_in, &reserves)?;

        let amount_out = *amounts.last().unwrap();

        // 计算价格影响（使用第一个池子）
        let (reserve_in, _) = reserves[0];
        let price_impact = self.calculate_price_impact(amount_in, reserve_in)?;

        Ok(SwapQuote {
            path,
            amount_out,
            price_impact,
            pair_addresses,
        })
    }

    /// 获取 Router 地址
    pub fn router_address(&self) -> Address {
        self.router_address
    }

    /// 模拟真实的 Router 交易
    /// 使用 eth_call 调用 swapExactTokensForTokens 进行模拟
    #[instrument(skip(self))]
    pub async fn simulate_swap(
        &self,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        amount_out_min: U256,
        from_address: Option<Address>,
    ) -> Result<SwapSimulation, UniswapError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(UniswapError::ProviderUnavailable)?;

        // 首先获取报价
        let quote = self.quote_swap(token_in, token_out, amount_in).await?;

        // 构建路径（直接或通过 WETH）
        let weth: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            .parse()
            .unwrap();

        let path = if token_in == weth || token_out == weth {
            vec![token_in, token_out]
        } else {
            vec![token_in, weth, token_out]
        };

        // 构建 swapExactTokensForTokens calldata
        // function swapExactTokensForTokens(
        //   uint amountIn,
        //   uint amountOutMin,
        //   address[] calldata path,
        //   address to,
        //   uint deadline
        // ) external returns (uint[] memory amounts);
        // selector: 0x38ed1739

        let mut data = vec![0x38, 0xed, 0x17, 0x39];

        // amountIn (uint256)
        let mut amount_in_bytes = [0u8; 32];
        amount_in.to_big_endian(&mut amount_in_bytes);
        data.extend_from_slice(&amount_in_bytes);

        // amountOutMin (uint256)
        let mut amount_out_min_bytes = [0u8; 32];
        amount_out_min.to_big_endian(&mut amount_out_min_bytes);
        data.extend_from_slice(&amount_out_min_bytes);

        // path offset (uint256) - 0xa0 (160)
        data.extend_from_slice(&[0u8; 31]);
        data.push(0xa0);

        // to (address) - 使用提供的地址（不应该是零地址）
        let to_addr = from_address.ok_or_else(|| {
            UniswapError::Other("需要提供有效的钱包地址进行模拟".to_string())
        })?;
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to_addr.as_bytes());

        // deadline (uint256) - 使用一个很大的值
        data.extend_from_slice(&[0xff; 32]);

        // path 数组
        // length
        let mut path_len_bytes = [0u8; 32];
        U256::from(path.len()).to_big_endian(&mut path_len_bytes);
        data.extend_from_slice(&path_len_bytes);

        // path 元素
        for addr in &path {
            data.extend_from_slice(&[0u8; 12]);
            data.extend_from_slice(addr.as_bytes());
        }

        // 构建交易请求
        let tx = Eip1559TransactionRequest::new()
            .to(self.router_address())
            .from(to_addr)
            .data(Bytes::from(data.clone()));

        // 尝试模拟调用
        let (simulation_success, revert_reason, gas_estimate) = match provider.call(&tx.clone().into(), None).await {
            Ok(_) => {
                // 调用成功，尝试估算 gas
                let gas = match provider.estimate_gas(&tx.into(), None).await {
                    Ok(g) => Some(g),
                    Err(e) => {
                        debug!(error = %e, "Gas 估算失败");
                        None
                    }
                };
                (true, None, gas)
            }
            Err(e) => {
                // 调用失败，提取 revert 原因
                let reason = extract_revert_reason(&e);
                debug!(error = %e, reason = ?reason, "交易模拟失败");
                (false, reason, None)
            }
        };

        Ok(SwapSimulation {
            quote,
            gas_estimate,
            simulation_success,
            revert_reason,
        })
    }
}

/// 从 ProviderError 中提取 revert 原因
fn extract_revert_reason(error: &ProviderError) -> Option<String> {
    // 尝试从错误消息中提取 revert 原因
    let error_msg = error.to_string();

    // 查找常见的 revert 模式
    if error_msg.contains("execution reverted") {
        Some(error_msg)
    } else if error_msg.contains("insufficient") {
        Some("Insufficient liquidity or balance".to_string())
    } else if error_msg.contains("TRANSFER_FROM_FAILED") {
        Some("Token transfer failed (check allowance)".to_string())
    } else if error_msg.contains("EXPIRED") {
        Some("Transaction deadline expired".to_string())
    } else {
        Some(error_msg)
    }
}

/// 交换报价结果
#[derive(Debug, Clone)]
pub struct SwapQuote {
    pub path: Vec<Address>,
    pub amount_out: U256,
    pub price_impact: f64,
    pub pair_addresses: Vec<Address>, // 🆕 缓存 pair 地址，避免重复查询
}

/// 交易模拟结果
#[derive(Debug, Clone)]
pub struct SwapSimulation {
    pub quote: SwapQuote,
    pub gas_estimate: Option<U256>,
    pub simulation_success: bool,
    pub revert_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_amount_out() {
        let client = UniswapV2Client::new(None);

        // 示例：1 ETH 换 USDC
        // reserve_in = 100 ETH, reserve_out = 200000 USDC
        let amount_in = U256::from(1_000_000_000_000_000_000u64); // 1 ETH
        let reserve_in = U256::from(100u64) * U256::exp10(18); // 100 ETH
        let reserve_out = U256::from(200_000_000_000u64); // 200000 USDC (6 decimals)

        let amount_out = client
            .calculate_amount_out(amount_in, reserve_in, reserve_out)
            .unwrap();

        // 预期：约 1988 USDC (含 0.3% 手续费)
        // 公式：(1e18 * 997 * 200000e6) / (100e18 * 1000 + 1e18 * 997)
        //     = (1 * 997 * 200000) / (100 * 1000 + 1 * 997)
        //     = 199400000 / 100997
        //     = 1974.089...
        assert!(amount_out > U256::from(1_974_000_000u64)); // > 1974 USDC
        assert!(amount_out < U256::from(1_975_000_000u64)); // < 1975 USDC
    }

    #[test]
    fn test_calculate_price_impact() {
        let client = UniswapV2Client::new(None);

        // 1 ETH in 100 ETH reserve = 1% impact
        let amount_in = U256::from(1_000_000_000_000_000_000u64);
        let reserve_in = U256::from(100u64) * U256::exp10(18);

        let impact = client.calculate_price_impact(amount_in, reserve_in).unwrap();

        assert!((impact - 1.0).abs() < 0.001); // 约 1%
    }

    #[test]
    fn test_calculate_amount_out_with_fee() {
        let client = UniswapV2Client::new(None);

        // 测试 0.3% 手续费
        let amount_in = U256::from(1000);
        let reserve_in = U256::from(10000);
        let reserve_out = U256::from(10000);

        let amount_out = client
            .calculate_amount_out(amount_in, reserve_in, reserve_out)
            .unwrap();

        // 无手续费：1000 * 10000 / (10000 + 1000) = 909.09
        // 含手续费：(1000 * 997 * 10000) / (10000 * 1000 + 1000 * 997) = 906.61
        assert_eq!(amount_out, U256::from(906));
    }

    #[test]
    fn test_calculate_amount_out_zero_amount() {
        let client = UniswapV2Client::new(None);

        let result = client.calculate_amount_out(
            U256::zero(),
            U256::from(1000),
            U256::from(1000),
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UniswapError::InvalidAmount));
    }

    #[test]
    fn test_calculate_amount_out_zero_reserves() {
        let client = UniswapV2Client::new(None);

        let result = client.calculate_amount_out(
            U256::from(100),
            U256::zero(),
            U256::from(1000),
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UniswapError::InsufficientLiquidity
        ));
    }

    #[test]
    fn test_calculate_amounts_out_multi_hop() {
        let client = UniswapV2Client::new(None);

        // 两跳交换：Token A -> WETH -> Token B
        let amount_in = U256::from(1000);
        let reserves = vec![
            (U256::from(10000), U256::from(5000)),  // A -> WETH
            (U256::from(5000), U256::from(20000)),  // WETH -> B
        ];

        let amounts = client.calculate_amounts_out(amount_in, &reserves).unwrap();

        assert_eq!(amounts.len(), 3); // [amount_in, intermediate, amount_out]
        assert_eq!(amounts[0], U256::from(1000));

        // 第一跳：(1000 * 997 * 5000) / (10000 * 1000 + 1000 * 997) = 453
        assert_eq!(amounts[1], U256::from(453));

        // 第二跳：(453 * 997 * 20000) / (5000 * 1000 + 453 * 997) = 1656
        assert_eq!(amounts[2], U256::from(1656));
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = UniswapV2Client::new(None);

        assert!(!client.is_available());

        // 验证主网地址
        assert_eq!(
            client.factory_address,
            "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(
            client.router_address,
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
                .parse::<Address>()
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_pair_without_provider() {
        let client = UniswapV2Client::new(None);

        let result = client.get_pair(Address::zero(), Address::zero()).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UniswapError::ProviderUnavailable
        ));
    }

    #[tokio::test]
    async fn test_get_reserves_without_provider() {
        let client = UniswapV2Client::new(None);

        let result = client.get_reserves(Address::zero()).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            UniswapError::ProviderUnavailable
        ));
    }
}
