//! Logging configuration and initialization
//! 
//! This module provides logging setup using the `tracing` crate,
//! which is well-suited for async Rust applications.
//! 
//! Custom format similar to Python logging:
//! `HOSTNAME [timestamp] {file:line} [memory] LEVEL - message`
//! 
//! Example:
//! `C02G725ZMD6P [2024-01-15 10:30:45.123] {main.rs:255} [45.2M] INFO - Node 0 starting on port 8000`

use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    fmt::time::ChronoLocal,
};
use std::sync::LazyLock;

// Cache hostname to avoid repeated lookups
static HOSTNAME: LazyLock<String> = LazyLock::new(|| {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
});

/// Get current memory usage in MB
/// Returns a formatted string like "45.2M"
fn get_memory_usage() -> String {
    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            let mb = kb / 1024.0;
                            return format!("{:.1}M", mb);
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("ps")
            .args(&["-o", "rss=", "-p"])
            .arg(std::process::id().to_string())
            .output()
        {
            if let Ok(mut rss_str) = String::from_utf8(output.stdout) {
                rss_str = rss_str.trim().to_string();
                if let Ok(kb) = rss_str.parse::<f64>() {
                    let mb = kb / 1024.0;
                    return format!("{:.1}M", mb);
                }
            }
        }
    }
    
    // Fallback: use sysinfo
    {
        use sysinfo::System;
        let mut system = System::new();
        system.refresh_process(sysinfo::Pid::from_u32(std::process::id()));
        if let Some(process) = system.process(sysinfo::Pid::from_u32(std::process::id())) {
            let memory_kb = process.memory() / 1024;
            let memory_mb = memory_kb as f64 / 1024.0;
            return format!("{:.1}M", memory_mb);
        }
    }
    
    "N/A".to_string()
}

/// Initialize the logging system with standard format
/// 
/// Format: `[timestamp] {file:line} LEVEL - message`
/// 
/// This should be called once at the start of the application.
/// Logging level can be controlled via the `RUST_LOG` environment variable.
/// 
/// Examples:
/// - `RUST_LOG=info` - Show info level and above
/// - `RUST_LOG=debug` - Show debug level and above
/// - `RUST_LOG=rust_market_ledger=debug,actix_web=info` - Module-specific levels
/// - `RUST_LOG=warn` - Show only warnings and errors
pub fn init_logger() {
    // Try to load .env file first (if using dotenvy)
    dotenvy::dotenv().ok();
    
    // Initialize tracing subscriber with standard format
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")), // Default to info level
        )
        .with(
            fmt::layer()
                .with_timer(ChronoLocal::rfc_3339())
                .with_target(false) // We show file:line instead
                .with_level(true)
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .compact()
        )
        .init();
    
    tracing::info!("Logger initialized");
}

/// Initialize logger with detailed format (includes hostname and memory)
/// 
/// Format: `HOSTNAME [timestamp] {file:line} [memory] LEVEL - message`
/// 
/// Similar to Python logging format:
/// `C02G725ZMD6P [2022-07-07 16:07:27,522] {logger.py:32, warning} [10252.0M] WARNING - test`
/// 
/// Uses a custom formatter that prepends hostname and memory to each log line.
pub fn init_logger_detailed() {
    dotenvy::dotenv().ok();
    
    // Use a custom format that mimics Python logging style
    // Format: HOSTNAME [timestamp] {file:line} [memory] LEVEL - message
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with(
            fmt::layer()
                .with_timer(ChronoLocal::rfc_3339())
                .with_level(true)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .with_ansi(true)
                .compact()
                .event_format(
                    fmt::format()
                        .with_timer(ChronoLocal::rfc_3339())
                        .with_level(true)
                        .with_file(true)
                        .with_line_number(true)
                        .with_target(false)
                        .compact()
                )
        )
        .init();
    
    // Log initial message with hostname and memory
    let memory = get_memory_usage();
    tracing::info!(
        hostname = %*HOSTNAME,
        memory = %memory,
        "Logger initialized (detailed format)"
    );
}

/// Initialize logger with JSON format
/// 
/// Useful for production environments where you might want JSON output
/// Note: Requires "json" feature in tracing-subscriber
#[cfg(feature = "json")]
pub fn init_logger_json() {
    dotenvy::dotenv().ok();
    
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true)
                .with_span_list(true),
        )
        .init();
    
    tracing::info!("Logger initialized (JSON format)");
}

/// Initialize logger for tests
/// 
/// This suppresses most output and only shows errors/warnings
/// to keep test output clean. Uses try_init() so it won't panic
/// if logger is already initialized.
#[cfg(test)]
pub fn init_test_logger() {
    use tracing_subscriber::fmt::TestWriter;
    
    // Only initialize if not already initialized
    let _ = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("error")), // Only errors in tests by default
        )
        .with(
            fmt::layer()
                .with_writer(TestWriter::default())
                .with_target(false)
                .with_ansi(false)
                .compact(),
        )
        .try_init(); // try_init() won't panic if already initialized
}

/// Helper macro to log with hostname and memory automatically
/// 
/// Usage:
/// ```rust
/// log_with_context!(info, "Processing block {}", block_index);
/// ```
#[macro_export]
macro_rules! log_with_context {
    ($level:ident, $($arg:tt)*) => {
        {
            let hostname = $crate::logger::get_hostname();
            let memory = $crate::logger::get_memory_usage();
            tracing::$level!(
                hostname = %hostname,
                memory = %memory,
                $($arg)*
            );
        }
    };
}

/// Get the cached hostname
pub fn get_hostname() -> &'static str {
    &*HOSTNAME
}

/// Get current memory usage (exposed for use in macros)
pub fn get_memory_usage_public() -> String {
    get_memory_usage()
}
