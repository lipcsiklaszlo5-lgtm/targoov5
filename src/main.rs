use targoo_v2::api::{download_handler, results_handler, run_handler, status_handler, upload_handler, SharedState};
use targoo_v2::db::{init_db, DbPool};
use targoo_v2::models::AppState;
use targoo_v2::ai_client::AiBridgeClient;
use axum::{
    extract::FromRef,
    routing::{get, post},
    Router,
};
use clap::Parser;
use dotenv::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input file path for headless mode
    #[arg(short, long)]
    input: Option<String>,

    /// Only process Scope 3 emissions
    #[arg(long, default_value_t = false)]
    scope3_only: bool,

    /// Only use local dictionary (disable AI Bridge)
    #[arg(long, default_value_t = false)]
    dictionary_only: bool,

    /// Output directory for Fritz Package
    #[arg(short, long, default_value = "./output")]
    output: String,

    /// Port to listen on (web mode)
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
}

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
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    let args = Args::parse();

    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    
    tracing::info!("Starting Targoo V2 ESG Data Refinery...");
    
    // Initialize database
    let db_pool = match init_db() {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            anyhow::bail!("Database initialization failed: {}", e);
        }
    };
    tracing::info!("SQLite database initialized with WORM triggers");
    
    // Initialize shared application state
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let ai_client = Arc::new(AiBridgeClient::new());

    if let Some(input_path) = args.input {
        run_headless(
            input_path,
            args.scope3_only,
            args.dictionary_only,
            args.output,
            db_pool,
            ai_client,
        ).await?;
        return Ok(());
    }

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
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    tracing::info!("Server listening on http://{}", addr);
    
    // Start the server
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            anyhow::bail!("Port binding failed: {}", e);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Server execution error: {}", e);
        anyhow::bail!("Server failed: {}", e);
    }

    Ok(())
}

