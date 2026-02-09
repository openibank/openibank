//! OpeniBank API Server
//!
//! Production-ready REST API server for the OpeniBank trading platform.
//! Provides Binance-compatible endpoints for seamless integration.
//!
//! # Features
//!
//! - JWT and API Key authentication
//! - Rate limiting per IP/user
//! - WebSocket support for real-time data
//! - OpenAPI documentation with Swagger UI
//! - Prometheus metrics export
//! - Graceful shutdown handling
//! - Health check endpoints
//!
//! # Usage
//!
//! ```bash
//! # Start with default settings
//! openibank-api-server
//!
//! # Start with custom config
//! openibank-api-server --config /path/to/config.toml
//!
//! # Start with environment overrides
//! OPENIBANK__SERVER__PORT=8080 openibank-api-server
//! ```

mod config;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use tokio::signal;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use openibank_api::{create_router, ApiConfig, AppState};
use openibank_auth::{AuthConfig, AuthService};
use openibank_db::{Database, DatabaseConfig as DbConfig};

use crate::config::ServerConfig;

// =============================================================================
// CLI Arguments
// =============================================================================

/// OpeniBank API Server - Production-ready trading platform API
#[derive(Parser, Debug)]
#[command(name = "openibank-api-server")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file (TOML, JSON, or YAML)
    #[arg(short, long, env = "OPENIBANK_CONFIG")]
    config: Option<String>,

    /// Host to bind to
    #[arg(long, env = "OPENIBANK_HOST")]
    host: Option<String>,

    /// Port to listen on
    #[arg(short, long, env = "OPENIBANK_PORT")]
    port: Option<u16>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "OPENIBANK_LOG_LEVEL", default_value = "info")]
    log_level: String,

    /// Log format (json, pretty)
    #[arg(long, env = "OPENIBANK_LOG_FORMAT", default_value = "pretty")]
    log_format: String,

    /// PostgreSQL connection URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Redis connection URL
    #[arg(long, env = "REDIS_URL")]
    redis_url: Option<String>,

    /// JWT secret key
    #[arg(long, env = "JWT_SECRET")]
    jwt_secret: Option<String>,

    /// Enable development mode (relaxed security)
    #[arg(long, env = "OPENIBANK_DEV_MODE")]
    dev_mode: bool,
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args = Args::parse();

    // Load configuration
    let mut server_config = ServerConfig::load(args.config.as_deref())?;

    // Override with CLI arguments
    if let Some(host) = args.host {
        server_config.server.host = host;
    }
    if let Some(port) = args.port {
        server_config.server.port = port;
    }
    if let Some(db_url) = args.database_url {
        server_config.database.postgres_url = db_url;
    }
    if let Some(redis_url) = args.redis_url {
        server_config.redis = Some(config::RedisConfig {
            url: redis_url,
            pool_size: 20,
        });
    }
    if let Some(jwt_secret) = args.jwt_secret {
        server_config.auth.jwt_secret = jwt_secret;
    }
    server_config.logging.level = args.log_level;
    server_config.logging.format = args.log_format;

    // Initialize logging
    init_logging(&server_config.logging)?;

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Starting OpeniBank API Server"
    );

    // Validate configuration
    validate_config(&server_config, args.dev_mode)?;

    // Initialize database
    let db = init_database(&server_config.database).await?;

    // Initialize auth service
    let auth = init_auth(&server_config.auth, db.clone())?;

    // Create application state
    let state = Arc::new(AppState { db, auth });

    // Create API configuration
    let api_config = ApiConfig {
        enable_cors: server_config.api.enable_cors,
        cors_origins: server_config.api.cors_origins.clone(),
        enable_compression: server_config.api.enable_compression,
        enable_tracing: server_config.api.enable_tracing,
        rate_limit: server_config.auth.rate_limit_per_minute,
        max_body_size: server_config.api.max_body_size,
    };

    // Create router
    let app = create_router(state, api_config);

    // Start metrics server if enabled
    if server_config.metrics.enabled {
        start_metrics_server(&server_config.metrics).await?;
    }

    // Get bind address
    let addr = server_config.server.socket_addr();

    tracing::info!(
        host = %server_config.server.host,
        port = %server_config.server.port,
        "Server listening"
    );

    // Start server with graceful shutdown
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(server_config.server.shutdown_timeout()))
        .await?;

    tracing::info!("Server shutdown complete");

    Ok(())
}

// =============================================================================
// Initialization Functions
// =============================================================================

