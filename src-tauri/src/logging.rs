use chrono::Local;
use directories::ProjectDirs;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// 全局日志启用状态
static LOGGING_ENABLED: AtomicBool = AtomicBool::new(true);

/// 获取日志文件路径
pub fn log_file_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "speaky", "Speaky").map(|dirs| {
        dirs.data_dir().join("speaky.log")
    })
}

/// 设置日志启用状态
pub fn set_logging_enabled(enabled: bool) {
    LOGGING_ENABLED.store(enabled, Ordering::SeqCst);
}

/// 检查日志是否启用
pub fn is_logging_enabled() -> bool {
    LOGGING_ENABLED.load(Ordering::SeqCst)
}

/// 写入一条日志
pub fn write_log(level: &str, message: &str) {
    if !is_logging_enabled() {
        return;
    }

    if let Some(path) = log_file_path() {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // 追加写入日志
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
            let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, message);
        }
    }
}

/// 读取日志内容（最后 N 行）
pub fn read_logs(max_lines: usize) -> Result<Vec<String>, String> {
    let path = log_file_path().ok_or("Failed to get log file path")?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path).map_err(|e| format!("Failed to open log file: {}", e))?;
    let reader = BufReader::new(file);

    // 读取所有行
    let lines: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .collect();

    // 返回最后 max_lines 行
    let start = if lines.len() > max_lines {
        lines.len() - max_lines
    } else {
        0
    };

    Ok(lines[start..].to_vec())
}

/// 获取日志文件大小（字节）
pub fn log_file_size() -> u64 {
    log_file_path()
        .and_then(|p| fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0)
}

/// 清空日志文件
pub fn clear_logs() -> Result<(), String> {
    let path = log_file_path().ok_or("Failed to get log file path")?;

    if path.exists() {
        fs::write(&path, "").map_err(|e| format!("Failed to clear log file: {}", e))?;
    }

    Ok(())
}

/// 自定义日志写入器，同时输出到 stderr 和文件
pub struct FileLogger;

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = record.level().as_str();
            let message = format!("{}", record.args());

            // 输出到 stderr
            eprintln!("[{}] [{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S"), level, message);

            // 写入文件
            write_log(level, &message);
        }
    }

    fn flush(&self) {}
}

/// 初始化日志系统
pub fn init_logger(enable_file_logging: bool) {
    set_logging_enabled(enable_file_logging);

    static LOGGER: FileLogger = FileLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Info);
}
