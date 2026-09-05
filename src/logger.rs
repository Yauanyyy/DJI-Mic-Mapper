use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_SIZE: u64 = 1_048_576;
const MAX_LOG_FILES: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(format!(
                "invalid log_level '{value}'; expected off/error/warn/info/debug/trace"
            )),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}

struct Logger {
    directory: PathBuf,
    level: Level,
    file: Option<File>,
}

impl Logger {
    fn new(directory: PathBuf, level: Level) -> Self {
        let mut logger = Self {
            directory,
            level,
            file: None,
        };
        logger.open_if_needed();
        logger
    }

    fn open_if_needed(&mut self) {
        if self.level == Level::Off || self.file.is_some() {
            return;
        }
        if fs::create_dir_all(&self.directory).is_ok() {
            self.file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.directory.join("dji-mic-mapper.log"))
                .ok();
        }
    }

    fn set_level(&mut self, level: Level) {
        self.level = level;
        if level == Level::Off {
            self.file = None;
        } else {
            self.open_if_needed();
        }
    }

    fn rotate_if_needed(&mut self) {
        let path = self.directory.join("dji-mic-mapper.log");
        let should_rotate = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .is_some_and(|metadata| metadata.len() >= MAX_LOG_SIZE);
        if !should_rotate {
            return;
        }

        self.file = None;
        let oldest = self
            .directory
            .join(format!("dji-mic-mapper.log.{MAX_LOG_FILES}"));
        let _ = fs::remove_file(oldest);
        for index in (1..MAX_LOG_FILES).rev() {
            let source = self.directory.join(format!("dji-mic-mapper.log.{index}"));
            let destination = self
                .directory
                .join(format!("dji-mic-mapper.log.{}", index + 1));
            let _ = fs::rename(source, destination);
        }
        let _ = fs::rename(path, self.directory.join("dji-mic-mapper.log.1"));
        self.open_if_needed();
    }

    fn write(&mut self, level: Level, message: &str) {
        if self.level == Level::Off || level > self.level {
            return;
        }
        self.rotate_if_needed();
        self.open_if_needed();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "{timestamp} [{}] {message}", level.label());
            let _ = file.flush();
        }
    }
}

static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

pub fn init(base_directory: &Path, level: Level) {
    let directory = base_directory.join("logs");
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut logger) = logger.lock() {
            logger.directory = directory;
            logger.set_level(level);
        }
        return;
    }
    let _ = LOGGER.set(Mutex::new(Logger::new(directory, level)));
}

pub fn set_level(level: Level) {
    if let Some(logger) = LOGGER.get()
        && let Ok(mut logger) = logger.lock()
    {
        logger.set_level(level);
    }
}

pub fn log(level: Level, message: &str) {
    if let Some(logger) = LOGGER.get()
        && let Ok(mut logger) = logger.lock()
    {
        logger.write(level, message);
    }
}

pub fn log_args(level: Level, arguments: fmt::Arguments<'_>) {
    log(level, &arguments.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_levels() {
        for name in ["off", "error", "warn", "info", "debug", "trace"] {
            assert!(Level::parse(name).is_ok());
        }
        assert!(Level::parse("verbose").is_err());
    }
}
