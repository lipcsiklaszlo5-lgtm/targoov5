use crate::config::models::*;
use uuid::Uuid;
use chrono::Utc;

pub fn validate(config: RunConfig) -> Result<ValidatedConfig, String> {
    // ── 1. Üres module lista ──────────────────────────────────
    if config.modules.is_empty() {
        return Err("Legalább egy ComplianceModule szükséges.".into());
    }

    // ── 2. Kötelező modulok hozzáadása (joghatóság alapján) ──
    let mut active_modules = config.modules.clone();
    for mandatory in config.jurisdiction.mandatory_modules() {
        if !active_modules.contains(&mandatory) {
            eprintln!(
                "[CCE] ℹ {:?} joghatósághoz {:?} modul automatikusan hozzáadva.",
                config.jurisdiction, mandatory
            );
            active_modules.push(mandatory);
        }
    }

    // ── 3. EF forrás meghatározás ────────────────────────────
    let ef_source = config.ef_source_override
        .clone()
        .unwrap_or_else(|| config.jurisdiction.preferred_ef_source().to_string());

    // ── 4. Szótár útvonalak összerakása ──────────────────────
    let lang_suffix = config.language.triage_dictionary_suffix();
    let dictionary_paths = vec![
        format!("data/dictionary_{}.json", lang_suffix),
        format!("data/dictionary_en.json"), // fallback mindig
    ];

    // ── 5. Output ZIP neve ───────────────────────────────────
    let primary_module = active_modules.first()
        .map(|m| m.output_label())
        .unwrap_or("GHG_Report");
    let output_label = format!(
        "Fritz_Package_{}_{}_{}.zip",
        config.client.name.replace(' ', "_"),
        config.client.reference_year,
        primary_module
    );

    // ── 6. AI depth konzisztencia ────────────────────────────
    if config.depth == CalculationDepth::Quick && config.ai.gemini_enabled {
        eprintln!(
            "[CCE] ⚠ Quick depth + Gemini enabled — Gemini ki lesz kapcsolva."
        );
    }

    Ok(ValidatedConfig {
        run_id: config.run_id.unwrap_or_else(Uuid::new_v4),
        config,
        ef_source,
        active_modules,
        dictionary_paths,
        output_label,
    })
}
