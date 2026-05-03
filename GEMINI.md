# Targoo V2 Project Instructions

## Core Mandates
- **Language:** Rust (Stable)
- **Error Handling:** NEVER use `.unwrap()` or `.expect()`. Always use idiomatic error handling (e.g., `Result` with `anyhow` or `thiserror`).
- **Robustness:** The motor must never panic.
- **Auditability:** All transformations must be traceable.
- **Data Integrity:** Use SQLite (WORM - Write Once Read Many pattern) for persistence where applicable.
- **Architecture:** Headless "ESG Data Refinery Motor".

## Workflows
- **Ingestion:** Handle raw Excel, CSV, ERP dumps.
- **Triage:** Support multi-language header recognition (DACH region priority).
- **Calculation:** Scope 1, 2, and 3 emissions.
- **Output:** Generate "Fritz Package" (audit-ready report package).
