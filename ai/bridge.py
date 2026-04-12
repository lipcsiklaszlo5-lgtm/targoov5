#!/usr/bin/env python3
"""
Targoo V2 AI Bridge - OLLAMA EDITION
Uses Ollama API for embeddings instead of HuggingFace.
No rate limits, no HF dependencies, 67MB model.
"""

import os
import json
import time
import asyncio
from typing import Optional, List, Dict, Any
from contextlib import asynccontextmanager

import numpy as np
import httpx
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

# --- Configuration ---
OLLAMA_URL = os.getenv("OLLAMA_URL", "http://localhost:11434")
OLLAMA_MODEL = "all-minilm:l12-v2"  # 67MB, 384 dims
INDEX_PATH = os.path.join(os.path.dirname(__file__), "esg_index.json")
SCOPE3_INDEX_PATH = os.path.join(os.path.dirname(__file__), "scope3_index.json")
PORT = int(os.getenv("AI_BRIDGE_PORT", 9000))

# --- Global state ---
keyword_vectors = None
keyword_metadata = []
scope3_vectors = None
scope3_metadata = []
ollama_available = False

# --- Pydantic Models (PONTOSAN UGYANAZ, MINT KORÁBBAN) ---
class ClassifyRequest(BaseModel):
    query: str
    top_k: int = 1

class ClassifyResponse(BaseModel):
    matched: bool
    ghg_category: Optional[str] = None
    scope3_id: Optional[int] = None
    canonical_unit: Optional[str] = None
    ef_value: Optional[float] = None
    calc_path: Optional[str] = None
    confidence: float
    matched_keyword: Optional[str] = None
    method: str = "semantic"

class HealthResponse(BaseModel):
    status: str
    embed_ready: bool
    index_size: int
    scope3_index_size: int
    ram_used_gb: float
    model_name: str

# --- Helper Functions ---
def cosine_similarity(query_vec: np.ndarray, corpus_vectors: np.ndarray) -> np.ndarray:
    if corpus_vectors.shape[0] == 0:
        return np.array([])
    query_norm = query_vec / np.linalg.norm(query_vec)
    corpus_norms = corpus_vectors / np.linalg.norm(corpus_vectors, axis=1, keepdims=True)
    return np.dot(corpus_norms, query_norm)

async def get_embedding(text: str) -> Optional[List[float]]:
    """Get embedding from Ollama API"""
    if not ollama_available:
        return None
    
    try:
        async with httpx.AsyncClient(timeout=10.0) as client:
            response = await client.post(
                f"{OLLAMA_URL}/api/embeddings",
                json={"model": OLLAMA_MODEL, "prompt": text}
            )
            if response.status_code == 200:
                data = response.json()
                return data.get("embedding")
            else:
                print(f"Ollama error: {response.status_code}")
                return None
    except Exception as e:
        print(f"Ollama request failed: {e}")
        return None

async def check_ollama_health() -> bool:
    """Check if Ollama is running and model is available"""
    try:
        async with httpx.AsyncClient(timeout=5.0) as client:
            # Check Ollama server
            response = await client.get(f"{OLLAMA_URL}/api/tags")
            if response.status_code != 200:
                return False
            
            # Check if our model is available
            models = response.json().get("models", [])
            model_names = [m.get("name") for m in models]
            return OLLAMA_MODEL in model_names or f"{OLLAMA_MODEL}:latest" in model_names
    except Exception:
        return False

# --- Lifespan ---
@asynccontextmanager
async def lifespan(app: FastAPI):
    global keyword_vectors, keyword_metadata, scope3_vectors, scope3_metadata, ollama_available
    
    # Check Ollama availability
    print(f"Checking Ollama at {OLLAMA_URL}...")
    ollama_available = await check_ollama_health()
    
    if ollama_available:
        print(f"✅ Ollama connected, model '{OLLAMA_MODEL}' available")
    else:
        print(f"⚠️ Ollama not available at {OLLAMA_URL}")
        print("   Install: curl -fsSL https://ollama.com/install.sh | sh")
        print(f"   Pull model: ollama pull {OLLAMA_MODEL}")
    
    # Load indices (same as before)
    if os.path.exists(INDEX_PATH):
        with open(INDEX_PATH, 'r') as f:
            data = json.load(f)
            keyword_vectors = np.array(data["vectors"], dtype=np.float32) if data["vectors"] else np.empty((0, 384))
            keyword_metadata = data["metadata"]
        print(f"Loaded {len(keyword_metadata)} keyword vectors")
    else:
        keyword_vectors = np.empty((0, 384), dtype=np.float32)
        keyword_metadata = []
        print("⚠️ esg_index.json not found")
    
    if os.path.exists(SCOPE3_INDEX_PATH):
        with open(SCOPE3_INDEX_PATH, 'r') as f:
            data = json.load(f)
            scope3_vectors = np.array(data["vectors"], dtype=np.float32) if data["vectors"] else np.empty((0, 384))
            scope3_metadata = data["metadata"]
        print(f"Loaded {len(scope3_metadata)} Scope 3 vectors")
    else:
        scope3_vectors = np.empty((0, 384), dtype=np.float32)
        scope3_metadata = []
        print("⚠️ scope3_index.json not found")
    
    yield
    print("Shutting down AI bridge...")

