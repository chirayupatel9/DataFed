use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::warn;

/// Server configuration loaded from TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub zmq: ZmqConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub monitoring: MonitoringConfig,
}

/// Server-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_worker_threads")]
    pub num_worker_threads: u32,
    #[serde(default = "default_core_server")]
    pub core_server: String,
    #[serde(default = "default_globus_path")]
    pub globus_collection_path: String,
    #[serde(default = "default_cred_dir")]
    pub cred_dir: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_console_logging")]
    pub console: bool,
    #[serde(default = "default_file_logging")]
    pub file: bool,
    #[serde(default = "default_log_file")]
    pub log_file: String,
}

/// ZeroMQ configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZmqConfig {
    #[serde(default = "default_receive_timeout")]
    pub receive_timeout_ms: u32,
    #[serde(default = "default_poll_timeout")]
    pub poll_timeout_ms: i64,
    #[serde(default = "default_zmq_debug")]
    pub debug: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_curve_enabled")]
    pub curve_enabled: bool,
    #[serde(default = "default_public_key")]
    pub public_key: String,
    #[serde(default = "default_private_key")]
    pub private_key: String,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_max_message_size")]
    pub max_message_size: usize,
    #[serde(default = "default_worker_idle_timeout")]
    pub worker_idle_timeout: u64,
    #[serde(default = "default_connection_pooling")]
    pub connection_pooling: bool,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
    #[serde(default = "default_metrics_interval")]
    pub metrics_interval: u64,
    #[serde(default = "default_health_check_enabled")]
    pub health_check_enabled: bool,
    #[serde(default = "default_health_check_port")]
    pub health_check_port: u16,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let config_content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_content)?;
        Ok(config)
    }

    /// Load configuration from default locations or create with defaults
    pub fn load() -> Self {
        // Try to load from config.toml in current directory
        if let Ok(config) = Self::from_file("config.toml") {
            warn!("Loaded configuration from config.toml");
            return config;
        }

        // Try to load from /etc/datafed/repo-server.toml
        if let Ok(config) = Self::from_file("/etc/datafed/repo-server.toml") {
            warn!("Loaded configuration from /etc/datafed/repo-server.toml");
            return config;
        }

        // Try to load from ~/.config/datafed/repo-server.toml
        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".config/datafed/repo-server.toml");
            if let Ok(config) = Self::from_file(config_path) {
                warn!("Loaded configuration from ~/.config/datafed/repo-server.toml");
                return config;
            }
        }

        // Use defaults if no config file found
        warn!("No configuration file found, using defaults");
        Self::default()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate server port
        if self.server.port == 0 {
            errors.push("Server port cannot be 0".to_string());
        }

        // Validate worker threads
        if self.server.num_worker_threads == 0 {
            errors.push("Number of worker threads cannot be 0".to_string());
        }

        // Validate timeouts
        if self.zmq.receive_timeout_ms == 0 {
            errors.push("Receive timeout cannot be 0".to_string());
        }

        if self.performance.max_message_size == 0 {
            errors.push("Maximum message size cannot be 0".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            zmq: ZmqConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            num_worker_threads: default_worker_threads(),
            core_server: default_core_server(),
            globus_collection_path: default_globus_path(),
            cred_dir: default_cred_dir(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            console: default_console_logging(),
            file: default_file_logging(),
            log_file: default_log_file(),
        }
    }
}

impl Default for ZmqConfig {
    fn default() -> Self {
        Self {
            receive_timeout_ms: default_receive_timeout(),
            poll_timeout_ms: default_poll_timeout(),
            debug: default_zmq_debug(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            curve_enabled: default_curve_enabled(),
            public_key: default_public_key(),
            private_key: default_private_key(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_message_size: default_max_message_size(),
            worker_idle_timeout: default_worker_idle_timeout(),
            connection_pooling: default_connection_pooling(),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: default_metrics_enabled(),
            metrics_interval: default_metrics_interval(),
            health_check_enabled: default_health_check_enabled(),
            health_check_port: default_health_check_port(),
        }
    }
}

// Default value functions
fn default_port() -> u16 { 10000 }
fn default_worker_threads() -> u32 { 4 }
fn default_core_server() -> String { "tcp://localhost:9998".to_string() }
fn default_globus_path() -> String { "/mnt/datafed-repo".to_string() }
fn default_cred_dir() -> String { "/mnt/storage/rust/DataFed/".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_console_logging() -> bool { true }
fn default_file_logging() -> bool { false }
fn default_log_file() -> String { "/var/log/datafed-repo-server.log".to_string() }
fn default_receive_timeout() -> u32 { 5000 }
fn default_poll_timeout() -> i64 { 10 }
fn default_zmq_debug() -> bool { false }
fn default_curve_enabled() -> bool { false }
fn default_public_key() -> String { "4X1hKiU5pdwdk.s7&=Q2(b1]p!^Nj=Dnk2&7vh@f".to_string() }
fn default_private_key() -> String { "G1DpacgVoCcRmLYQ6PA8:Q$]/w5SE*Qm?)}L!@Gv".to_string() }
fn default_max_message_size() -> usize { 1048576 }
fn default_worker_idle_timeout() -> u64 { 300 }
fn default_connection_pooling() -> bool { true }
fn default_metrics_enabled() -> bool { false }
fn default_metrics_interval() -> u64 { 60 }
fn default_health_check_enabled() -> bool { false }
fn default_health_check_port() -> u16 { 10001 }