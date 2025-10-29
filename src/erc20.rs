use crate::types::TokenInfo;
use ethers::prelude::*;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, instrument};

/// ERC20 代币错误类型
#[derive(Debug, thiserror::Error)]
pub enum Erc20Error {
    #[error("提供者错误: {0}")]
    ProviderError(#[from] ProviderError),

    #[error("ABI 编码/解码错误: {0}")]
    AbiError(String),

    #[error("Provider 不可用")]
    ProviderUnavailable,
}

/// ERC20 客户端
#[derive(Clone)]
pub struct Erc20Client {
    provider: Option<Arc<Provider<Http>>>,
}

impl Erc20Client {
    /// 创建新的 ERC20 客户端
    pub fn new(provider: Option<Arc<Provider<Http>>>) -> Self {
        Self { provider }
    }

    /// 检查客户端是否可用
    pub fn is_available(&self) -> bool {
        self.provider.is_some()
    }

    /// 查询 ERC20 代币余额
    #[instrument(skip(self))]
    pub async fn balance_of(
        &self,
        token: Address,
        owner: Address,
    ) -> Result<U256, Erc20Error> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(Erc20Error::ProviderUnavailable)?;

        debug!(
            token_address = %token,
            owner_address = %owner,
            "查询 ERC20 余额"
        );

        // 构建 balanceOf(address) 调用数据
        // function selector: 0x70a08231
        let mut data = vec![0x70, 0xa0, 0x82, 0x31];
        // owner 地址（32 字节，左填充 0）
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(owner.as_bytes());

        let tx = Eip1559TransactionRequest::new()
            .to(token)
            .data(Bytes::from(data));

        let result = provider.call(&tx.into(), None).await?;

        // 解析返回值（uint256）
        if result.len() != 32 {
            return Err(Erc20Error::AbiError(format!(
                "期望 32 字节返回值，实际 {} 字节",
                result.len()
            )));
        }

        Ok(U256::from_big_endian(&result))
    }

    /// 查询代币符号（symbol）
    #[instrument(skip(self))]
    pub async fn symbol(&self, token: Address) -> Result<String, Erc20Error> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(Erc20Error::ProviderUnavailable)?;

        // function selector: symbol() = 0x95d89b41
        let data = vec![0x95, 0xd8, 0x9b, 0x41];

        let tx = Eip1559TransactionRequest::new()
            .to(token)
            .data(Bytes::from(data));

        let result = provider.call(&tx.into(), None).await?;

        // 解析字符串返回值
        parse_string_return(&result).ok_or_else(|| {
            Erc20Error::AbiError("无法解析 symbol 返回值".to_string())
        })
    }

    /// 查询代币名称（name）
    #[instrument(skip(self))]
    pub async fn name(&self, token: Address) -> Result<String, Erc20Error> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(Erc20Error::ProviderUnavailable)?;

        // function selector: name() = 0x06fdde03
        let data = vec![0x06, 0xfd, 0xde, 0x03];

        let tx = Eip1559TransactionRequest::new()
            .to(token)
            .data(Bytes::from(data));

        let result = provider.call(&tx.into(), None).await?;

        parse_string_return(&result).ok_or_else(|| {
            Erc20Error::AbiError("无法解析 name 返回值".to_string())
        })
    }

    /// 查询代币小数位数（decimals）
    #[instrument(skip(self))]
    pub async fn decimals(&self, token: Address) -> Result<u8, Erc20Error> {
        let provider = self
            .provider
            .as_ref()
            .ok_or(Erc20Error::ProviderUnavailable)?;

        // function selector: decimals() = 0x313ce567
        let data = vec![0x31, 0x3c, 0xe5, 0x67];

        let tx = Eip1559TransactionRequest::new()
            .to(token)
            .data(Bytes::from(data));

        let result = provider.call(&tx.into(), None).await?;

        if result.is_empty() {
            return Err(Erc20Error::AbiError("空返回值".to_string()));
        }

        // decimals 通常返回 uint8，但某些合约返回 uint256
        if result.len() == 32 {
            let value = U256::from_big_endian(&result);
            Ok(value.as_u32() as u8)
        } else if result.len() == 1 {
            Ok(result[0])
        } else {
            Err(Erc20Error::AbiError(format!(
                "意外的 decimals 返回值长度: {}",
                result.len()
            )))
        }
    }

    /// 查询完整代币信息
    #[instrument(skip(self))]
    pub async fn token_info(&self, token: Address) -> Result<TokenInfo, Erc20Error> {
        debug!(token_address = %token, "查询代币信息");

        // 并发查询三个字段
        let (symbol_res, name_res, decimals_res) = tokio::join!(
            self.symbol(token),
            self.name(token),
            self.decimals(token)
        );

        // 使用默认值处理错误（有些代币可能没有实现全部接口）
        let symbol = symbol_res.unwrap_or_else(|_| "UNKNOWN".to_string());
        let name = name_res.unwrap_or_else(|_| "Unknown Token".to_string());
        let decimals = decimals_res.unwrap_or(18); // 默认 18 位

        Ok(TokenInfo {
            symbol,
            name,
            address: format!("{:?}", token),
            decimals,
        })
    }
}