/// Initialize tracing/logging
fn init_logging(config: &config::LoggingConfig) -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let subscriber = tracing_subscriber::registry().with(env_filter);

    match config.format.as_str() {
        "json" => {
            subscriber
                .with(fmt::layer().json().with_target(true))
                .init();
        }
        _ => {
            subscriber
                .with(fmt::layer().pretty().with_target(true))
                .init();
        }
    }

    Ok(())
}

/// Validate configuration
fn validate_config(config: &ServerConfig, dev_mode: bool) -> anyhow::Result<()> {
    // Check JWT secret in production
    if !dev_mode && config.auth.jwt_secret == "change-me-in-production" {
        anyhow::bail!(
            "JWT secret must be changed in production. Set JWT_SECRET environment variable."
        );
    }

    // Check TLS in production
    if !dev_mode && config.server.port == 443 && !config.server.tls_enabled {
        tracing::warn!("Running on port 443 without TLS enabled");
    }

    // Validate TLS configuration
    if config.server.tls_enabled {
        if config.server.tls_cert.is_none() || config.server.tls_key.is_none() {
            anyhow::bail!("TLS is enabled but certificate or key path is not set");
        }
    }

    Ok(())
}

/// Initialize database connection
async fn init_database(config: &config::DatabaseConfig) -> anyhow::Result<Arc<Database>> {
    tracing::info!("Connecting to database...");

    // Create database configuration
    let db_config = DbConfig {
        postgres_url: config.postgres_url.clone(),
        redis_url: "redis://localhost:6379".to_string(), // Default, may not be used
        pg_max_connections: config.max_connections,
        pg_min_connections: config.min_connections,
        pg_acquire_timeout_secs: config.connect_timeout_secs,
    };

    // Connect to database
    let db = Database::connect(&db_config).await?;

    tracing::info!("Database connected successfully");

    // Run health check
    let health = db.health_check().await?;
    if !health.healthy {
        anyhow::bail!("Database health check failed");
    }

    tracing::info!(
        postgres = health.postgres,
        redis = health.redis,
        "Database health check passed"
    );

    Ok(Arc::new(db))
}

/// Initialize authentication service
fn init_auth(config: &config::AuthSettings, db: Arc<Database>) -> anyhow::Result<Arc<AuthService>> {
    tracing::info!("Initializing authentication service...");

    // Use defaults and override specific fields
    let auth_config = AuthConfig {
        jwt: openibank_auth::config::JwtConfig {
            secret: config.jwt_secret.clone(),
            issuer: config.jwt_issuer.clone(),
            audience: config.jwt_audience.clone(),
            access_token_lifetime: Duration::from_secs(config.access_token_lifetime_secs),
            refresh_token_lifetime: Duration::from_secs(config.refresh_token_lifetime_secs),
            algorithm: "HS256".to_string(),
            rotate_refresh_tokens: true,
            refresh_grace_period: Duration::from_secs(60),
        },
        password: openibank_auth::config::PasswordConfig::default(),
        session: openibank_auth::config::SessionConfig::default(),
        totp: openibank_auth::config::TotpConfig::default(),
        rate_limit: openibank_auth::config::RateLimitConfig::default(),
        api_key: openibank_auth::config::ApiKeyConfig::default(),
    };

    let auth_service = AuthService::new(db, auth_config);

    tracing::info!("Authentication service initialized");

    Ok(Arc::new(auth_service))
}

/// Start Prometheus metrics server
async fn start_metrics_server(config: &config::MetricsConfig) -> anyhow::Result<()> {
    if let Some(port) = config.port {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        tracing::info!(
            port = port,
            path = %config.path,
            "Starting metrics server"
        );

        // Install metrics exporter
        let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
        let handle = builder
            .with_http_listener(addr)
            .install_recorder()?;

        // The recorder runs in the background
        tracing::info!("Metrics server started on port {}", port);

        // Keep the handle alive (it will be cleaned up on shutdown)
        tokio::spawn(async move {
            // Keep handle alive
            let _handle = handle;
            // This task runs until the process exits
            std::future::pending::<()>().await;
        });
    }

    Ok(())
}

// =============================================================================
// Graceful Shutdown
// =============================================================================

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal(timeout: Duration) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown...");
        }
    }

    // Allow time for in-flight requests to complete
    tracing::info!(
        timeout_secs = timeout.as_secs(),
        "Waiting for in-flight requests to complete..."
    );

    tokio::time::sleep(timeout).await;
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let args = Args::parse_from(["openibank-api-server", "--port", "8080"]);
        assert_eq!(args.port, Some(8080));
    }

    #[test]
    fn test_development_config() {
        let config = ServerConfig::development();
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.logging.level, "debug");
    }
}
