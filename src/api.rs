use crate::aggregation::Aggregator;
use crate::benchmarking::{IndustryBenchmark, PeerComparison};
use crate::compliance::{ObligationStatus, OmnibusValidator};
use crate::db::{create_run, insert_ledger_row, insert_quarantine_row, update_run_status, DbPool};
use crate::finance::risk_analytics::{CarbonRiskMetrics, PortfolioAsset};
use crate::gemini_client::GeminiClient;
use crate::ingest::{IngestionEngine, RawRow};
use crate::ledger::{LedgerProcessor, ProcessResult};
use crate::models::{AppState, Jurisdiction, ResultsResponse, RunRequest, StatusResponse};
use crate::output_factory::OutputFactory;
use crate::triage::TriageEngine;
use anyhow::Result;
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub type SharedState = Arc<Mutex<AppState>>;

// Embedded dictionary JSON
const EMBEDDED_DICTIONARY: &str = include_str!("../data/dictionary.json");

pub async fn upload_handler(
    State(state): State<SharedState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut staged_files = Vec::new();
    
    while let Some(field) = multipart.next_field().await.map_err(|_| AppError::BadRequest)? {
        let file_name = field
            .file_name()
            .ok_or(AppError::BadRequest)?
            .to_string();
        
        let data = field.bytes().await.map_err(|_| AppError::BadRequest)?;
        
        // Save to temp directory
        let temp_path = format!("/tmp/{}", file_name);
        tokio::fs::write(&temp_path, data)
            .await
            .map_err(|_| AppError::InternalError)?;
        
        staged_files.push(file_name);
        
        // Update state
        {
            let mut state_guard = state.lock().await;
            state_guard.staged_files = staged_files.clone();
        }
    }
    
    Ok(Json(serde_json::json!({
        "status": "success",
        "files": staged_files
    })))
}

pub async fn run_handler(
    State(state): State<SharedState>,
    State(db_pool): State<DbPool>,
    Json(payload): Json<RunRequest>,
) -> Result<impl IntoResponse, AppError> {
    let run_id = Uuid::new_v4().to_string();
    
    // Update state to processing
    {
        let mut state_guard = state.lock().await;
        state_guard.status = "processing".to_string();
        state_guard.current_step = 1;
        state_guard.run_id = Some(run_id.clone());
        state_guard.jurisdiction = Some(payload.jurisdiction);
        state_guard.language = Some(payload.language.clone());
        state_guard.industry = Some(payload.industry.clone());
        state_guard.deep_mode = payload.deep_mode;
        state_guard.employee_count = payload.employee_count;
        state_guard.revenue_eur = payload.revenue_eur;
        state_guard.progress_message = Some("Initializing pipeline...".to_string());
    }
    
    // Spawn processing task
    let state_clone = state.clone();
    let db_clone = db_pool.clone();
    let run_id_clone = run_id.clone();
    let deep_mode = payload.deep_mode;
    
    tokio::spawn(async move {
        if let Err(e) = process_pipeline(
            state_clone.clone(),
            db_clone,
            run_id_clone,
            payload.jurisdiction,
            payload.language,
            payload.industry,
            deep_mode,
        )
        .await
        {
            eprintln!("Pipeline error: {}", e);
            let mut state_guard = state_clone.lock().await;
            state_guard.status = "error".to_string();
            state_guard.progress_message = Some(format!("Error: {}", e));
        }
    });
    
    Ok(Json(serde_json::json!({
        "status": "processing",
        "run_id": run_id
    })))
}

