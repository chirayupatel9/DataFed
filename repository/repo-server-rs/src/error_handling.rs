// Enhanced error handling and user experience improvements
use std::fmt;
use std::error::Error as StdError;

/// Enhanced error types for better user experience
#[derive(Debug, Clone)]
pub enum RepoServerError {
    ConfigurationError(String),
    NetworkError(String),
    SecurityError(String),
    FileSystemError(String),
    MessageError(String),
    WorkerError(String),
    ShutdownError(String),
    InternalError(String),
}

impl fmt::Display for RepoServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoServerError::ConfigurationError(msg) => {
                write!(f, "Configuration Error: {}\n\nPlease check your configuration file and ensure all required settings are correct.", msg)
            }
            RepoServerError::NetworkError(msg) => {
                write!(f, "Network Error: {}\n\nPlease check your network connection and server address.", msg)
            }
            RepoServerError::SecurityError(msg) => {
                write!(f, "Security Error: {}\n\nThis operation was blocked for security reasons.", msg)
            }
            RepoServerError::FileSystemError(msg) => {
                write!(f, "File System Error: {}\n\nPlease check file permissions and disk space.", msg)
            }
            RepoServerError::MessageError(msg) => {
                write!(f, "Message Processing Error: {}\n\nPlease check the message format and try again.", msg)
            }
            RepoServerError::WorkerError(msg) => {
                write!(f, "Worker Error: {}\n\nPlease check worker thread configuration.", msg)
            }
            RepoServerError::ShutdownError(msg) => {
                write!(f, "Shutdown Error: {}\n\nPlease check if the server is running properly.", msg)
            }
            RepoServerError::InternalError(msg) => {
                write!(f, "Internal Error: {}\n\nThis is an unexpected error. Please report this issue.", msg)
            }
        }
    }
}

impl StdError for RepoServerError {}

