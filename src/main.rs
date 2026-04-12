mod aggregation;
mod ai_client;
mod api;
mod audit;
mod benchmarking;
mod compliance;
mod db;
mod eeio_engine;
mod eidas;
mod finance;
mod flags;
mod gemini_client;
mod ingest;
mod ixbrl;
mod ledger;
mod models;
mod output_factory;
mod physics;
mod scope3_classifier;
mod scope3_hybrid;
mod scope3_range;
mod taxonomy;
mod triage;
mod triage_context;

use crate::api::{download_handler, results_handler, run_handler, status_handler, upload_handler, SharedState};
use crate::db::{init_db, DbPool};
use crate::models::AppState;
use axum::{
    extract::FromRef,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::ai_client::AiBridgeClient;

#[derive(Clone)]
pub struct CombinedState {
    pub app_state: SharedState,
    pub db_pool: DbPool,
    pub ai_client: Arc<AiBridgeClient>,
}

impl FromRef<CombinedState> for SharedState {
    fn from_ref(state: &CombinedState) -> Self {
        state.app_state.clone()
    }
}

impl FromRef<CombinedState> for DbPool {
    fn from_ref(state: &CombinedState) -> Self {
        state.db_pool.clone()
    }
}

impl FromRef<CombinedState> for Arc<AiBridgeClient> {
    fn from_ref(state: &CombinedState) -> Self {
        state.ai_client.clone()
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    
    tracing::info!("Starting Targoo V2 ESG Data Refinery...");
    
    // Initialize database
    let db_pool = init_db().expect("Failed to initialize database");
    tracing::info!("SQLite database initialized with WORM triggers");
    
    // Initialize shared application state
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let ai_client = Arc::new(AiBridgeClient::new());

    let combined_state = CombinedState {
        app_state,
        db_pool,
        ai_client,
    };
    
    // Configure CORS for frontend and allow all methods
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    
    // Build the router with all routes
    let app = Router::new()
        .route("/upload", post(upload_handler))
        .route("/run", post(run_handler))
        .route("/status", get(status_handler))
        .route("/results", get(results_handler))
        .route("/download", get(download_handler))
        .route("/health", get(health_handler))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(combined_state);
    
    // Define the address to listen on
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Server listening on http://{}", addr);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> &'static str {
    "OK"
}
