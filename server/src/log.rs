//! Logging to stderr (shown in Zed's language server log) and, when
//! `COMMENT_TRANSLATE_LOG_FILE` is set, to that file.

use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};

static FILE: OnceLock<Option<Mutex<File>>> = OnceLock::new();

pub fn init() {
    let file = std::env::var_os("COMMENT_TRANSLATE_LOG_FILE")
        .and_then(|path| OpenOptions::new().create(true).append(true).open(path).ok());
    let _ = FILE.set(file.map(Mutex::new));
}

pub fn write(message: std::fmt::Arguments) {
    eprintln!("[comment-translate] {message}");
    if let Some(Some(file)) = FILE.get() {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let _ = writeln!(
            file.lock().unwrap(),
            "{timestamp:.3} [{}] {message}",
            std::process::id()
        );
    }
}

macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(format_args!($($arg)*))
    };
}
