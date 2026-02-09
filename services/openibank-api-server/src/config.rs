//! Server Configuration
//!
//! Configuration management for the OpeniBank API server.
//! Supports environment variables, config files, and CLI arguments.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server binding configuration
    #[serde(default)]
    pub server: ServerSettings,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Redis configuration (optional)
    #[serde(default)]
    pub redis: Option<RedisConfig>,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthSettings,

    /// API configuration
    #[serde(default)]
    pub api: ApiSettings,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Metrics configuration
    #[serde(default)]
    pub metrics: MetricsConfig,
}

/// Server binding settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Worker threads (0 = auto)
    #[serde(default)]
    pub workers: usize,

    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// TLS certificate path
    pub tls_cert: Option<PathBuf>,

    /// TLS key path
    pub tls_key: Option<PathBuf>,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout_secs: u64,

    /// Shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            workers: 0,
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
            request_timeout_secs: default_request_timeout(),
            shutdown_timeout_secs: default_shutdown_timeout(),
        }
    }
}

impl ServerSettings {
    /// Get the socket address to bind to
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.host, self.port)
            .parse()
            .expect("Invalid socket address")
    }

    /// Get the request timeout duration
    pub fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request_timeout_secs)
    }

    /// Get the shutdown timeout duration
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.shutdown_timeout_secs)
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub postgres_url: String,

    /// Maximum connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum connections in pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_secs: u64,

    /// Idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,

    /// Run migrations on startup
    #[serde(default = "default_true")]
    pub run_migrations: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            postgres_url: "postgres://openibank:openibank@localhost:5432/openibank".to_string(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            connect_timeout_secs: default_connect_timeout(),
            idle_timeout_secs: default_idle_timeout(),
            run_migrations: true,
        }
    }
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Maximum connections in pool
    #[serde(default = "default_redis_pool_size")]
    pub pool_size: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: default_redis_pool_size(),
        }
    }
}

/// Authentication settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSettings {
    /// JWT secret key
    pub jwt_secret: String,

    /// JWT issuer
    #[serde(default = "default_jwt_issuer")]
    pub jwt_issuer: String,

    /// JWT audience
    #[serde(default = "default_jwt_audience")]
    pub jwt_audience: String,

    /// Access token lifetime in seconds
    #[serde(default = "default_access_token_lifetime")]
    pub access_token_lifetime_secs: u64,

    /// Refresh token lifetime in seconds
    #[serde(default = "default_refresh_token_lifetime")]
    pub refresh_token_lifetime_secs: u64,

    /// Enable API key authentication
    #[serde(default = "default_true")]
    pub enable_api_keys: bool,

    /// Rate limit requests per minute per IP
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me-in-production".to_string(),
            jwt_issuer: default_jwt_issuer(),
            jwt_audience: default_jwt_audience(),
            access_token_lifetime_secs: default_access_token_lifetime(),
            refresh_token_lifetime_secs: default_refresh_token_lifetime(),
            enable_api_keys: true,
            rate_limit_per_minute: default_rate_limit(),
        }
    }
}

/// API settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSettings {
    /// Enable CORS
    #[serde(default = "default_true")]
    pub enable_cors: bool,

    /// CORS allowed origins (comma-separated)
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,

    /// Enable response compression
    #[serde(default = "default_true")]
    pub enable_compression: bool,

    /// Enable request tracing
    #[serde(default = "default_true")]
    pub enable_tracing: bool,

    /// Maximum request body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,

    /// API version prefix
    #[serde(default = "default_api_prefix")]
    pub api_prefix: String,
}

impl Default for ApiSettings {
    fn default() -> Self {
        Self {
            enable_cors: true,
            cors_origins: default_cors_origins(),
            enable_compression: true,
            enable_tracing: true,
            max_body_size: default_max_body_size(),
            api_prefix: default_api_prefix(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format (json, pretty)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Enable request/response logging
    #[serde(default = "default_true")]
    pub log_requests: bool,

    /// Log slow requests threshold in ms
    #[serde(default = "default_slow_request_threshold")]
    pub slow_request_threshold_ms: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            log_requests: true,
            slow_request_threshold_ms: default_slow_request_threshold(),
        }
    }
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics endpoint
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Metrics endpoint path
    #[serde(default = "default_metrics_path")]
    pub path: String,

    /// Metrics port (separate from main server)
    #[serde(default = "default_metrics_port")]
    pub port: Option<u16>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_metrics_path(),
            port: Some(default_metrics_port().unwrap_or(9090)),
        }
    }
}

