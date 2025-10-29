use crate::{
    config::Config,
    erc20::{format_units, parse_units, Erc20Client},
    logging::info,
    token_registry::TokenRegistry,
    types::TokenInfo,
    uniswap::UniswapV2Client,
};
use ethers::prelude::*;
use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, ErrorData as McpError,
};
use std::sync::Arc;

/// SwapTokens 工具的参数
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SwapTokensArgs {
    /// 源代币地址或符号(必需)
    pub from_token: String,
    /// 目标代币地址或符号(必需)
    pub to_token: String,
    /// 交易数量(必需)
    pub amount: String,
    /// 滑点(基点,默认 50 = 0.5%)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u32>,
    /// 钱包地址(用于 Gas 估算,可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
}

/// SwapTokens 工具的返回结果
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SwapSimulationResult {
    pub from_token: TokenInfo,
    pub to_token: TokenInfo,
    pub input_amount: String,
    pub estimated_output: String,
    pub minimum_output: String,
    pub price_impact: String,
    pub route: SwapRoute,
    pub simulation_success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_estimate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert_reason: Option<String>,
}

/// 交换路径信息
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SwapRoute {
    pub protocol: String,
    pub path: Vec<String>,
    pub pools: Vec<String>,
}

/// 模拟代币交换(Uniswap V2)
#[tool(description = "模拟 Uniswap V2 代币交换,返回预估输出和价格影响")]
pub fn swap_tokens(
    config: &Arc<Config>,
    uniswap_client: &Arc<UniswapV2Client>,
    erc20_client: &Arc<Erc20Client>,
    token_registry: &Arc<TokenRegistry>,
    Parameters(args): Parameters<SwapTokensArgs>,
) -> Result<CallToolResult, McpError> {
    info!("收到 swap_tokens 请求");

    let slippage_bps = args.slippage_bps.unwrap_or(50); // 默认 0.5%

    // 🔒 校验滑点范围（0-10000 基点，即 0-100%）
    if slippage_bps > 10000 {
        return Err(McpError::invalid_params(
            format!(
                "滑点参数无效: {} bps (必须 ≤ 10000，即 ≤ 100%)",
                slippage_bps
            ),
            None,
        ));
    }

    info!(
        from = %args.from_token,
        to = %args.to_token,
        amount = %args.amount,
        slippage = slippage_bps,
        "模拟代币交换"
    );

    // 测试模式
    if config.server.test_mode {
        let from_token = TokenInfo {
            symbol: "FROM".to_string(),
            name: "From Token".to_string(),
            address: args.from_token.clone(),
            decimals: 18,
        };

        let to_token = TokenInfo {
            symbol: "TO".to_string(),
            name: "To Token".to_string(),
            address: args.to_token.clone(),
            decimals: 18,
        };

        let result = SwapSimulationResult {
            from_token,
            to_token,
            input_amount: args.amount.clone(),
            estimated_output: "100.0".to_string(),
            minimum_output: "99.5".to_string(),
            price_impact: "0.5%".to_string(),
            route: SwapRoute {
                protocol: "Uniswap V2".to_string(),
                path: vec![args.from_token.clone(), args.to_token.clone()],
                pools: vec!["0xtest".to_string()],
            },
            simulation_success: true,
            gas_estimate: Some("150000".to_string()),
            revert_reason: None,
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

    // 解析源代币
    let mut from_token_info = token_registry
        .resolve(&args.from_token)
        .ok_or_else(|| {
            McpError::invalid_params(format!("未知的源代币: {}", args.from_token), None)
        })?;

    let from_token_addr: Address = from_token_info.address.parse().map_err(|_| {
        McpError::internal_error("无效的源代币地址".to_string(), None)
    })?;

    // 🔍 动态查询未知源代币信息
    if from_token_info.symbol == "UNKNOWN" && erc20_client.is_available() {
        let erc20_client_clone = erc20_client.clone();
        let real_info = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                erc20_client_clone.token_info(from_token_addr).await
            })
        })
        .map_err(|e| McpError::internal_error(format!("查询源代币信息失败: {}", e), None))?;

        // 缓存到注册表
        token_registry.register(real_info.symbol.clone(), real_info.clone());
        from_token_info = real_info;
    }

    // 解析目标代币
    let mut to_token_info = token_registry
        .resolve(&args.to_token)
        .ok_or_else(|| {
            McpError::invalid_params(format!("未知的目标代币: {}", args.to_token), None)
        })?;

    let to_token_addr: Address = to_token_info.address.parse().map_err(|_| {
        McpError::internal_error("无效的目标代币地址".to_string(), None)
    })?;

    // 🔍 动态查询未知目标代币信息
    if to_token_info.symbol == "UNKNOWN" && erc20_client.is_available() {
        let erc20_client_clone = erc20_client.clone();
        let real_info = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                erc20_client_clone.token_info(to_token_addr).await
            })
        })
        .map_err(|e| McpError::internal_error(format!("查询目标代币信息失败: {}", e), None))?;

        // 缓存到注册表
        token_registry.register(real_info.symbol.clone(), real_info.clone());
        to_token_info = real_info;
    }

    // 解析输入金额（使用 rust_decimal 保持精度）
    let amount_in = parse_units(&args.amount, from_token_info.decimals).map_err(|e| {
        McpError::invalid_params(format!("解析金额失败: {}", e), None)
    })?;

    // 计算最小输出(考虑滑点)
    let slippage_factor = 10000 - slippage_bps; // 9950 for 0.5% slippage

    // 解析钱包地址（用于模拟）
    let wallet_addr = if let Some(ref addr_str) = args.wallet_address {
        addr_str.parse::<Address>().map_err(|_| {
            McpError::invalid_params(format!("无效的钱包地址: {}", addr_str), None)
        })?
    } else {
        // 使用配置的模拟地址（从 private_key 派生或使用默认地址）
        config.get_simulation_address()
    };

    let uniswap_client = uniswap_client.clone();

    // 使用 simulate_swap 进行真实的 Router 模拟
    let simulation = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            // 首先计算最小输出（我们需要先获取报价）
            let quote = uniswap_client
                .quote_swap(from_token_addr, to_token_addr, amount_in)
                .await
                .map_err(|e| McpError::internal_error(format!("查询交换报价失败: {}", e), None))?;

            let minimum_output = quote.amount_out * U256::from(slippage_factor) / U256::from(10000);

            // 进行真实的 Router 模拟
            uniswap_client
                .simulate_swap(from_token_addr, to_token_addr, amount_in, minimum_output, Some(wallet_addr))
                .await
                .map_err(|e| McpError::internal_error(format!("模拟交换失败: {}", e), None))
        })
    })?;

    let quote = &simulation.quote;

    // 计算最小输出
    let minimum_output = quote.amount_out * U256::from(slippage_factor) / U256::from(10000);

    // 格式化输出
    let estimated_output_formatted = format_units(quote.amount_out, to_token_info.decimals);
    let minimum_output_formatted = format_units(minimum_output, to_token_info.decimals);

    // 构建路径字符串
    let path_strings: Vec<String> = quote
        .path
        .iter()
        .map(|addr| format!("{:?}", addr))
        .collect();

    // 🚀 使用缓存的 pair 地址，避免重复 RPC 调用
    let pool_addresses: Vec<String> = quote
        .pair_addresses
        .iter()
        .map(|addr| format!("{:?}", addr))
        .collect();

    let result = SwapSimulationResult {
        from_token: from_token_info,
        to_token: to_token_info,
        input_amount: args.amount,
        estimated_output: estimated_output_formatted,
        minimum_output: minimum_output_formatted,
        price_impact: format!("{:.2}%", quote.price_impact),
        route: SwapRoute {
            protocol: "Uniswap V2".to_string(),
            path: path_strings,
            pools: pool_addresses,
        },
        simulation_success: simulation.simulation_success,
        gas_estimate: simulation.gas_estimate.map(|g| g.to_string()),
        revert_reason: simulation.revert_reason,
    };

    let json_str = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    info!("成功返回交换模拟结果");

    Ok(CallToolResult::success(vec![Content::text(json_str)]))
}
