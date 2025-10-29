use ethers::prelude::*;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, warn};

/// Ethereum 客户端错误类型
#[derive(Debug, thiserror::Error)]
pub enum EthClientError {
    #[error("提供者错误: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("无效的地址: {0}")]
    InvalidAddress(String),

    #[error("RPC URL 未配置")]
    NoRpcUrl,

    #[error("连接超时")]
    Timeout,

    #[error("其他错误: {0}")]
    Other(String),
}

/// Ethereum RPC 客户端
#[derive(Clone)]
pub struct EthClient {
    provider: Option<Arc<Provider<Http>>>,
}

impl EthClient {
    /// 创建新的 Ethereum 客户端
    ///
    /// # 参数
    /// - `rpc_url`: RPC 节点地址（可选）
    /// - `network_id`: 网络 ID（可选）
    #[instrument(skip(rpc_url))]
    pub async fn new(rpc_url: Option<&str>, network_id: Option<u64>) -> anyhow::Result<Self> {
        let provider = if let Some(url) = rpc_url {
            info!(rpc_url = %url, "初始化 Ethereum 客户端");

            match Provider::<Http>::try_from(url) {
                Ok(provider) => {
                    // 测试连接
                    match provider.get_chainid().await {
                        Ok(chain_id) => {
                            let chain_id_u64 = chain_id.as_u64();
                            if let Some(expected) = network_id {
                                if expected != chain_id_u64 {
                                    warn!(
                                        expected = expected,
                                        actual = chain_id_u64,
                                        "提供的 Chain ID 与节点返回值不一致"
                                    );
                                }
                            }

                            info!(
                                chain_id = %chain_id_u64,
                                "成功连接到 Ethereum 节点"
                            );
                            Some(Arc::new(provider))
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                "无法连接到 Ethereum 节点，将在测试模式下运行"
                            );
                            None
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "创建 Provider 失败");
                    None
                }
            }
        } else {
            debug!("未配置 RPC URL，客户端将不可用");
            None
        };

        Ok(Self { provider })
    }

    /// 检查客户端是否可用
    pub fn is_available(&self) -> bool {
        self.provider.is_some()
    }

    /// 获取地址余额（返回 Wei 格式的 U256）
    ///
    /// # 参数
    /// - `address`: 以太坊地址字符串
    ///
    /// # 返回
    /// 余额（以 Wei 为单位的 U256）
    #[instrument(skip(self))]
    pub async fn get_balance(&self, address: &str) -> Result<U256, EthClientError> {
        // 检查客户端是否可用
        let provider = self
            .provider
            .as_ref()
            .ok_or(EthClientError::NoRpcUrl)?;

        debug!(address = %address, "查询地址余额");

        // 解析地址
        let addr: Address = address
            .parse()
            .map_err(|_| EthClientError::InvalidAddress(address.to_string()))?;

        // 查询余额
        let balance_wei = provider.get_balance(addr, None).await?;

        info!(
            address = %address,
            balance_wei = %balance_wei,
            "成功查询余额"
        );

        Ok(balance_wei)
    }

    /// 获取当前区块号
    #[instrument(skip(self))]
    pub async fn get_block_number(&self) -> Result<u64, EthClientError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(EthClientError::NoRpcUrl)?;

        let block_number = provider.get_block_number().await?;

        debug!(block_number = %block_number, "获取当前区块号");

        Ok(block_number.as_u64())
    }

    /// 获取链 ID
    #[instrument(skip(self))]
    pub async fn get_chain_id(&self) -> Result<u64, EthClientError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(EthClientError::NoRpcUrl)?;

        let chain_id = provider.get_chainid().await?;

        debug!(chain_id = %chain_id, "获取链 ID");

        Ok(chain_id.as_u64())
    }

    /// 获取网络 Gas 价格
    #[instrument(skip(self))]
    pub async fn get_gas_price(&self) -> Result<f64, EthClientError> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(EthClientError::NoRpcUrl)?;

        let gas_price_wei = provider.get_gas_price().await?;
        let gas_price_gwei = wei_to_gwei(gas_price_wei);

        debug!(gas_price_gwei = %gas_price_gwei, "获取 Gas 价格");

        Ok(gas_price_gwei)
    }
}

/// 将 Wei 转换为 ETH
fn wei_to_eth(wei: U256) -> f64 {
    let eth_decimals = U256::from(10).pow(U256::from(18));
    let eth_value = wei.as_u128() as f64 / eth_decimals.as_u128() as f64;
    eth_value
}

/// 将 Wei 转换为 Gwei
fn wei_to_gwei(wei: U256) -> f64 {
    let gwei_decimals = U256::from(10).pow(U256::from(9));
    let gwei_value = wei.as_u128() as f64 / gwei_decimals.as_u128() as f64;
    gwei_value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wei_to_eth() {
        // 1 ETH = 10^18 Wei
        let one_eth = U256::from(10).pow(U256::from(18));
        assert_eq!(wei_to_eth(one_eth), 1.0);

        // 0.5 ETH
        let half_eth = U256::from(5) * U256::from(10).pow(U256::from(17));
        assert_eq!(wei_to_eth(half_eth), 0.5);

        // 0 ETH
        assert_eq!(wei_to_eth(U256::zero()), 0.0);
    }

    #[test]
    fn test_wei_to_gwei() {
        // 1 Gwei = 10^9 Wei
        let one_gwei = U256::from(10).pow(U256::from(9));
        assert_eq!(wei_to_gwei(one_gwei), 1.0);

        // 50 Gwei
        let fifty_gwei = U256::from(50) * U256::from(10).pow(U256::from(9));
        assert_eq!(wei_to_gwei(fifty_gwei), 50.0);
    }

    #[tokio::test]
    async fn test_eth_client_without_provider() {
        let client = EthClient::new(None, None).await.unwrap();
        assert!(!client.is_available());

        let result = client.get_balance("0x0").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_block_number_without_provider() {
        let client = EthClient::new(None, None).await.unwrap();
        assert!(client.get_block_number().await.is_err());
    }

    #[tokio::test]
    async fn test_get_chain_id_without_provider() {
        let client = EthClient::new(None, None).await.unwrap();
        assert!(client.get_chain_id().await.is_err());
    }

    #[tokio::test]
    async fn test_get_gas_price_without_provider() {
        let client = EthClient::new(None, None).await.unwrap();
        assert!(client.get_gas_price().await.is_err());
    }

    #[test]
    fn test_eth_client_error_variants_display() {
        assert_eq!(EthClientError::Timeout.to_string(), "连接超时");
        assert_eq!(
            EthClientError::Other("oops".to_string()).to_string(),
            "其他错误: oops"
        );
    }
}
