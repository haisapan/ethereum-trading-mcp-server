/// 以太坊交易 MCP 工具模块
///
/// 本模块包含三个核心工具：
/// - `balance`: 查询 ETH 和 ERC20 代币余额
/// - `price`: 获取代币价格（USD 或 ETH）
/// - `swap`: 模拟 Uniswap 代币交换

pub mod balance;
pub mod price;
pub mod swap;

// 重新导出主要类型，方便外部使用
pub use balance::{get_balance, BalanceResponse, GetBalanceRequest};
pub use price::{get_token_price, GetTokenPriceRequest, TokenPriceResponse};
pub use swap::{swap_tokens, SwapResponse, SwapTokensRequest};