/// Error context for providing additional information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub details: String,
    pub suggestion: Option<String>,
    pub error_code: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: &str, details: &str) -> Self {
        Self {
            operation: operation.to_string(),
            details: details.to_string(),
            suggestion: None,
            error_code: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    pub fn with_error_code(mut self, code: &str) -> Self {
        self.error_code = Some(code.to_string());
        self
    }
}

/// Enhanced error with context
#[derive(Debug, Clone)]
pub struct ContextualError {
    pub error: RepoServerError,
    pub context: ErrorContext,
}

impl ContextualError {
    pub fn new(error: RepoServerError, context: ErrorContext) -> Self {
        Self { error, context }
    }

    pub fn print_helpful_message(&self) {
        eprintln!("❌ Error: {}", self.error);
        eprintln!("📍 Operation: {}", self.context.operation);
        eprintln!("📝 Details: {}", self.context.details);
        
        if let Some(suggestion) = &self.context.suggestion {
            eprintln!("💡 Suggestion: {}", suggestion);
        }
        
        if let Some(code) = &self.context.error_code {
            eprintln!("🔢 Error Code: {}", code);
        }
        
        eprintln!();
    }
}

impl fmt::Display for ContextualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl StdError for ContextualError {}

/// User-friendly error messages and suggestions
pub struct ErrorMessageBuilder;

impl ErrorMessageBuilder {
    pub fn configuration_error(operation: &str, details: &str) -> ContextualError {
        let context = ErrorContext::new(operation, details)
            .with_suggestion("Run 'repo --help' to see configuration options")
            .with_error_code("CONFIG_001");
        
        ContextualError::new(
            RepoServerError::ConfigurationError(details.to_string()),
            context,
        )
    }

    pub fn network_error(operation: &str, details: &str) -> ContextualError {
        let context = ErrorContext::new(operation, details)
            .with_suggestion("Check if the core server is running and accessible")
            .with_error_code("NET_001");
        
        ContextualError::new(
            RepoServerError::NetworkError(details.to_string()),
            context,
        )
    }

    pub fn security_error(operation: &str, details: &str) -> ContextualError {
        let context = ErrorContext::new(operation, details)
            .with_suggestion("Check file permissions and path security settings")
            .with_error_code("SEC_001");
        
        ContextualError::new(
            RepoServerError::SecurityError(details.to_string()),
            context,
        )
    }

    pub fn file_system_error(operation: &str, details: &str) -> ContextualError {
        let context = ErrorContext::new(operation, details)
            .with_suggestion("Check disk space and file permissions")
            .with_error_code("FS_001");
        
        ContextualError::new(
            RepoServerError::FileSystemError(details.to_string()),
            context,
        )
    }

    pub fn message_error(operation: &str, details: &str) -> ContextualError {
        let context = ErrorContext::new(operation, details)
            .with_suggestion("Check message format and try again")
            .with_error_code("MSG_001");
        
        ContextualError::new(
            RepoServerError::MessageError(details.to_string()),
            context,
        )
    }

    pub fn worker_error(operation: &str, details: &str) -> ContextualError {
        let context = ErrorContext::new(operation, details)
            .with_suggestion("Check worker thread configuration and system resources")
            .with_error_code("WORKER_001");
        
        ContextualError::new(
            RepoServerError::WorkerError(details.to_string()),
            context,
        )
    }
}

/// Progress reporting for long-running operations
pub struct ProgressReporter {
    operation: String,
    total_steps: u32,
    current_step: u32,
    start_time: std::time::Instant,
}

impl ProgressReporter {
    pub fn new(operation: &str, total_steps: u32) -> Self {
        Self {
            operation: operation.to_string(),
            total_steps,
            current_step: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn step(&mut self, message: &str) {
        self.current_step += 1;
        let percentage = (self.current_step as f64 / self.total_steps as f64) * 100.0;
        let elapsed = self.start_time.elapsed();
        
        print!("\r🔄 {}: {} ({:.1}%) - {}", 
               self.operation, 
               message, 
               percentage,
               format_duration(elapsed));
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }

    pub fn complete(&self, message: &str) {
        let elapsed = self.start_time.elapsed();
        println!("\r✅ {}: {} (completed in {})", 
                self.operation, 
                message,
                format_duration(elapsed));
    }

    pub fn error(&self, message: &str) {
        let elapsed = self.start_time.elapsed();
        println!("\r❌ {}: {} (failed after {})", 
                self.operation, 
                message,
                format_duration(elapsed));
    }
}

/// Format duration in a human-readable way
fn format_duration(duration: std::time::Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.1}s", duration.as_secs_f64())
    } else {
        format!("{}ms", duration.as_millis())
    }
}

/// Help system for common issues
pub struct HelpSystem;

impl HelpSystem {
    pub fn print_startup_help() {
        println!("🚀 DataFed Repo Server Starting...");
        println!();
        println!("📋 Quick Start Guide:");
        println!("   1. Ensure the core server is running");
        println!("   2. Check your configuration file (repo-server.toml)");
        println!("   3. Verify your credentials directory contains the required keys");
        println!("   4. Monitor logs for any issues");
        println!();
        println!("🔧 Common Commands:");
        println!("   repo version          - Show version information");
        println!("   repo serve --cfg <file> - Start server with config file");
        println!("   repo --gen-keys       - Generate new server keys");
        println!("   repo --help           - Show detailed help");
        println!();
    }

    pub fn print_troubleshooting_help() {
        println!("🔍 Troubleshooting Guide:");
        println!();
        println!("❌ Connection Issues:");
        println!("   • Check if core server is running: telnet <host> <port>");
        println!("   • Verify network connectivity");
        println!("   • Check firewall settings");
        println!();
        println!("❌ Configuration Issues:");
        println!("   • Validate TOML syntax in config file");
        println!("   • Check file paths and permissions");
        println!("   • Ensure all required fields are set");
        println!();
        println!("❌ Permission Issues:");
        println!("   • Check file and directory permissions");
        println!("   • Ensure user has write access to data directories");
        println!("   • Verify key file permissions (600 recommended)");
        println!();
        println!("❌ Performance Issues:");
        println!("   • Monitor system resources (CPU, memory, disk)");
        println!("   • Check worker thread configuration");
        println!("   • Review log files for bottlenecks");
        println!();
    }

    pub fn print_health_check_help(health_status: &crate::metrics::HealthStatus) {
        if health_status.is_healthy {
            println!("✅ Server Health: Healthy");
        } else {
            println!("❌ Server Health: Issues Detected");
            println!("   Issues:");
            for issue in &health_status.issues {
                println!("   • {}", issue);
            }
            println!();
            Self::print_troubleshooting_help();
        }
    }
}
