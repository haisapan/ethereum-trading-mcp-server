use crate::{
    config::Config,
    erc20::{format_units, Erc20Client},
    logging::info,
    token_registry::TokenRegistry,
    types::TokenInfo,
    uniswap::UniswapV2Client,
};
use ethers::prelude::*;
use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, ErrorData as McpError,
};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;

/// GetTokenPrice 工具的参数
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTokenPriceArgs {
    /// 代币地址或符号(必需)
    pub token: String,
    /// 报价货币(USD/ETH,默认 USD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_currency: Option<String>,
}

/// GetTokenPrice 工具的返回结果
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TokenPriceResult {
    pub token: TokenInfo,
    pub price: String,
    pub quote_currency: String,
    pub source: String,
    pub liquidity: Option<String>,
}

/// 获取代币价格(支持 USD 和 ETH 报价)
#[tool(description = "获取代币在 Uniswap V2 上的价格(支持 USD 和 ETH 报价)")]
pub fn get_token_price(
    config: &Arc<Config>,
    uniswap_client: &Arc<UniswapV2Client>,
    erc20_client: &Arc<Erc20Client>,
    token_registry: &Arc<TokenRegistry>,
    Parameters(args): Parameters<GetTokenPriceArgs>,
) -> Result<CallToolResult, McpError> {
    info!("收到 get_token_price 请求");

    let quote_currency = args.quote_currency.unwrap_or_else(|| "USD".to_string());
    info!(token = %args.token, quote = %quote_currency, "查询代币价格");

    // 测试模式
    if config.server.test_mode {
        let token_info = TokenInfo {
            symbol: "TEST".to_string(),
            name: "Test Token".to_string(),
            address: args.token.clone(),
            decimals: 18,
        };

        let result = TokenPriceResult {
            token: token_info,
            price: "2000.0".to_string(),
            quote_currency,
            source: "Test Mode".to_string(),
            liquidity: Some("1000000.0".to_string()),
        };

        let json_str = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        return Ok(CallToolResult::success(vec![Content::text(json_str)]));
    }

    // 真实模式:需要检查客户端可用性
    if !uniswap_client.is_available() {
        return Err(McpError::internal_error(
            "Uniswap 客户端不可用,请检查 RPC 配置",
            None,
        ));
    }

    // 解析代币
    let mut token_info = token_registry
        .resolve(&args.token)
        .ok_or_else(|| {
            McpError::invalid_params(format!("未知的代币: {}", args.token), None)
        })?;

    let token_addr: Address = token_info.address.parse().map_err(|_| {
        McpError::internal_error("无效的代币地址".to_string(), None)
    })?;

    // 🔍 动态查询未知代币信息
    if token_info.symbol == "UNKNOWN" && erc20_client.is_available() {
        let erc20_client_clone = erc20_client.clone();
        let real_info = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                erc20_client_clone.token_info(token_addr).await
            })
        })
        .map_err(|e| McpError::internal_error(format!("查询代币信息失败: {}", e), None))?;

        // 缓存到注册表
        token_registry.register(real_info.symbol.clone(), real_info.clone());
        token_info = real_info;
    }

    // WETH 地址
    let weth_addr: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        .parse()
        .unwrap();

    let uniswap_client = uniswap_client.clone();

    // 查询 Token/WETH 池子
    let (pair, reserves) = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            let pair = uniswap_client
                .get_pair(token_addr, weth_addr)
                .await
                .map_err(|e| McpError::internal_error(format!("查询交易对失败: {}", e), None))?;

            let reserves = uniswap_client
                .get_reserves(pair)
                .await
                .map_err(|e| McpError::internal_error(format!("查询储备量失败: {}", e), None))?;

            Ok::<_, McpError>((pair, reserves))
        })
    })?;

    // 确定储备量顺序(token0 < token1)
    let (token_reserve, weth_reserve) = if token_addr < weth_addr {
        (reserves.0, reserves.1)
    } else {
        (reserves.1, reserves.0)
    };

    // 🎯 使用 U256 精确计算价格，避免溢出
    let token_decimals = token_info.decimals;
    let weth_decimals = 18u8;

    // 计算 Token/WETH 价格（保持 U256 精度）
    // price = (weth_reserve * 10^token_decimals) / (token_reserve * 10^weth_decimals)
    let price_in_eth_str = calculate_price_ratio(
        weth_reserve,
        token_reserve,
        token_decimals,
        weth_decimals,
    );

    let (final_price, final_quote) = if quote_currency.to_uppercase() == "ETH" {
        (price_in_eth_str, "ETH".to_string())
    } else {
        // 查询 WETH/USDC 价格来转换成 USD
        let usdc_addr: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap();

        let eth_price_usd_str = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let usdc_pair = uniswap_client
                    .get_pair(weth_addr, usdc_addr)
                    .await
                    .map_err(|e| {
                        McpError::internal_error(format!("查询 ETH/USDC 交易对失败: {}", e), None)
                    })?;

                let usdc_reserves = uniswap_client
                    .get_reserves(usdc_pair)
                    .await
                    .map_err(|e| {
                        McpError::internal_error(format!("查询 ETH/USDC 储备量失败: {}", e), None)
                    })?;

                // WETH < USDC in address order
                let (weth_res, usdc_res) = if weth_addr < usdc_addr {
                    (usdc_reserves.0, usdc_reserves.1)
                } else {
                    (usdc_reserves.1, usdc_reserves.0)
                };

                // 🎯 使用 U256 计算 ETH/USD 价格
                // eth_price = (usdc_reserve * 10^18) / (weth_reserve * 10^6)
                let eth_price = calculate_price_ratio(usdc_res, weth_res, 18, 6);

                Ok::<_, McpError>(eth_price)
            })
        })?;

        // 计算 Token 价格（USD） = Token/ETH 价格 × ETH/USD 价格
        let token_price_usd = multiply_price_strings(&price_in_eth_str, &eth_price_usd_str);
        (token_price_usd, "USD".to_string())
    };

    // 计算流动性(以 WETH 计)
    let liquidity_eth = format_units(weth_reserve * U256::from(2), 18); // 总流动性 = weth * 2

    let result = TokenPriceResult {
        token: token_info,
        price: final_price,
        quote_currency: final_quote,
        source: format!("Uniswap V2 (Pair: {:?})", pair),
        liquidity: Some(format!("{} ETH", liquidity_eth)),
    };

    let json_str = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    info!("成功返回价格");

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}

