use anyhow::{Context, Result};
use chrono::{Duration as ChronoDuration, NaiveDate, Utc};
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::config::AppConfig;

pub struct LoggingRuntime {
    _guard: WorkerGuard,
    pub log_file: PathBuf,
}

pub fn init_logging(config: &AppConfig) -> Result<LoggingRuntime> {
    std::fs::create_dir_all(&config.log_dir)
        .with_context(|| format!("failed to create log dir: {}", config.log_dir.display()))?;
    let removed = cleanup_old_logs(&config.log_dir, config.log_retention_days)?;
    if removed > 0 {
        eprintln!(
            "cleaned {removed} old log files under {}",
            config.log_dir.display()
        );
    }

    let today = Utc::now().date_naive();
    let (file_writer, guard, log_file) = create_non_blocking_writer(&config.log_dir, today)?;
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(config.log_level.clone()));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(file_writer),
        )
        .try_init()
        .context("failed to initialize tracing subscriber")?;

    Ok(LoggingRuntime {
        _guard: guard,
        log_file,
    })
}

pub fn cleanup_old_logs(log_dir: &Path, retention_days: u16) -> Result<usize> {
    cleanup_old_logs_at(log_dir, retention_days, Utc::now().date_naive())
}

pub fn cleanup_old_logs_at(log_dir: &Path, retention_days: u16, today: NaiveDate) -> Result<usize> {
    if !log_dir.exists() {
        return Ok(0);
    }

    let keep_days = retention_days.max(1) as i64;
    let cutoff = today - ChronoDuration::days(keep_days - 1);
    let mut removed = 0usize;

    for entry in std::fs::read_dir(log_dir)
        .with_context(|| format!("failed to read log dir: {}", log_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_date) = parse_dated_log_file(&path) else {
            continue;
        };
        if file_date < cutoff {
            std::fs::remove_file(&path)
                .with_context(|| format!("failed to remove old log file: {}", path.display()))?;
            removed += 1;
        }
    }

    Ok(removed)
}

fn parse_dated_log_file(path: &Path) -> Option<NaiveDate> {
    let file_name = path.file_name()?.to_string_lossy();
    let date_part = file_name.strip_suffix(".log")?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

fn create_non_blocking_writer(
    log_dir: &Path,
    today: NaiveDate,
) -> Result<(NonBlocking, WorkerGuard, PathBuf)> {
    let file_name = format!("{}.log", today.format("%Y-%m-%d"));
    let file_path = log_dir.join(&file_name);
    let appender = tracing_appender::rolling::never(log_dir, &file_name);
    let (writer, guard) = tracing_appender::non_blocking(appender);
    Ok((writer, guard, file_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use tracing_subscriber::fmt::MakeWriter;

    #[test]
    fn cleanup_old_logs_removes_files_older_than_retention_window() {
        let temp = tempdir().expect("tempdir");
        let log_dir = temp.path();

        std::fs::write(log_dir.join("2026-02-15.log"), "old").expect("write old");
        std::fs::write(log_dir.join("2026-02-18.log"), "keep").expect("write keep");
        std::fs::write(log_dir.join("README.txt"), "ignore").expect("write misc");

        let removed = cleanup_old_logs_at(
            log_dir,
            7,
            NaiveDate::from_ymd_opt(2026, 2, 24).expect("date"),
        )
        .expect("cleanup");

        assert_eq!(removed, 1);
        assert!(!log_dir.join("2026-02-15.log").exists());
        assert!(log_dir.join("2026-02-18.log").exists());
        assert!(log_dir.join("README.txt").exists());
    }

    #[test]
    fn non_blocking_writer_flushes_on_guard_drop() {
        let temp = tempdir().expect("tempdir");
        let today = NaiveDate::from_ymd_opt(2026, 2, 24).expect("date");
        let (writer, guard, log_file) =
            create_non_blocking_writer(temp.path(), today).expect("writer");

        let mut handle = writer.make_writer();
        handle.write_all(b"queued-log-line\n").expect("write log");
        handle.flush().expect("flush");
        drop(handle);
        drop(guard);

        let content = std::fs::read_to_string(log_file).expect("read log file");
        assert!(content.contains("queued-log-line"));
    }
}
