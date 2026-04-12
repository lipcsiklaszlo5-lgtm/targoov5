#!/bin/bash
set -e

echo "=========================================="
echo "Targoo V2 AI Bridge Setup (Ollama Edition)"
echo "=========================================="

cd "$(dirname "$0")"

# Create venv
if [ ! -d "venv" ]; then
    echo "Creating Python virtual environment..."
    python3 -m venv venv
fi

source venv/bin/activate
pip install --upgrade pip --quiet

# Install dependencies (NO sentence-transformers!)
echo "Installing dependencies..."
pip install -r requirements_ollama.txt --quiet

# Check Ollama installation
if ! command -v ollama &> /dev/null; then
    echo ""
    echo "⚠️  Ollama is not installed!"
    echo "   Install with: curl -fsSL https://ollama.com/install.sh | sh"
    echo ""
else
    echo "✅ Ollama found"
    
    # Check if model is pulled
    if ollama list | grep -q "all-minilm:l12-v2"; then
        echo "✅ Model 'all-minilm:l12-v2' found"
    else
        echo ""
        echo "⚠️  Model not pulled yet!"
        echo "   Pull with: ollama pull all-minilm:l12-v2"
        echo "   (This is a 67MB download, takes ~1 minute)"
        echo ""
    fi
fi

# Build indices if dictionary exists
if [ -f "../data/dictionary.json" ]; then
    echo "Building vector indices..."
    python build_index.py --dict "../data/dictionary.json" 2>/dev/null || echo "⚠️ Index build skipped (model not needed for this step)"
fi

echo ""
echo "=========================================="
echo "Setup complete!"
echo "=========================================="
echo ""
echo "To start the AI bridge:"
echo "  source venv/bin/activate"
echo "  python bridge.py"
echo ""
echo "Make sure Ollama is running in another terminal:"
echo "  ollama serve"
