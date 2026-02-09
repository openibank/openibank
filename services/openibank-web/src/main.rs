//! OpeniBank Web Server
//!
//! A high-performance static file server for the OpeniBank marketing website.
//! Serves the landing page and provides basic API endpoints for dynamic stats.

use axum::{
    extract::State,
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use clap::Parser;
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// OpeniBank Web Server - Marketing landing page server
#[derive(Parser, Debug)]
#[command(name = "openibank-web")]
#[command(about = "OpeniBank marketing website and documentation server")]
struct Args {
    /// Host to bind to
    #[arg(long, env = "OPENIBANK_WEB_HOST", default_value = "0.0.0.0")]
    host: String,

    /// Port to bind to
    #[arg(short, long, env = "OPENIBANK_WEB_PORT", default_value = "3080")]
    port: u16,

    /// Static files directory
    #[arg(long, env = "OPENIBANK_WEB_STATIC_DIR", default_value = "static")]
    static_dir: String,
}

/// Platform statistics for the landing page
#[derive(Debug, Clone, Serialize)]
struct PlatformStats {
    agents_created: u64,
    transactions_processed: u64,
    total_volume_usd: u64,
    uptime_percentage: f64,
    avg_latency_ns: u64,
}

impl Default for PlatformStats {
    fn default() -> Self {
        Self {
            agents_created: 12_847,
            transactions_processed: 2_341_892,
            total_volume_usd: 847_293_412,
            uptime_percentage: 99.997,
            avg_latency_ns: 847,
        }
    }
}

/// Application state
struct AppState {
    stats: RwLock<PlatformStats>,
}

impl AppState {
    fn new() -> Self {
        Self {
            stats: RwLock::new(PlatformStats::default()),
        }
    }
}

/// GET /api/stats - Returns platform statistics
async fn get_stats(State(state): State<Arc<AppState>>) -> Json<PlatformStats> {
    let stats = state.stats.read().await;
    Json(stats.clone())
}

/// GET /api/health - Health check endpoint
async fn health_check() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
        r#"{"status":"healthy","service":"openibank-web"}"#,
    )
}

/// Simulate stats updates (in production, this would come from real data)
async fn stats_updater(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        interval.tick().await;
        let mut stats = state.stats.write().await;
        // Simulate gradual increases
        stats.agents_created += rand_increment(0, 3);
        stats.transactions_processed += rand_increment(10, 50);
        stats.total_volume_usd += rand_increment(1000, 10000);
    }
}

/// Generate a random increment within a range
fn rand_increment(min: u64, max: u64) -> u64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as u64;
    min + (nanos % (max - min + 1))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Parse CLI arguments
    let args = Args::parse();
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    // Initialize state
    let state = Arc::new(AppState::new());
    let state_for_updater = state.clone();

    // Start background stats updater
    tokio::spawn(async move {
        stats_updater(state_for_updater).await;
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build API routes
    let api_routes = Router::new()
        .route("/stats", get(get_stats))
        .route("/health", get(health_check));

    // Build main router
    let app = Router::new()
        .nest("/api", api_routes)
        .fallback_service(
            ServeDir::new(&args.static_dir)
                .append_index_html_on_directories(true)
        )
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    info!("Starting OpeniBank Web Server");
    info!("Listening on http://{}", addr);
    info!("Static files: {}", args.static_dir);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
