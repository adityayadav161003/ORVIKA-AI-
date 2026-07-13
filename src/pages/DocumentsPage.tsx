import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DocumentUpload } from "@/features/documents/components/DocumentUpload";
import { DocumentViewer } from "@/features/documents/components/DocumentViewer";
import { Button } from "@/components/ui/Button";

interface Document {
  id: string;
  filename: string;
  fileSize: number;
  fileType: string;
  chunkCount: number;
  createdAt: string;
  parsedAt: string | null;
  metadata?: string | null;
}

export function DocumentsPage() {
  const [documents, setDocuments] = useState<Document[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isRebuilding, setIsRebuilding] = useState(false);
  const [selectedDocument, setSelectedDocument] = useState<Document | null>(null);

  const loadDocuments = async () => {
    try {
      setIsLoading(true);
      const docs = await invoke<Document[]>("list_documents", { sessionId: null });
      setDocuments(docs || []);
    } catch (err) {
      console.error("Failed to load documents", err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRebuildIndex = async () => {
    if (
      !confirm(
        "Are you sure you want to rebuild the vector database index? This will re-embed all documents and may take some time.",
      )
    )
      return;
    try {
      setIsRebuilding(true);
      await invoke("rebuild_vector_store");
      alert("Vector database index has been successfully rebuilt!");
    } catch (err) {
      alert(
        "Failed to rebuild vector index: " + (err instanceof Error ? err.message : String(err)),
      );
    } finally {
      setIsRebuilding(false);
    }
  };

  useEffect(() => {
    loadDocuments();
  }, []);

  const handleDelete = async (id: string) => {
    if (!confirm("Are you sure you want to delete this document?")) return;
    try {
      await invoke("delete_document", { documentId: id });
      await loadDocuments();
    } catch (err) {
      console.error("Failed to delete document", err);
    }
  };

  return (
    <div className="flex flex-col h-full overflow-y-auto p-6 space-y-8">
      <div className="flex flex-col gap-2">
        <h2 className="font-serif text-3xl font-bold text-text-primary">Document Library</h2>
        <p className="text-text-secondary max-w-2xl">
          Upload documents to be parsed and chunked locally. These documents can be used in your
          research sessions and chats for Retrieval-Augmented Generation (RAG).
        </p>
      </div>

      <div className="max-w-xl">
        <DocumentUpload onUploadComplete={loadDocuments} />
      </div>

      <div className="flex flex-col space-y-4">
        <div className="flex items-center justify-between border-b border-border pb-2">
          <h3 className="font-medium text-lg text-text-primary">Your Documents</h3>
          <Button
            size="sm"
            variant="secondary"
            loading={isRebuilding}
            onClick={handleRebuildIndex}
            className="text-xs focus:ring-2 focus:ring-accent/40"
            aria-label="Rebuild all document vector embeddings"
          >
            Rebuild Index
          </Button>
        </div>

        {isLoading ? (
          <div className="text-text-muted">Loading documents...</div>
        ) : documents.length === 0 ? (
          <div className="text-text-muted italic">No documents uploaded yet.</div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {documents.map((doc) => (
              <div
                key={doc.id}
                className="flex flex-col p-4 border border-border rounded-lg bg-surface relative group cursor-pointer hover:border-accent/50 hover:shadow-md transition-all"
                onClick={() => setSelectedDocument(doc)}
              >
                <div className="flex items-start justify-between mb-2">
                  <h4 className="font-medium text-text-primary truncate pr-4" title={doc.filename}>
                    {doc.filename}
                  </h4>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(doc.id);
                    }}
                    className="text-text-muted hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity p-1 z-10 relative"
                    title="Delete Document"
                  >
                    ×
                  </button>
                </div>

                <div className="text-xs text-text-secondary space-y-1">
                  <p>Type: {doc.fileType.toUpperCase()}</p>
                  <p>Size: {(doc.fileSize / 1024 / 1024).toFixed(2)} MB</p>
                  <p>Status: {doc.parsedAt ? `Parsed (${doc.chunkCount} chunks)` : "Parsing..."}</p>
                  <p>Uploaded: {new Date(doc.createdAt).toLocaleDateString()}</p>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {selectedDocument && (
        <DocumentViewer document={selectedDocument} onClose={() => setSelectedDocument(null)} />
      )}
    </div>
  );
}
