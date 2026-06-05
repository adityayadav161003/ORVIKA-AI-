import argparse
import sys
import json
import os
import traceback
from typing import List, Dict, Any, Optional

# Suppress HuggingFace hub symlink warnings on Windows
os.environ["HF_HUB_DISABLE_SYMLINKS_WARNING"] = "1"

# Initialize FastAPI
try:
    from fastapi import FastAPI, HTTPException
    from pydantic import BaseModel
    import uvicorn
except ImportError:
    print(json.dumps({"status": "error", "error": "fastapi, uvicorn, or pydantic not installed"}), flush=True)
    sys.exit(1)

app = FastAPI(title="ORVIKA AI Persistent Service")

# Global models and Chroma client references
embedding_model = None
rerank_model = None
chroma_client = None
chroma_collection = None

# Pydantic schemas
class EmbedRequest(BaseModel):
    texts: List[str]

class RerankRequest(BaseModel):
    query: str
    candidates: List[str]

class VectorAddRequest(BaseModel):
    ids: List[str]
    embeddings: List[List[float]]
    metadatas: List[Dict[str, Any]]
    documents: List[str]

class VectorQueryRequest(BaseModel):
    query_embedding: List[float]
    n_results: int
    where: Optional[Dict[str, Any]] = None

class VectorDeleteRequest(BaseModel):
    where: Optional[Dict[str, Any]] = None

@app.get("/health")
def health():
    return {"status": "healthy"}

@app.post("/embed")
def embed(req: EmbedRequest):
    global embedding_model
    if embedding_model is None:
        raise HTTPException(status_code=500, detail="Embedding model not loaded")
    try:
        embeddings = embedding_model.encode(req.texts, convert_to_numpy=True)
        return {"embeddings": embeddings.tolist()}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Embedding failed: {str(e)}")

@app.post("/rerank")
def rerank(req: RerankRequest):
    global rerank_model
    if rerank_model is None:
        raise HTTPException(status_code=500, detail="Reranker model not loaded")
    if not req.candidates:
        return {"scores": []}
    try:
        # CrossEncoder expects pairs of (query, candidate)
        pairs = [(req.query, cand) for cand in req.candidates]
        scores = rerank_model.predict(pairs, convert_to_numpy=True)
        # Convert float32/numpy objects to standard Python floats for JSON serialization
        return {"scores": [float(score) for score in scores.tolist()]}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Reranking failed: {str(e)}")

@app.post("/vector/add")
def vector_add(req: VectorAddRequest):
    global chroma_collection
    if chroma_collection is None:
        raise HTTPException(status_code=500, detail="Chroma DB collection not initialized")
    try:
        chroma_collection.add(
            ids=req.ids,
            embeddings=req.embeddings,
            metadatas=req.metadatas,
            documents=req.documents
        )
        return {"status": "success"}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to add vectors to Chroma: {str(e)}")

@app.post("/vector/query")
def vector_query(req: VectorQueryRequest):
    global chroma_collection
    if chroma_collection is None:
        raise HTTPException(status_code=500, detail="Chroma DB collection not initialized")
    try:
        results = chroma_collection.query(
            query_embeddings=[req.query_embedding],
            n_results=req.n_results,
            where=req.where
        )
        # Chroma results return lists of lists. We extract the first element since we query one vector.
        return {
            "ids": results.get("ids", [[]])[0],
            "distances": results.get("distances", [[]])[0],
            "metadatas": results.get("metadatas", [[]])[0],
            "documents": results.get("documents", [[]])[0]
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Vector query failed: {str(e)}")

@app.post("/vector/delete")
def vector_delete(req: VectorDeleteRequest):
    global chroma_collection
    if chroma_collection is None:
        raise HTTPException(status_code=500, detail="Chroma DB collection not initialized")
    try:
        # Chroma requires either ids or a where filter
        if req.where:
            chroma_collection.delete(where=req.where)
        else:
            raise HTTPException(status_code=400, detail="Delete request must contain a where filter")
        return {"status": "success"}
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Vector delete failed: {str(e)}")

def main():
    parser = argparse.ArgumentParser(description="ORVIKA AI Local sidecar server")
    parser.add_argument("--host", default="127.0.0.1", help="Host address")
    parser.add_argument("--port", type=int, default=8082, help="Port to run FastAPI server")
    parser.add_argument("--chroma-path", required=True, help="Directory path to persist Chroma DB")
    args = parser.parse_args()

    # Disable HF telemetry and ensure we load offline if models are cached
    global embedding_model, rerank_model, chroma_client, chroma_collection
    
    print("Loading local AI models...", flush=True)
    try:
        from sentence_transformers import SentenceTransformer
        # Load local embedding model (768-dim output)
        embedding_model = SentenceTransformer('BAAI/bge-base-en-v1.5')
        print("Embedding model BAAI/bge-base-en-v1.5 loaded.", flush=True)
    except Exception as e:
        print(f"Error loading embedding model: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

    try:
        from sentence_transformers import CrossEncoder
        # Load local cross-encoder model for reranking
        rerank_model = CrossEncoder('BAAI/bge-reranker-base')
        print("Rerank model BAAI/bge-reranker-base loaded.", flush=True)
    except Exception as e:
        print(f"Error loading rerank model: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

    print(f"Initializing local ChromaDB at {args.chroma-path}...", flush=True)
    try:
        import chromadb
        from chromadb.config import Settings
        # Secrecy/privacy guarantee: anonymized_telemetry = False MUST be set
        chroma_client = chromadb.PersistentClient(
            path=args.chroma_path,
            settings=Settings(anonymized_telemetry=False)
        )
        chroma_collection = chroma_client.get_or_create_collection("orvika_chunks")
        print("ChromaDB initialized and collection 'orvika_chunks' ready.", flush=True)
    except Exception as e:
        print(f"Error initializing ChromaDB: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

    print(f"Starting server on {args.host}:{args.port}", flush=True)
    uvicorn.run(app, host=args.host, port=args.port, log_level="info")

if __name__ == "__main__":
    main()