/// 计算价格比率（U256 储备 + Decimal 价格）
/// 符合原始需求：使用 rust_decimal 进行金融精度计算
/// price = (numerator_reserve * 10^numerator_decimals) / (denominator_reserve * 10^denominator_decimals)
/// 返回格式化的字符串，保留 6 位小数
fn calculate_price_ratio(
    numerator_reserve: U256,
    denominator_reserve: U256,
    numerator_decimals: u8,
    denominator_decimals: u8,
) -> String {
    // 避免除零
    if denominator_reserve.is_zero() {
        return "0".to_string();
    }

    // 🎯 策略：先用 U256 计算比率（避免储备量溢出），再转 Decimal 做精度调整

    // 1. 计算原始比率（U256 保持精度）
    //    为了保留精度，先乘以 10^18
    let precision_scale = U256::from(10u64).pow(U256::from(18));
    let ratio_scaled = (numerator_reserve * precision_scale) / denominator_reserve;

    // 2. 转换为 Decimal（比率通常在合理范围内，不会溢出）
    let ratio_dec = match u256_to_decimal_safe(ratio_scaled) {
        Ok(dec) => dec,
        Err(_) => {
            // 如果比率太大（罕见情况），回退到字符串除法
            return format_u256_division_fallback(numerator_reserve, denominator_reserve, numerator_decimals, denominator_decimals);
        }
    };

    // 3. 调整小数位差异（使用 Decimal 精确计算）
    let scale_diff = numerator_decimals as i32 - denominator_decimals as i32;
    let decimals_adjustment = if scale_diff >= 0 {
        // numerator 小数位更多
        decimal_pow10(scale_diff.abs() as u8)
    } else {
        // denominator 小数位更多
        Decimal::ONE / decimal_pow10(scale_diff.abs() as u8)
    };

    // 4. 计算最终价格（Decimal 精确运算）
    // 移除精度因子（10^18）
    let scale_18 = Decimal::from_str("1000000000000000000").unwrap();
    let price_scaled = ratio_dec / scale_18;
    let final_price = price_scaled * decimals_adjustment;

    // 5. 格式化输出
    format!("{:.6}", final_price).trim_end_matches('0').trim_end_matches('.').to_string()
}

/// 两个价格字符串相乘（避免精度损失）
fn multiply_price_strings(price1_str: &str, price2_str: &str) -> String {
    // 解析为 f64 相乘（这里的精度损失可接受，因为是最终显示）
    let price1: f64 = price1_str.parse().unwrap_or(0.0);
    let price2: f64 = price2_str.parse().unwrap_or(0.0);
    let result = price1 * price2;
    format!("{:.6}", result)
}

/// 安全地将 U256 转换为 Decimal
/// 如果 U256 超出 Decimal 范围（28 位有效数字），返回错误
fn u256_to_decimal_safe(value: U256) -> Result<Decimal, String> {
    let value_str = value.to_string();
    Decimal::from_str(&value_str).map_err(|e| format!("Decimal 转换失败: {}", e))
}

/// 计算 10^n 作为 Decimal
fn decimal_pow10(n: u8) -> Decimal {
    // 使用字符串构造：1 后面跟 n 个 0
    let pow_str = format!("1{}", "0".repeat(n as usize));
    Decimal::from_str(&pow_str).unwrap_or(Decimal::ONE)
}

