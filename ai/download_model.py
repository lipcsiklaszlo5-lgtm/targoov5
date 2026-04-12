#!/usr/bin/env python3
"""
One-time model download script for offline use.
Run this ONCE on a machine with good internet.
"""
import os
import sys

MODEL_ID = "sentence-transformers/all-MiniLM-L12-v2"
LOCAL_PATH = os.path.join(os.path.dirname(__file__), "models", "all-MiniLM-L12-v2")

def download_model():
    print(f"Downloading {MODEL_ID}...")
    print("This may take 2-5 minutes depending on your connection.")
    
    try:
        from sentence_transformers import SentenceTransformer
        
        # Create models directory
        os.makedirs(LOCAL_PATH, exist_ok=True)
        
        # Download and save
        model = SentenceTransformer(MODEL_ID)
        model.save(LOCAL_PATH)
        
        print(f"\n✅ SUCCESS! Model saved to {LOCAL_PATH}")
        print("You can now copy this folder to any server and run in offline mode.")
        
        # Test loading from local path
        print("\nTesting local load...")
        local_model = SentenceTransformer(LOCAL_PATH)
        test_embedding = local_model.encode(["Test sentence"])
        print(f"✅ Local load successful! Embedding shape: {test_embedding.shape}")
        
    except Exception as e:
        print(f"❌ Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    download_model()
