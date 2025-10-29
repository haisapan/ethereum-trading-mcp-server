use tracing::Level;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

/// 初始化日志系统
///
/// 支持多种日志格式和输出方式：
/// - 标准输出（stderr）
/// - JSON 格式（可选）
/// - 文件输出（可选）
pub fn init_logging(log_level: &str, json_format: bool) -> anyhow::Result<()> {
    // 解析日志级别
    let level = parse_log_level(log_level);

    // 构建环境过滤器
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("{}", level)))?;

    if json_format {
        // JSON 格式日志（适合生产环境）
        Registry::default()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(true)
                    .with_writer(std::io::stderr),
            )
            .init();
    } else {
        // 人类可读格式（适合开发环境）
        Registry::default()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(std::io::stderr),
            )
            .init();
    }

    tracing::info!(
        log_level = %log_level,
        json_format = %json_format,
        "日志系统初始化完成"
    );

    Ok(())
}

/// 解析日志级别字符串
fn parse_log_level(level_str: &str) -> Level {
    match level_str.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => {
            eprintln!(
                "⚠️  无效的日志级别 '{}', 使用默认值 'info'",
                level_str
            );
            Level::INFO
        }
    }
}

/// 日志宏的便捷重导出（仅暴露实际使用的宏）
pub use tracing::info;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("trace"), Level::TRACE);
        assert_eq!(parse_log_level("DEBUG"), Level::DEBUG);
        assert_eq!(parse_log_level("Info"), Level::INFO);
        assert_eq!(parse_log_level("WARN"), Level::WARN);
        assert_eq!(parse_log_level("error"), Level::ERROR);
        assert_eq!(parse_log_level("invalid"), Level::INFO);
    }
}