async fn run_headless(
    input_path: String,
    scope3_only: bool,
    dictionary_only: bool,
    output_dir: String,
    db_pool: DbPool,
    ai_client: Arc<AiBridgeClient>,
) -> anyhow::Result<()> {
    use targoo_v2::models::{Jurisdiction, GhgScope};
    use targoo_v2::triage::TriageEngine;
    use targoo_v2::ingest::{IngestEngine, RawRow};
    use targoo_v2::ledger::{LedgerProcessor, ProcessResult, verify_chain};
    use targoo_v2::aggregation::Aggregator;
    use targoo_v2::gemini_client::GeminiClient;
    use targoo_v2::output_factory::OutputFactory;
    use targoo_v2::db::{create_run, bulk_insert_ledger, bulk_insert_quarantine, update_run_status};
    use futures::stream::{self, StreamExt};
    use uuid::Uuid;
    use std::collections::HashMap;

    tracing::info!("RUNNING IN HEADLESS MODE");
    tracing::info!("Input: {}", input_path);
    tracing::info!("Scope3 Only: {}", scope3_only);
    tracing::info!("Dictionary Only: {}", dictionary_only);
    tracing::info!("Output Dir: {}", output_dir);

    let run_id = Uuid::new_v4().to_string();
    let jurisdiction = Jurisdiction::EU;
    let language = "en".to_string();
    let industry = "Manufacturing".to_string();

    // 1. Init DB
    {
        let mut conn = db_pool.lock().map_err(|e| anyhow::anyhow!("DB Lock error: {}", e))?;
        create_run(&mut conn, &run_id, "EU", &language, &industry)?;
    }

    // 2. Load Engines
    let mut triage_engine = TriageEngine::with_client(ai_client);
    triage_engine.allow_ai = !dictionary_only;
    
    let dict_content = std::fs::read_to_string("data/dictionary.json")?;
    triage_engine.load_from_json(&dict_content)?;

    let ingestion_engine = IngestEngine::new();
    let ledger_processor = LedgerProcessor::new();
    let aggregator = Aggregator::new();

    // 3. Ingest & Process (Streaming)
    let (_, stream) = ingestion_engine.open(std::path::Path::new(&input_path))?;
    
    const CONCURRENT_TASKS: usize = 16;
    let process_results: Vec<ProcessResult> = stream::iter(stream)
        .map(|row_res| {
            let mut lp = ledger_processor.clone();
            let mut te = triage_engine.clone();
            let rid = run_id.clone();
            let jur = jurisdiction;
            async move {
                match row_res {
                    Ok(raw_row) => {
                        let res: anyhow::Result<Option<ProcessResult>> = tokio::spawn(async move {
                            lp.process_row(&rid, &raw_row, &mut te, jur).await
                        }).await.unwrap_or_else(|e| Err(anyhow::anyhow!("Spawn error: {}", e)));
                        res
                    },
                    Err(_) => Ok(None)
                }
            }
        })
        .buffer_unordered(CONCURRENT_TASKS)
        .filter_map(|res: anyhow::Result<Option<ProcessResult>>| async { res.ok().flatten() })
        .collect()
        .await;

    let mut ledger_rows = Vec::new();
    let mut quarantine_rows = Vec::new();

    for result in process_results {
        match result {
            ProcessResult::Ledger(row) => {
                if scope3_only && row.ghg_scope != GhgScope::SCOPE3 {
                    continue;
                }
                ledger_rows.push(row);
            },
            ProcessResult::Quarantine(row) => quarantine_rows.push(row),
        }
    }

    // 5. Chain & DB
    ledger_rows.sort_by_key(|r| r.raw_row_index);
    let mut prev_hash = String::new();
    for row in &mut ledger_rows {
        let hash_input = format!(
            "{}{}{}{}{:.8}{:?}{:.4}",
            prev_hash, row.raw_row_index, row.raw_header, row.raw_value, row.tco2e,
            row.scope3_extension.as_ref().map(|e| e.category_id).unwrap_or(0),
            row.confidence
        );
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(hash_input.as_bytes());
        row.sha256_hash = hex::encode(hasher.finalize());
        prev_hash = row.sha256_hash.clone();
    }

    {
        let mut conn = db_pool.lock().map_err(|e| anyhow::anyhow!("DB Lock error: {}", e))?;
        bulk_insert_ledger(&mut conn, &run_id, &ledger_rows)?;
        bulk_insert_quarantine(&mut conn, &run_id, &quarantine_rows)?;
        update_run_status(&mut conn, &run_id, "completed")?;
    }

    // 6. Aggregate & AI Narrative
    let aggregation = aggregator.aggregate(&ledger_rows, quarantine_rows.len());
    let scope3_breakdown: HashMap<u8, targoo_v2::models::Scope3CategorySummary> = aggregation
        .scope3_breakdown.iter().map(|(id, s)| (*id, s.clone())).collect();

    let gemini_api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let narrative = if !gemini_api_key.is_empty() {
        let gemini_client = GeminiClient::new(gemini_api_key)?;
        gemini_client.generate_narrative(&aggregation, jurisdiction, &language, &industry, &scope3_breakdown).await
    } else {
        "AI Narrative skipped (no API key)".to_string()
    };

    // 7. Fritz Package
    let output_factory = OutputFactory::new();
    let zip_data = output_factory.generate_fritz_package(
        &run_id, &ledger_rows, &quarantine_rows, &aggregation, &scope3_breakdown,
        &narrative, "EU", &language, None, None
    ).await?;

    std::fs::create_dir_all(&output_dir)?;
    let output_file = format!("{}/TargooV2_Fritz_Package_{}.zip", output_dir, run_id);
    std::fs::write(&output_file, zip_data)?;

    tracing::info!("HEADLESS PROCESSING COMPLETE");
    tracing::info!("Package saved to: {}", output_file);
    tracing::info!("Total tCO2e: {:.2}", aggregation.total_tco2e);

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}
