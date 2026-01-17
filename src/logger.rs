//! Logging configuration

use std::sync::LazyLock;
use tracing_subscriber::{
    fmt, fmt::time::ChronoLocal, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

static HOSTNAME: LazyLock<String> = LazyLock::new(|| {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
});

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

#[allow(dead_code)]
pub fn init_logger() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            fmt::layer()
                .with_timer(ChronoLocal::rfc_3339())
                .with_target(false)
                .with_level(true)
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .compact(),
        )
        .init();

    tracing::info!("Logger initialized");
}

pub fn init_logger_detailed() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
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
                        .compact(),
                ),
        )
        .init();

    let memory = get_memory_usage();
    tracing::info!(
        hostname = %*HOSTNAME,
        memory = %memory,
        "Logger initialized (detailed format)"
    );
}

#[cfg(feature = "json")]
pub fn init_logger_json() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
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

#[cfg(test)]
pub fn init_test_logger() {
    use tracing_subscriber::fmt::TestWriter;

    let _ = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("error")))
        .with(
            fmt::layer()
                .with_writer(TestWriter::default())
                .with_target(false)
                .with_ansi(false)
                .compact(),
        )
        .try_init();
}

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

pub fn get_hostname() -> &'static str {
    &*HOSTNAME
}

pub fn get_memory_usage_public() -> String {
    get_memory_usage()
}