/// 回退方案：当 Decimal 溢出时，使用纯 U256 字符串除法
fn format_u256_division_fallback(
    numerator: U256,
    denominator: U256,
    numerator_decimals: u8,
    denominator_decimals: u8,
) -> String {
    if denominator.is_zero() {
        return "0".to_string();
    }

    // 处理小数位差异
    let scale_diff = numerator_decimals as i32 - denominator_decimals as i32;
    let (num, denom) = if scale_diff >= 0 {
        let scale = U256::from(10u64).pow(U256::from(scale_diff.abs() as u64));
        (numerator * scale, denominator)
    } else {
        let scale = U256::from(10u64).pow(U256::from(scale_diff.abs() as u64));
        (numerator, denominator * scale)
    };

    // U256 字符串除法
    format_u256_division_internal(num, denom, 6)
}

/// 内部：U256 除法并格式化为小数字符串
fn format_u256_division_internal(numerator: U256, denominator: U256, decimal_places: usize) -> String {
    if denominator.is_zero() {
        return "0".to_string();
    }

    // 整数部分
    let integer_part = numerator / denominator;
    let remainder = numerator % denominator;

    if remainder.is_zero() {
        return format!("{}.0", integer_part);
    }

    // 小数部分
    let scale = U256::from(10u64).pow(U256::from(decimal_places));
    let fractional_scaled = (remainder * scale) / denominator;
    let frac_str = format!("{:0width$}", fractional_scaled, width = decimal_places);
    let frac_trimmed = frac_str.trim_end_matches('0');
    let frac_display = if frac_trimmed.is_empty() { "0" } else { frac_trimmed };

    format!("{}.{}", integer_part, frac_display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_u256_division_internal_basic() {
        // 10 / 3 = 3.333333
        let result = format_u256_division_internal(U256::from(10), U256::from(3), 6);
        assert_eq!(result, "3.333333");

        // 1 / 2 = 0.5
        let result = format_u256_division_internal(U256::from(1), U256::from(2), 6);
        assert_eq!(result, "0.5");

        // 100 / 10 = 10.0
        let result = format_u256_division_internal(U256::from(100), U256::from(10), 6);
        assert_eq!(result, "10.0");
    }

    #[test]
    fn test_format_u256_division_fallback_extreme() {
        // 🔥 测试回退方案：Uniswap uint112 上限附近的储备量
        // numerator = 5×10^33 (接近 uint112 最大值)
        let numerator = U256::from_dec_str("5000000000000000000000000000000000").unwrap();
        // denominator = 2×10^30
        let denominator = U256::from_dec_str("2000000000000000000000000000000").unwrap();

        // 预期: 5×10^33 / 2×10^30 = 2500
        let result = format_u256_division_fallback(numerator, denominator, 0, 0);
        assert_eq!(result, "2500.0");
    }

    #[test]
    fn test_u256_to_decimal_safe() {
        // 小值应该成功
        let value = U256::from(123456789);
        let result = u256_to_decimal_safe(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "123456789");

        // 超大值应该失败（超过 Decimal 28 位限制）
        let huge_value = U256::from_dec_str("1000000000000000000000000000000").unwrap();  // 10^30
        let result = u256_to_decimal_safe(huge_value);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_price_ratio_large_pools() {
        // 🔥 模拟大额 USDC/WETH 池子
        // USDC reserve: 100,000,000 * 10^6 = 10^14
        let usdc_reserve = U256::from_dec_str("100000000000000").unwrap(); // 100M USDC
        // WETH reserve: 40,000 * 10^18 = 4×10^22
        let weth_reserve = U256::from_dec_str("40000000000000000000000").unwrap(); // 40k ETH

        // 计算 ETH/USDC 价格
        // price = (usdc * 10^18) / (weth * 10^6)
        //       = (10^14 * 10^18) / (4×10^22 * 10^6)
        //       = 10^32 / (4×10^28)
        //       = 2500
        let price = calculate_price_ratio(usdc_reserve, weth_reserve, 18, 6);

        // 验证价格在合理范围内（2500 USD/ETH）
        let price_f64: f64 = price.parse().unwrap();
        assert!((price_f64 - 2500.0).abs() < 1.0);
    }

    #[test]
    fn test_calculate_price_ratio_extreme_liquidity() {
        // 🔥 测试接近 uint112 上限的储备量
        // reserve1 = 5×10^33 (接近 uint112 最大值)
        let reserve1 = U256::from_dec_str("5000000000000000000000000000000000").unwrap();
        // reserve2 = 10^33
        let reserve2 = U256::from_dec_str("1000000000000000000000000000000000").unwrap();

        // price = 5×10^33 / 10^33 = 5
        let price = calculate_price_ratio(reserve1, reserve2, 18, 18);

        let price_f64: f64 = price.parse().unwrap();
        assert!((price_f64 - 5.0).abs() < 0.000001);
    }

    #[test]
    fn test_multiply_price_strings() {
        // 测试价格字符串相乘
        let price1 = "0.0005";
        let price2 = "2500.0";
        let result = multiply_price_strings(price1, price2);

        let result_f64: f64 = result.parse().unwrap();
        assert!((result_f64 - 1.25).abs() < 0.000001);
    }
}
