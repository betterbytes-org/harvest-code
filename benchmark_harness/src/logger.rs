/// A simple logger that tees to stdout and a file
use log::{set_boxed_logger, LevelFilter, Log, Metadata, Record, SetLoggerError};
use std::fs::File;
use std::io::{stdout, Write};
use std::sync::Mutex;

/// The TeeLogger struct. Takes a file and prints to the file and stdout
pub struct TeeLogger {
    // level: LevelFilter,
    file: Mutex<File>,
    // output_lock: Mutex<()>,
}

impl TeeLogger {
    /// Globally initializes the TeeLogger as the one and only used logger.
    pub fn init(log_level: LevelFilter, file: File) -> Result<(), SetLoggerError> {
        // set_max_level(log_level);
        set_boxed_logger(TeeLogger::new(log_level, file))
    }

    #[must_use]
    pub fn new(_log_level: LevelFilter, file: File) -> Box<TeeLogger> {
        Box::new(TeeLogger {
            file: Mutex::new(file),
        })
    }
}

impl Log for TeeLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
        // metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let stdout = stdout();
            let mut stdout_lock = stdout.lock();
            let mut file_lock = self.file.lock().unwrap();
            writeln!(stdout_lock, "{}", record.args()).unwrap();
            writeln!(file_lock, "{}", record.args()).unwrap();

            self.flush();
        }
    }

    fn flush(&self) {
        use std::io::Write;
        let _ = stdout().flush();
        // let _ = self.file.flush();
        let _ = self.file.lock().unwrap().flush();
    }
}