/// 解析 ABI 编码的字符串返回值
fn parse_string_return(data: &[u8]) -> Option<String> {
    if data.len() < 64 {
        return None;
    }

    // ABI 字符串编码：
    // 前 32 字节：offset (通常是 32)
    // 接下来 32 字节：length
    // 剩余：实际数据

    let offset = U256::from_big_endian(&data[0..32]).as_usize();
    if offset >= data.len() {
        return None;
    }

    let length = U256::from_big_endian(&data[offset..offset + 32]).as_usize();
    if offset + 32 + length > data.len() {
        return None;
    }

    let string_data = &data[offset + 32..offset + 32 + length];
    String::from_utf8(string_data.to_vec()).ok()
}

/// 格式化代币金额
pub fn format_units(amount: U256, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }

    let divisor = U256::from(10).pow(U256::from(decimals));
    let integer_part = amount / divisor;
    let fractional_part = amount % divisor;

    if fractional_part.is_zero() {
        integer_part.to_string()
    } else {
        // 格式化小数部分，移除尾部的 0
        let frac_str = format!("{:0width$}", fractional_part, width = decimals as usize);
        let frac_trimmed = frac_str.trim_end_matches('0');
        if frac_trimmed.is_empty() {
            integer_part.to_string()
        } else {
            format!("{}.{}", integer_part, frac_trimmed)
        }
    }
}