// =============================================================================
// Default Functions
// =============================================================================

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_request_timeout() -> u64 {
    30
}

fn default_shutdown_timeout() -> u64 {
    30
}

fn default_max_connections() -> u32 {
    100
}

fn default_min_connections() -> u32 {
    5
}

fn default_connect_timeout() -> u64 {
    30
}

fn default_idle_timeout() -> u64 {
    600
}

fn default_redis_pool_size() -> usize {
    20
}

fn default_jwt_issuer() -> String {
    "openibank".to_string()
}

fn default_jwt_audience() -> String {
    "openibank-api".to_string()
}

fn default_access_token_lifetime() -> u64 {
    3600 // 1 hour
}

fn default_refresh_token_lifetime() -> u64 {
    604800 // 7 days
}

fn default_rate_limit() -> u32 {
    1200
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_max_body_size() -> usize {
    10 * 1024 * 1024 // 10MB
}

fn default_api_prefix() -> String {
    "/api/v1".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_slow_request_threshold() -> u64 {
    1000
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_metrics_port() -> Option<u16> {
    Some(9090)
}

fn default_true() -> bool {
    true
}

// =============================================================================
// Configuration Loading
// =============================================================================

impl ServerConfig {
    /// Load configuration from environment and optional config file
    pub fn load(config_path: Option<&str>) -> anyhow::Result<Self> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        let mut builder = config::Config::builder();

        // Add config file if specified
        if let Some(path) = config_path {
            builder = builder.add_source(config::File::with_name(path).required(false));
        }

        // Add default config locations
        builder = builder
            .add_source(config::File::with_name("config/default").required(false))
            .add_source(config::File::with_name("config/local").required(false));

        // Add environment variables with OPENIBANK_ prefix
        builder = builder.add_source(
            config::Environment::with_prefix("OPENIBANK")
                .separator("__")
                .try_parsing(true),
        );

        // Build and deserialize
        let config = builder.build()?;

        // Try to deserialize, falling back to defaults where needed
        let server_config: ServerConfig = config.try_deserialize().unwrap_or_else(|_| {
            tracing::warn!("Using default configuration - some settings may need adjustment");
            ServerConfig::default()
        });

        Ok(server_config)
    }

    /// Create a configuration for development/testing
    pub fn development() -> Self {
        Self {
            server: ServerSettings::default(),
            database: DatabaseConfig::default(),
            redis: None,
            auth: AuthSettings::default(),
            api: ApiSettings::default(),
            logging: LoggingConfig {
                level: "debug".to_string(),
                format: "pretty".to_string(),
                log_requests: true,
                slow_request_threshold_ms: 500,
            },
            metrics: MetricsConfig::default(),
        }
    }

    /// Create a configuration for production
    pub fn production() -> Self {
        Self {
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
                workers: 0,
                tls_enabled: true,
                tls_cert: Some(PathBuf::from("/etc/ssl/certs/server.crt")),
                tls_key: Some(PathBuf::from("/etc/ssl/private/server.key")),
                request_timeout_secs: 60,
                shutdown_timeout_secs: 60,
            },
            database: DatabaseConfig {
                max_connections: 200,
                min_connections: 20,
                ..Default::default()
            },
            redis: Some(RedisConfig::default()),
            auth: AuthSettings {
                jwt_secret: std::env::var("JWT_SECRET")
                    .expect("JWT_SECRET must be set in production"),
                rate_limit_per_minute: 600,
                ..Default::default()
            },
            api: ApiSettings {
                cors_origins: vec![
                    "https://app.openibank.io".to_string(),
                    "https://openibank.io".to_string(),
                ],
                ..Default::default()
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                log_requests: true,
                slow_request_threshold_ms: 1000,
            },
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::development()
    }
}
