import sys
import json
import traceback
import os

# Suppress HuggingFace hub symlink warnings on Windows
os.environ["HF_HUB_DISABLE_SYMLINKS_WARNING"] = "1"

def main():
    try:
        from sentence_transformers import SentenceTransformer
        # Load standard small, fast embedding model
        model = SentenceTransformer('all-MiniLM-L6-v2')
    except ImportError:
        print(json.dumps({"status": "error", "error": "sentence-transformers not installed"}), flush=True)
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"status": "error", "error": f"Model load failed: {str(e)}"}), flush=True)
        sys.exit(1)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
            
        try:
            req = json.loads(line)
            chunks = req.get("chunks", [])
            
            if not chunks:
                print(json.dumps({"status": "success", "embeddings": []}), flush=True)
                continue
            
            # Embed all chunks at once
            embeddings = model.encode(chunks, convert_to_numpy=True)
            
            # Convert numpy array to list of lists of floats for JSON serialization
            embeddings_list = embeddings.tolist()
            
            print(json.dumps({
                "status": "success",
                "embeddings": embeddings_list
            }), flush=True)
            
        except Exception as e:
            err = traceback.format_exc()
            print(json.dumps({
                "status": "error",
                "error": str(e),
                "traceback": err
            }), flush=True)

if __name__ == "__main__":
    main()
