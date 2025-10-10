use log::LevelFilter;
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};
use std::error::Error;

/// Initialize the logging system with a console-only handler.
///
/// This sets up log4rs with a console appender that outputs to stderr.
/// The log format includes timestamp, level, target, and message.
///
/// # Arguments
/// * `level` - The minimum log level to display (e.g., LevelFilter::Info)
///
/// # Returns
/// * `Ok(())` - If logging was successfully initialized
/// * `Err(Box<dyn Error>)` - If initialization failed
///
/// # Examples
/// ```rust
/// use log::LevelFilter;
///
/// // Initialize with Info level logging
/// init_logger(LevelFilter::Info).expect("Failed to initialize logger");
///
/// // Now you can use logging macros
/// log::info!("Application started");
/// log::debug!("This won't be shown with Info level");
/// ```
pub fn init_logger(level: LevelFilter) -> Result<(), Box<dyn Error>> {
    let console_appender = ConsoleAppender::builder()
        .target(Target::Stderr)
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {t} - {m}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console_appender)))
        .build(Root::builder().appender("console").build(level))?;

    log4rs::init_config(config)?;
    Ok(())
}

/// Initialize logging with default settings (Info level).
///
/// This is a convenience function that calls `init_logger` with `LevelFilter::Info`.
///
/// # Returns
/// * `Ok(())` - If logging was successfully initialized
/// * `Err(Box<dyn Error>)` - If initialization failed
#[allow(dead_code)]
pub fn init_default_logger() -> Result<(), Box<dyn Error>> {
    init_logger(LevelFilter::Info)
}

/// Initialize logging for development (Debug level).
///
/// This is a convenience function for development that enables debug-level logging.
///
/// # Returns
/// * `Ok(())` - If logging was successfully initialized  
/// * `Err(Box<dyn Error>)` - If initialization failed
#[allow(dead_code)]
pub fn init_dev_logger() -> Result<(), Box<dyn Error>> {
    init_logger(LevelFilter::Debug)
}

/// Initialize logging with custom pattern.
///
/// # Arguments
/// * `level` - The minimum log level to display
/// * `pattern` - Custom pattern for log formatting
///
/// # Returns
/// * `Ok(())` - If logging was successfully initialized
/// * `Err(Box<dyn Error>)` - If initialization failed
///
/// # Pattern Examples
/// * `"{m}{n}"` - Just the message
/// * `"[{l}] {m}{n}"` - Level and message
/// * `"{d} [{l}] {t}: {m}{n}"` - Timestamp, level, target, and message
#[allow(dead_code)]
pub fn init_logger_with_pattern(level: LevelFilter, pattern: &str) -> Result<(), Box<dyn Error>> {
    let console_appender = ConsoleAppender::builder()
        .target(Target::Stderr)
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(console_appender)))
        .build(Root::builder().appender("console").build(level))?;

    log4rs::init_config(config)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, warn};
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_logger() {
        INIT.call_once(|| {
            let _ = init_logger(LevelFilter::Debug);
        });
    }

    #[test]
    fn test_logger_initialization() {
        init_test_logger();

        // Test logging at different levels
        error!("Test error message");
        warn!("Test warning message");
        info!("Test info message");
        debug!("Test debug message");
    }

    #[test]
    fn test_custom_pattern() {
        // Since we can't reinitialize the logger, just test that the function exists
        // and would work if called first
        init_test_logger();
        info!("Test message with existing logger");

        // Test that the function signature is correct by calling it
        // (it will fail internally but shouldn't panic)
        let result = init_logger_with_pattern(LevelFilter::Info, "[{l}] {m}{n}");
        // We expect this to fail since logger is already initialized
        assert!(result.is_err());
    }

    #[test]
    fn test_convenience_functions() {
        init_test_logger();

        // Test that convenience functions exist and have correct signatures
        let result1 = init_default_logger();
        let result2 = init_dev_logger();

        // These should fail since logger is already initialized, but shouldn't panic
        assert!(result1.is_err());
        assert!(result2.is_err());
    }
}
