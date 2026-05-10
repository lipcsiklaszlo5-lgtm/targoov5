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

    /// Configuration profile JSON file
    #[arg(short, long, default_value = "config/run_profile.json")]
    config: String,

    /// Only process Scope 3 emissions (legacy CLI compatibility)
    #[arg(long, default_value_t = false)]
    scope3_only: bool,

    /// Only use local dictionary (disable AI Bridge)
    #[arg(long, default_value_t = false)]
    dictionary_only: bool,

    /// Quick override: run only this module
    #[arg(long)]
    module: Option<String>,

    /// Output directory for Fritz Package (overrides config)
    #[arg(long)]
    output_override: Option<String>,

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
    
    // ── 1. CCE Loading ──────────────────────────────────────
    let mut validated_config = targoo_v2::config::loader::load_config(Some(&args.config))
        .map_err(|e| anyhow::anyhow!("Konfiguráció betöltése sikertelen: {}", e))?;

    // ── 2. CLI Overrides ─────────────────────────────────────
    if args.scope3_only {
        validated_config.config.modules = vec![targoo_v2::config::models::ComplianceModule::Scope3];
        validated_config.active_modules = vec![targoo_v2::config::models::ComplianceModule::Scope3];
    }

    if let Some(ref mod_name) = args.module {
        let module = match mod_name.to_lowercase().as_str() {
            "scope1_2" => Some(targoo_v2::config::models::ComplianceModule::Scope1_2),
            "scope3" => Some(targoo_v2::config::models::ComplianceModule::Scope3),
            "cbam" => Some(targoo_v2::config::models::ComplianceModule::Cbam),
            "pcaf" => Some(targoo_v2::config::models::ComplianceModule::Pcaf),
            "lksg" => Some(targoo_v2::config::models::ComplianceModule::LkSG),
            "esrse1" => Some(targoo_v2::config::models::ComplianceModule::EsrsE1),
            "swisscsa" => Some(targoo_v2::config::models::ComplianceModule::SwissCSA),
            "secclimate" => Some(targoo_v2::config::models::ComplianceModule::SecClimate),
            _ => {
                tracing::warn!("Ismeretlen modul: {}. Az alapértelmezett lista marad.", mod_name);
                None
            }
        };
        if let Some(m) = module {
            validated_config.config.modules = vec![m.clone()];
            validated_config.active_modules = vec![m];
        }
    }

    if let Some(ref out_dir) = args.output_override {
        validated_config.config.fritz_package.output_dir = out_dir.clone();
    }

    // Initialize database
    let db_pool = match init_db() {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("Failed to initialize database: {}", e);
            anyhow::bail!("Database initialization failed: {}", e);
        }
    };
    tracing::info!("SQLite database initialized with WORM triggers");

    // Save run config to DB (WORM persistence)
    {
        let conn = db_pool.lock().map_err(|e| anyhow::anyhow!("DB Lock error: {}", e))?;
        targoo_v2::config::persistence::save_run_config(&conn, &validated_config)?;
    }
    
    // Initialize shared application state
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let ai_client = Arc::new(AiBridgeClient::new());

    if let Some(input_path) = args.input {
        run_headless(
            input_path,
            validated_config,
            args.dictionary_only,
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

async fn health_handler() -> &'static str {
    "OK"
}

async fn run_headless(
    input_path: String,
    validated: targoo_v2::config::models::ValidatedConfig,
    dictionary_only: bool,
    db_pool: DbPool,
    ai_client: Arc<AiBridgeClient>,
) -> anyhow::Result<()> {
    use targoo_v2::models::{GhgScope, Jurisdiction};
    use targoo_v2::triage::TriageEngine;
    use targoo_v2::ingest::{IngestEngine};
    use targoo_v2::ledger::{LedgerProcessor, ProcessResult};
    use targoo_v2::aggregation::Aggregator;
    use targoo_v2::gemini_client::GeminiClient;
    use targoo_v2::output_factory::OutputFactory;
    use targoo_v2::db::{create_run, bulk_insert_ledger, bulk_insert_quarantine, update_run_status};
    use futures::stream::{self, StreamExt};
    use std::collections::HashMap;

    tracing::info!("RUNNING IN HEADLESS MODE");
    tracing::info!("Input: {}", input_path);
    tracing::info!("Profile: {}", validated.config.profile_name);
    tracing::info!("Run ID: {}", validated.run_id);

    let run_id = validated.run_id.to_string();
    
    // Map CCE Jurisdiction to models::Jurisdiction
    let jurisdiction = match validated.config.jurisdiction {
        targoo_v2::config::models::Jurisdiction::DE => Jurisdiction::DE,
        targoo_v2::config::models::Jurisdiction::AT => Jurisdiction::AT,
        targoo_v2::config::models::Jurisdiction::CH => Jurisdiction::CH,
        targoo_v2::config::models::Jurisdiction::HU => Jurisdiction::HU,
        targoo_v2::config::models::Jurisdiction::EU => Jurisdiction::EU,
        targoo_v2::config::models::Jurisdiction::UK => Jurisdiction::UK,
        targoo_v2::config::models::Jurisdiction::US => Jurisdiction::US,
        targoo_v2::config::models::Jurisdiction::GLOBAL => Jurisdiction::GLOBAL,
    };

    let language = validated.config.language.triage_dictionary_suffix().to_string();
    let industry = validated.config.client.industry.clone();

    // 1. Init DB
    {
        let mut conn = db_pool.lock().map_err(|e| anyhow::anyhow!("DB Lock error: {}", e))?;
        create_run(&mut conn, &run_id, &format!("{:?}", jurisdiction), &language, &industry)?;
    }

    // 2. Load Engines
    let mut triage_engine = TriageEngine::new(&validated);
    if dictionary_only {
        triage_engine.allow_ai = false;
    }
    
    let ingestion_engine = IngestEngine::new();
    let mut ledger_processor = LedgerProcessor::new(); // Changed to mutable
    let aggregator = Aggregator::new();

    // 3. Ingest & Process (Streaming) - SYNCHRONOUS
    let (_, stream) = ingestion_engine.open(std::path::Path::new(&input_path))?;
    
    let mut ledger_rows = Vec::new();
    let mut quarantine_rows = Vec::new();

    for row_res in stream {
        let raw_row = row_res?;
        let res = ledger_processor.process_row(
            &run_id,
            &raw_row,
            &mut triage_engine,
            jurisdiction,
        ).await?;
        match res {
            Some(ProcessResult::Ledger(r)) => ledger_rows.push(r),
            Some(ProcessResult::Quarantine(q)) => quarantine_rows.push(q),
            None => {}
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
    let narrative = if validated.config.fritz_package.include_narrative && !gemini_api_key.is_empty() {
        let gemini_client = GeminiClient::new(gemini_api_key)?;
        gemini_client.generate_narrative(&aggregation, jurisdiction, &language, &industry, &scope3_breakdown).await
    } else {
        "AI Narrative skipped (config or no API key)".to_string()
    };

    // 7. Fritz Package
    let output_factory = OutputFactory::new();
    let output_file = output_factory.generate_fritz_package(
        &ledger_rows, &quarantine_rows, &aggregation, &scope3_breakdown,
        &narrative, &validated
    ).await?;

    tracing::info!("HEADLESS PROCESSING COMPLETE");
    tracing::info!("Package saved to: {}", output_file);
    tracing::info!("Total tCO2e: {:.2}", aggregation.total_tco2e);

    Ok(())
}
