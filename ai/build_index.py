#!/usr/bin/env python3
import os
import json
import argparse
import asyncio
import numpy as np
import httpx

OLLAMA_URL = os.getenv("OLLAMA_URL", "http://localhost:11434")
OLLAMA_MODEL = "all-minilm:l12-v2"

async def get_embedding(text: str):
    async with httpx.AsyncClient(timeout=30.0) as client:
        try:
            response = await client.post(
                f"{OLLAMA_URL}/api/embeddings",
                json={"model": OLLAMA_MODEL, "prompt": text}
            )
            if response.status_code == 200:
                return response.json().get("embedding")
        except Exception as e:
            print(f"Error getting embedding for '{text}': {e}")
    return None

async def build_index(dict_path: str):
    print(f"Loading dictionary from {dict_path}...")
    with open(dict_path, 'r') as f:
        dictionary = json.load(f)
    
    keyword_vectors = []
    keyword_metadata = []
    scope3_vectors = []
    scope3_metadata = []
    
    print(f"Processing {len(dictionary)} entries...")
    
    for entry in dictionary:
        kw = entry.get("keyword", "")
        if not kw: continue
        
        embedding = await get_embedding(kw)
        if embedding:
            meta = {
                "keyword": kw,
                "ghg_category": entry.get("ghg_category"),
                "scope3_id": entry.get("scope3_id"),
                "scope3_name": entry.get("scope3_name"),
                "canonical_unit": entry.get("canonical_unit"),
                "ef_value": entry.get("ef_value"),
                "calc_path": entry.get("calc_path")
            }
            
            if entry.get("ghg_category") == "Scope3":
                scope3_vectors.append(embedding)
                scope3_metadata.append(meta)
            else:
                keyword_vectors.append(embedding)
                keyword_metadata.append(meta)
                
        # Small sleep to not overwhelm Ollama
        await asyncio.sleep(0.05)
    
    # Save general ESG index
    index_path = os.path.join(os.path.dirname(__file__), "esg_index.json")
    with open(index_path, 'w') as f:
        json.dump({"vectors": keyword_vectors, "metadata": keyword_metadata}, f)
    
    # Save Scope 3 index
    scope3_index_path = os.path.join(os.path.dirname(__file__), "scope3_index.json")
    with open(scope3_index_path, 'w') as f:
        json.dump({"vectors": scope3_vectors, "metadata": scope3_metadata}, f)
        
    print(f"✅ Indices built! ESG: {len(keyword_metadata)}, Scope 3: {len(scope3_metadata)}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--dict", required=True)
    args = parser.parse_args()
    asyncio.run(build_index(args.dict))