/// 解析代币金额（使用 Decimal 保持精度）
pub fn parse_units(amount_str: &str, decimals: u8) -> Result<U256, String> {
    // 使用 rust_decimal 解析，保持完整精度
    let decimal = Decimal::from_str(amount_str)
        .map_err(|e| format!("无法解析金额 '{}': {}", amount_str, e))?;

    // 检查是否为负数
    if decimal.is_sign_negative() {
        return Err("金额不能为负数".to_string());
    }

    // 将 Decimal 转换为字符串，然后手动处理小数点
    let decimal_str = decimal.to_string();

    // 分离整数和小数部分
    let (integer_part, fractional_part) = if let Some(dot_pos) = decimal_str.find('.') {
        let (int_part, frac_part_with_dot) = decimal_str.split_at(dot_pos);
        let frac_part = &frac_part_with_dot[1..]; // 跳过小数点
        (int_part, frac_part)
    } else {
        (decimal_str.as_str(), "")
    };

    // 检查精度是否超过代币支持的精度
    if fractional_part.len() > decimals as usize {
        return Err(format!(
            "金额 '{}' 的精度超过了代币支持的 {} 位小数",
            amount_str, decimals
        ));
    }

    // 构建最终的字符串：整数部分 + 小数部分 + 补齐的0
    let padding_zeros = decimals as usize - fractional_part.len();
    let final_str = format!("{}{}{}", integer_part, fractional_part, "0".repeat(padding_zeros));

    // 解析为 U256
    U256::from_dec_str(&final_str)
        .map_err(|e| format!("金额过大，无法转换: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_units() {
        // 18 位小数（ETH）
        let amount = U256::from(1_000_000_000_000_000_000u64); // 1 ETH
        assert_eq!(format_units(amount, 18), "1");

        // 带小数
        let amount = U256::from(1_500_000_000_000_000_000u64); // 1.5 ETH
        assert_eq!(format_units(amount, 18), "1.5");

        // 6 位小数（USDC）
        let amount = U256::from(1_000_000u64); // 1 USDC
        assert_eq!(format_units(amount, 6), "1");

        // 8 位小数（WBTC）
        let amount = U256::from(100_000_000u64); // 1 WBTC
        assert_eq!(format_units(amount, 8), "1");

        // 0 小数
        let amount = U256::from(42u64);
        assert_eq!(format_units(amount, 0), "42");
    }

    #[test]
    fn test_parse_units() {
        // 18 位小数（ETH）
        let result = parse_units("1", 18).unwrap();
        assert_eq!(result, U256::from(1_000_000_000_000_000_000u64));

        // 带小数
        let result = parse_units("1.5", 18).unwrap();
        assert_eq!(result, U256::from(1_500_000_000_000_000_000u64));

        // 6 位小数（USDC）
        let result = parse_units("1", 6).unwrap();
        assert_eq!(result, U256::from(1_000_000u64));

        // 大额（超过 u64 范围但不溢出 Decimal）
        let result = parse_units("1000000", 18).unwrap();
        assert_eq!(result, U256::from_dec_str("1000000000000000000000000").unwrap());

        // 0 小数
        let result = parse_units("42", 0).unwrap();
        assert_eq!(result, U256::from(42u64));

        // 高精度小数
        let result = parse_units("0.000000000000000001", 18).unwrap();
        assert_eq!(result, U256::from(1u64));
    }

    #[test]
    fn test_parse_units_errors() {
        // 负数
        assert!(parse_units("-1", 18).is_err());

        // 精度过高
        assert!(parse_units("0.0000000000000000001", 18).is_err());

        // 无效格式
        assert!(parse_units("abc", 18).is_err());
    }

    #[test]
    fn test_parse_units_high_decimals() {
        // 测试 20 位小数（超过 u64 的 10^19）
        let result = parse_units("1", 20).unwrap();
        assert_eq!(result, U256::from_dec_str("100000000000000000000").unwrap());

        // 测试 30 位小数
        let result = parse_units("1", 30).unwrap();
        assert_eq!(result, U256::from_dec_str("1000000000000000000000000000000").unwrap());

        // 测试 50 位小数（极端情况）
        let result = parse_units("1", 50).unwrap();
        assert_eq!(result, U256::from_dec_str("100000000000000000000000000000000000000000000000000").unwrap());

        // 测试带小数的高精度
        let result = parse_units("1.5", 20).unwrap();
        assert_eq!(result, U256::from_dec_str("150000000000000000000").unwrap());
    }

    #[tokio::test]
    async fn test_symbol_without_provider_returns_error() {
        let client = Erc20Client::new(None);
        let result = client.symbol(Address::zero()).await;
        assert!(matches!(result, Err(Erc20Error::ProviderUnavailable)));
    }

    #[tokio::test]
    async fn test_name_without_provider_returns_error() {
        let client = Erc20Client::new(None);
        let result = client.name(Address::zero()).await;
        assert!(matches!(result, Err(Erc20Error::ProviderUnavailable)));
    }

    #[tokio::test]
    async fn test_decimals_without_provider_returns_error() {
        let client = Erc20Client::new(None);
        let result = client.decimals(Address::zero()).await;
        assert!(matches!(result, Err(Erc20Error::ProviderUnavailable)));
    }

    #[tokio::test]
    async fn test_token_info_without_provider_uses_defaults() {
        let client = Erc20Client::new(None);
        let info = client.token_info(Address::zero()).await.expect("应当返回默认信息");
        assert_eq!(info.symbol, "UNKNOWN");
        assert_eq!(info.decimals, 18);
    }

    #[test]
    fn test_parse_format_roundtrip() {
        // 往返测试
        let original = "123.456789";
        let parsed = parse_units(original, 18).unwrap();
        let formatted = format_units(parsed, 18);
        assert_eq!(formatted, original);
    }

    #[test]
    fn test_parse_string_return() {
        // 模拟 ABI 编码的字符串 "USDC"
        let mut data = vec![0u8; 96];
        // offset = 32
        data[31] = 32;
        // length = 4
        data[63] = 4;
        // string data: "USDC"
        data[64..68].copy_from_slice(b"USDC");

        let result = parse_string_return(&data);
        assert_eq!(result, Some("USDC".to_string()));
    }

    #[tokio::test]
    async fn test_erc20_client_without_provider() {
        let client = Erc20Client::new(None);
        assert!(!client.is_available());

        let token = Address::zero();
        let owner = Address::zero();

        let result = client.balance_of(token, owner).await;
        assert!(result.is_err());
    }
}
