# Targoo V2 · Industrial ESG Data Refinery

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/python-3.10%2B-blue)](https://www.python.org/)
[![License](https://img.shields.io/badge/license-Proprietary-red)](LICENSE)

Targoo V2 is a high-assurance ESG data processing engine that transforms messy CSV/XLSX files into CSRD/ESRS E1 compliant carbon footprint reports. It implements the complete GHG Protocol Corporate Standard with full Scope 3 category coverage (1-15).

**Price:** $2,000 per run · **Target:** DACH/US/UK ESG Consultants

---

## 🚀 Quick Start (GitHub Codespaces)

```bash
# 1. Setup AI Bridge (Python)
cd ai && ./setup.sh

# 2. Start AI Bridge (Terminal 1)
cd ai && source venv/bin/activate && python3 bridge.py

# 3. Start Rust Backend (Terminal 2)
cargo run --release

# 4. Serve Frontend (Terminal 3)
python3 -m http.server 3000 --directory frontend
```

Access the application at **http://localhost:3000**

---

## 📁 Tech Stack

| Component   | Technology                    | Port |
|-------------|-------------------------------|------|
| Backend     | Rust + Axum + Tokio + SQLite  | 8080 |
| Frontend    | Single HTML + Vue.js (vanilla)| 3000 |
| AI Bridge   | Python + FastAPI + Transformers| 9000 |
| AI Model    | sentence-transformers/MiniLM  | 90MB |

---

## 🔧 Development Setup

### Prerequisites

- Rust 1.75+ with `cargo`
- Python 3.10+ with `pip` and `venv`
- SQLite 3 (bundled with `rusqlite`)

### Backend

```bash
# Build
cargo build --release

# Run tests
cargo test

# Watch mode (development)
cargo watch -x run
```

### AI Bridge

```bash
cd ai

# First time setup
./setup.sh

# Activate and run
source venv/bin/activate
python3 bridge.py
```

The bridge loads the `sentence-transformers/all-MiniLM-L12-v2` model (~90MB) and pre-computed vector indices.

### Dictionary

The dictionary is located at `data/dictionary.json` and contains 75+ entries covering:
- Scope 1: 8+ entries (EN/DE/HU)
- Scope 2: 5+ entries (EN/DE/HU)
- Scope 3: All 15 categories with 3-7 entries each (EN/DE/HU)

---

## 📡 API Endpoints

| Method | Endpoint   | Description                                    |
|--------|------------|------------------------------------------------|
| POST   | `/upload`  | Upload CSV/XLSX files (multipart/form-data)    |
| POST   | `/run`     | Start processing pipeline                      |
| GET    | `/status`  | Poll pipeline progress and live counters       |
| GET    | `/results` | Get final aggregation and Scope 3 breakdown    |
| GET    | `/download`| Download Fritz Package ZIP (7 audit files)     |
| GET    | `/health`  | Backend health check                           |

---

## 📦 Fritz Package Structure

The ZIP download contains 7 audit-ready files:

1. **00_Manifest.json** — SHA-256 chain, methodology, scope3_coverage_map
2. **01_GHG_Inventar_Zusammenfassung.xlsx** — A4 summary with signature field
3. **02_Scope_Aufschluesselung.xlsx** — 4 worksheets including Scope 3 Kategorien
4. **03_Audit_Trail_Master.xlsx** — Full immutable ledger (GREEN/YELLOW rows)
5. **04_Quarantaene_Log.xlsx** — Quarantined rows with correction column
5. **05_Emissionsfaktoren_Referenz.xlsx** — All EFs and GWP100 table
6. **06_Narrative_Bericht.docx** — Gemini-generated executive summary

---

## 🔒 Security & Auditability

- **WORM Database:** SQLite triggers prevent UPDATE/DELETE on ledger and quarantine tables
- **SHA-256 Chain:** Every ledger row includes previous row's hash (blockchain-style)
- **Immutable Run ID:** UUID v4 generated per pipeline execution
- **Range Guards:** Category-specific tCO2e limits prevent anomalous values

---

## 📄 License

Proprietary — Targoo GmbH. All rights reserved.

---

## 🆘 Troubleshooting

| Issue | Solution |
|-------|----------|
| AI Bridge fails to start | Run `./ai/setup.sh` to rebuild indices |
| Port 8080 already in use | `lsof -i :8080` and `kill -9 <PID>` |
| Gemini API timeout | Fallback narrative is generated automatically |
| Dictionary not found | Ensure `data/dictionary.json` exists and is valid JSON |

---

**Built for CSRD/ESRS E1 Compliance · GHG Protocol Scope 3 Standard (2011)**