# --- FastAPI App ---
app = FastAPI(title="Targoo V2 AI Bridge (Ollama Edition)", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

@app.get("/health", response_model=HealthResponse)
async def health():
    import psutil
    process = psutil.Process()
    ram_used = process.memory_info().rss / (1024 ** 3)
    
    return HealthResponse(
        status="healthy" if ollama_available else "degraded",
        embed_ready=ollama_available,
        index_size=len(keyword_metadata),
        scope3_index_size=len(scope3_metadata),
        ram_used_gb=round(ram_used, 2),
        model_name=OLLAMA_MODEL if ollama_available else "unavailable"
    )

@app.post("/classify_batch")
async def classify_batch(headers: List[str]):
    if not ollama_available:
        return [{"matched": False, "confidence": 0.0, "method": "ollama_unavailable"} for _ in headers]
    
    results = []
    for header in headers:
        query = header.strip().lower()
        if not query:
            results.append({"matched": False, "confidence": 0.0, "method": "ollama"})
            continue
            
        embedding = await get_embedding(query)
        if embedding is None:
            results.append({"matched": False, "confidence": 0.0, "method": "ollama_error"})
            continue
            
        query_embedding = np.array(embedding, dtype=np.float32)
        best_score = 0.0
        best_meta = None
        
        # Search general ESG index
        if keyword_vectors.shape[0] > 0:
            similarities = cosine_similarity(query_embedding, keyword_vectors)
            top_idx = int(np.argmax(similarities))
            score = float(similarities[top_idx])
            if score > best_score:
                best_score = score
                best_meta = keyword_metadata[top_idx].copy()
        
        # Search Scope 3 index
        if scope3_vectors.shape[0] > 0:
            similarities = cosine_similarity(query_embedding, scope3_vectors)
            top_idx = int(np.argmax(similarities))
            score = float(similarities[top_idx])
            if score > best_score:
                best_score = score
                best_meta = scope3_metadata[top_idx].copy()
        
        CONFIDENCE_THRESHOLD = 0.35
        if best_score >= CONFIDENCE_THRESHOLD and best_meta is not None:
            results.append({
                "matched": True,
                "ghg_category": best_meta.get("ghg_category"),
                "scope3_id": best_meta.get("scope3_id"),
                "canonical_unit": best_meta.get("canonical_unit"),
                "ef_value": best_meta.get("ef_value"),
                "calc_path": best_meta.get("calc_path"),
                "confidence": round(best_score, 4),
                "matched_keyword": best_meta.get("keyword"),
                "method": "ollama"
            })
        else:
            results.append({
                "matched": False,
                "confidence": round(best_score, 4),
                "method": "ollama"
            })
    return results

@app.post("/classify", response_model=ClassifyResponse)
async def classify(request: ClassifyRequest):
    if not ollama_available:
        return ClassifyResponse(
            matched=False,
            confidence=0.0,
            method="ollama_unavailable"
        )
    
    query = request.query.strip().lower()
    if not query:
        return ClassifyResponse(matched=False, confidence=0.0, method="ollama")
    
    # Get embedding from Ollama
    embedding = await get_embedding(query)
    if embedding is None:
        return ClassifyResponse(matched=False, confidence=0.0, method="ollama_error")
    
    query_embedding = np.array(embedding, dtype=np.float32)
    
    best_score = 0.0
    best_meta = None
    
    # Search general ESG index
    if keyword_vectors.shape[0] > 0:
        similarities = cosine_similarity(query_embedding, keyword_vectors)
        top_idx = int(np.argmax(similarities))
        score = float(similarities[top_idx])
        if score > best_score:
            best_score = score
            best_meta = keyword_metadata[top_idx].copy()
    
    # Search Scope 3 index (prioritize if score is good)
    if scope3_vectors.shape[0] > 0:
        similarities = cosine_similarity(query_embedding, scope3_vectors)
        top_idx = int(np.argmax(similarities))
        score = float(similarities[top_idx])
        if score > best_score:
            best_score = score
            best_meta = scope3_metadata[top_idx].copy()
    
    CONFIDENCE_THRESHOLD = 0.35
    
    if best_score >= CONFIDENCE_THRESHOLD and best_meta is not None:
        return ClassifyResponse(
            matched=True,
            ghg_category=best_meta.get("ghg_category"),
            scope3_id=best_meta.get("scope3_id"),
            canonical_unit=best_meta.get("canonical_unit"),
            ef_value=best_meta.get("ef_value"),
            calc_path=best_meta.get("calc_path"),
            confidence=round(best_score, 4),
            matched_keyword=best_meta.get("keyword"),
            method="ollama"
        )
    else:
        return ClassifyResponse(
            matched=False,
            confidence=round(best_score, 4),
            method="ollama"
        )

if __name__ == "__main__":
    uvicorn.run("bridge:app", host="0.0.0.0", port=PORT, reload=False)