async fn process_pipeline(
    state: SharedState,
    db_pool: DbPool,
    run_id: String,
    jurisdiction: Jurisdiction,
    language: String,
    industry: String,
    _deep_mode: bool,
) -> Result<()> {
    // Step 1: Initialize database run
    {
        let mut conn = db_pool.lock().unwrap();
        create_run(&mut conn, &run_id, &format!("{:?}", jurisdiction), &language, &industry)?;
    }
    
    // Step 2: Load dictionary and initialize engines
    {
        let mut state_guard = state.lock().await;
        state_guard.current_step = 2;
        state_guard.progress_message = Some("Loading emission factor dictionary...".to_string());
    }
    
    let mut triage_engine = TriageEngine::new();
    let dict_content = std::fs::read_to_string("data/dictionary.json")?;
    triage_engine.load_from_json(&dict_content)?;
    
    let ingestion_engine = IngestionEngine::new();
    let mut ledger_processor = LedgerProcessor::new();
    let aggregator = Aggregator::new();
    
    // Step 3: Ingest files
    let staged_files = {
        let state_guard = state.lock().await;
        state_guard.staged_files.clone()
    };
    
    let mut all_raw_rows: Vec<RawRow> = Vec::new();
    for file_name in staged_files {
        let file_path = std::path::Path::new("/tmp").join(&file_name);
        let rows = ingestion_engine.parse_to_raw_rows(&file_path)?;
        all_raw_rows.extend(rows);
    }
    
    {
        let mut state_guard = state.lock().await;
        state_guard.current_step = 3;
        state_guard.progress_message = Some(format!("Processing {} rows...", all_raw_rows.len()));
    }
    
    // Step 4: Process rows into ledger/quarantine
    let mut ledger_rows = Vec::new();
    let mut quarantine_rows = Vec::new();
    
    for raw_row in all_raw_rows {
        if let Some(result) = ledger_processor.process_row(&run_id, &raw_row, &mut triage_engine, jurisdiction).await? {
            match result {
                ProcessResult::Ledger(row) => {
                    ledger_rows.push(row);
                }
                ProcessResult::Quarantine(row) => {
                    eprintln!("Row quarantined: {} - Reason: {:?} - Fix: {:?}", row.raw_header, row.error_reason, row.suggested_fix);
                    quarantine_rows.push(row);
                }
            }
        }
    }
    
    {
        let mut state_guard = state.lock().await;
        state_guard.current_step = 4;
        state_guard.progress_message = Some("Writing to database...".to_string());
    }
    
    // Step 5: Write to database
    {
        let mut conn = db_pool.lock().unwrap();
        for row in &ledger_rows {
            insert_ledger_row(&mut conn, &run_id, row)?;
        }
        for row in &quarantine_rows {
            insert_quarantine_row(&mut conn, &run_id, row)?;
        }
        update_run_status(&mut conn, &run_id, "completed")?;
    }
    
    // Step 6: Aggregate results
    let aggregation = aggregator.aggregate(&ledger_rows, quarantine_rows.len());
    
    let scope3_breakdown: HashMap<u8, crate::models::Scope3CategorySummary> = aggregation
        .scope3_breakdown
        .iter()
        .map(|(id, summary)| (*id, summary.clone()))
        .collect();
    
    // Step 7: Generate narrative (Gemini)
    let gemini_api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
    let gemini_client = GeminiClient::new(gemini_api_key);
    let narrative = gemini_client
        .generate_narrative(
            &aggregation,
            jurisdiction,
            &language,
            &industry,
            &scope3_breakdown,
        )
        .await;
    
    // Step 8: Generate ZIP package
    let output_factory = OutputFactory::new();
    let (emp_count, rev_eur) = {
        let state_guard = state.lock().await;
        (state_guard.employee_count, state_guard.revenue_eur)
    };
    let zip_data = output_factory
        .generate_fritz_package(
            &run_id,
            &ledger_rows,
            &quarantine_rows,
            &aggregation,
            &scope3_breakdown,
            &narrative,
            &format!("{:?}", jurisdiction),
            &language,
            emp_count,
            rev_eur,
        )
        .await?;
    
    // Step 9: Update final state
    {
        let mut state_guard = state.lock().await;
        state_guard.status = "finished".to_string();
        state_guard.current_step = 6;
        state_guard.ledger = ledger_rows;
        state_guard.quarantine = quarantine_rows;
        state_guard.scope3_breakdown = scope3_breakdown;
        state_guard.zip_package = Some(zip_data);
        state_guard.total_tco2e = Some(aggregation.total_tco2e);
        state_guard.progress_message = Some("Processing complete.".to_string());
    }
    
    Ok(())
}

pub async fn status_handler(
    State(state): State<SharedState>,
) -> Result<Json<StatusResponse>, AppError> {
    let state_guard = state.lock().await;
    
    let total_rows = state_guard.ledger.len() + state_guard.quarantine.len();
    let green_rows = state_guard.ledger.iter().filter(|r| r.confidence >= 0.9).count();
    let yellow_rows = state_guard.ledger.iter().filter(|r| r.confidence < 0.9).count();
    let red_rows = state_guard.quarantine.len();
    
    let progress = state_guard.current_step as f32 / 6.0;
    
    Ok(Json(StatusResponse {
        status: state_guard.status.clone(),
        current_step: state_guard.current_step,
        progress,
        total_rows,
        green_rows,
        yellow_rows,
        red_rows,
        scope3_categories_covered: state_guard.scope3_breakdown.len(),
        total_tco2e: state_guard.total_tco2e,
        message: state_guard.progress_message.clone(),
    }))
}

