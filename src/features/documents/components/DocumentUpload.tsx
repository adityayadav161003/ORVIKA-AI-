import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/Button";

interface DocumentUploadProps {
  onUploadComplete?: () => void;
}

export function DocumentUpload({ onUploadComplete }: DocumentUploadProps) {
  const [isUploading, setIsUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'Documents',
          extensions: ['pdf', 'docx', 'pptx', 'txt', 'md']
        }]
      });

      if (!selected) return;

      const filePath = Array.isArray(selected) ? selected[0] : selected;
      
      setIsUploading(true);
      setError(null);
      
      await invoke("upload_document", {
        filePath: filePath,
        sessionId: null,
      });

      onUploadComplete?.();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsUploading(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center p-6 border-2 border-dashed border-border rounded-lg bg-surface/50">
      <div className="mb-4 text-3xl">📁</div>
      <h3 className="mb-2 font-medium text-text-primary">Upload Document</h3>
      <p className="mb-4 text-sm text-text-secondary text-center max-w-sm">
        Select a PDF, DOCX, PPTX, TXT, or MD file to parse and analyze locally.
      </p>
      
      {error && (
        <div className="mb-4 text-sm text-red-500 bg-red-500/10 px-3 py-2 rounded-md">
          {error}
        </div>
      )}

      <Button 
        onClick={handleSelectFile} 
        disabled={isUploading}
        className="w-48"
      >
        {isUploading ? "Parsing..." : "Select File"}
      </Button>
    </div>
  );
}