pub async fn results_handler(
    State(state): State<SharedState>,
) -> Result<Json<ResultsResponse>, AppError> {
    let state_guard = state.lock().await;
    
    if state_guard.status != "finished" {
        return Err(AppError::NotReady);
    }
    
    let scope1_tco2e = state_guard
        .ledger
        .iter()
        .filter(|r| matches!(r.ghg_scope, crate::models::GhgScope::SCOPE1))
        .map(|r| r.tco2e)
        .sum();
    
    let scope2_lb_tco2e = state_guard
        .ledger
        .iter()
        .filter(|r| matches!(r.ghg_scope, crate::models::GhgScope::SCOPE2_LB))
        .map(|r| r.tco2e)
        .sum();
    
    let scope2_mb_tco2e = state_guard
        .ledger
        .iter()
        .filter(|r| matches!(r.ghg_scope, crate::models::GhgScope::SCOPE2_MB))
        .map(|r| r.tco2e)
        .sum();
    
    let scope3_tco2e = state_guard
        .ledger
        .iter()
        .filter(|r| matches!(r.ghg_scope, crate::models::GhgScope::SCOPE3))
        .map(|r| r.tco2e)
        .sum();
    
    let mut dq_breakdown = HashMap::new();
    dq_breakdown.insert("Primary".to_string(), state_guard.ledger.iter().filter(|r| r.confidence >= 0.9).count());
    dq_breakdown.insert("Secondary".to_string(), state_guard.ledger.iter().filter(|r| r.confidence >= 0.7 && r.confidence < 0.9).count());
    dq_breakdown.insert("Estimated".to_string(), state_guard.ledger.iter().filter(|r| r.confidence < 0.7).count());
    
    let csrd_completeness_pct = (state_guard.scope3_breakdown.len() as f32 / 15.0) * 100.0;
    
    // Compliance Check
    let validator = OmnibusValidator::new(state_guard.employee_count, state_guard.revenue_eur, None);
    let obligation = validator.is_csrd_obligated();
    let has_financial = state_guard.ledger.iter().any(|r| r.scope3_extension.as_ref().map(|e| e.category_id) == Some(15));
    let reporting_scope = validator.get_reporting_scope(has_financial);
    
    let compliance = Some(crate::models::ComplianceInfo {
        obligation_status: format!("{:?}", obligation),
        reporting_scope: format!("{:?}", reporting_scope),
        employee_count: state_guard.employee_count,
        revenue_eur: state_guard.revenue_eur,
        threshold_met: matches!(obligation, ObligationStatus::Obligated),
    });
    
    let mut risk_metrics = None;
    if state_guard.industry.as_deref() == Some("Financial") {
        let assets: Vec<PortfolioAsset> = state_guard.ledger.iter()
            .filter(|r| r.ghg_scope == crate::models::GhgScope::SCOPE3 && r.scope3_extension.as_ref().map(|e| e.category_id) == Some(15))
            .map(|r| PortfolioAsset {
                investment_amount: r.raw_value,
                emissions_tco2e: r.tco2e,
                revenue_meur: r.raw_value / 10.0,
            })
            .collect();
        
        let total_value: f64 = assets.iter().map(|a| a.investment_amount).sum();
        risk_metrics = Some(CarbonRiskMetrics::calculate(&assets, total_value));
    }

    // Benchmarking
    let industry_str = state_guard.industry.as_deref().unwrap_or("General");
    let benchmark = IndustryBenchmark::get_for_sector(industry_str);
    
    let client_revenue = state_guard.revenue_eur.unwrap_or(10_000_000.0);
    let client_intensity = state_guard.total_tco2e.unwrap_or(0.0) / (client_revenue / 1_000_000.0);
    
    let comparison = PeerComparison::new(client_intensity, benchmark.avg_carbon_intensity);
    
    let benchmarking = Some(crate::models::BenchmarkingInfo {
        sector: industry_str.to_string(),
        client_intensity,
        industry_average: benchmark.avg_carbon_intensity,
        percentile: comparison.percentile.unwrap_or(50),
        performance_tier: format!("{:?}", comparison.performance_tier),
        narrative: comparison.generate_narrative(),
    });

    Ok(Json(ResultsResponse {
        run_id: state_guard.run_id.clone().unwrap_or_default(),
        total_tco2e: state_guard.total_tco2e.unwrap_or(0.0),
        scope1_tco2e,
        scope2_lb_tco2e,
        scope2_mb_tco2e,
        scope3_tco2e,
        scope3_breakdown: state_guard.scope3_breakdown.values().cloned().collect(),
        data_quality_tier_breakdown: dq_breakdown,
        quarantine_count: state_guard.quarantine.len(),
        csrd_completeness_pct,
        risk_metrics,
        compliance,
        benchmarking,
    }))
}

pub async fn download_handler(
    State(state): State<SharedState>,
) -> Result<impl IntoResponse, AppError> {
    let mut state_guard = state.lock().await;
    
    if let Some(zip_data) = state_guard.zip_package.take() {
        let filename = format!(
            "attachment; filename=\"TargooV2_Fritz_Package_{}.zip\"",
            state_guard.run_id.as_deref().unwrap_or("report")
        );
        let headers = [
            ("Content-Type", "application/zip".to_string()),
            ("Content-Disposition", filename),
        ];
        Ok((headers, zip_data))
    } else {
        Err(AppError::NotFound)
    }
}

#[derive(Debug)]
pub enum AppError {
    BadRequest,
    InternalError,
    NotReady,
    NotFound,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest => (StatusCode::BAD_REQUEST, "Bad request"),
            AppError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
            AppError::NotReady => (StatusCode::SERVICE_UNAVAILABLE, "Processing not complete"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found"),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
