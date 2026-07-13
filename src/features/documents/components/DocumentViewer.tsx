import { useState } from "react";
import { invoke, Channel } from "@tauri-apps/api/core";
import { cn } from "@/utils/cn";

interface DocumentChunk {
  id: string;
  documentId: string;
  chunkIndex: number;
  content: string;
  pageNumber?: number;
  sectionHeading?: string;
}

interface DocumentViewerProps {
  document: {
    id: string;
    filename: string;
    fileType: string;
    fileSize: number;
    chunkCount: number;
    metadata?: string | null;
  };
  onClose: () => void;
}

export function DocumentViewer({ document, onClose }: DocumentViewerProps) {
  const [activeTab, setActiveTab] = useState<"summary" | "search" | "ocr">("summary");

  // Parse metadata for OCR metrics
  const parsedMeta = document.metadata ? JSON.parse(document.metadata) : null;
  const isOcr = parsedMeta?.isOcr || false;
  const ocrConfidence = parsedMeta?.ocrConfidence;
  const ocrConfidencePerPage = parsedMeta?.ocrConfidencePerPage || [];

  // Summary State
  const [summary, setSummary] = useState<string>("");
  const [isGeneratingSummary, setIsGeneratingSummary] = useState(false);
  const [summaryError, setSummaryError] = useState("");

  // Search State
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<DocumentChunk[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [searchError, setSearchError] = useState("");

  const handleGenerateSummary = async () => {
    setIsGeneratingSummary(true);
    setSummary("");
    setSummaryError("");

    try {
      const channel = new Channel<string>();
      channel.onmessage = (token) => {
        setSummary((prev) => prev + token);
      };

      const finalSummary = await invoke<string>("summarize_document", {
        documentId: document.id,
        onToken: channel,
      });

      if (finalSummary) {
        setSummary(finalSummary);
      }
    } catch (err) {
      setSummaryError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsGeneratingSummary(false);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!searchQuery.trim()) return;

    setIsSearching(true);
    setSearchError("");
    setSearchResults([]);

    try {
      const results = await invoke<DocumentChunk[]>("search_document", {
        documentId: document.id,
        query: searchQuery.trim(),
      });
      setSearchResults(results);
    } catch (err) {
      setSearchError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsSearching(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 sm:p-6 lg:p-8 backdrop-blur-sm">
      <div className="flex h-full w-full max-w-5xl flex-col overflow-hidden rounded-xl bg-surface border border-border shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border bg-[#1a1a19] p-4">
          <div className="flex items-center gap-3">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              className="text-accent opacity-80"
            >
              <path
                d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                d="M14 2V8H20"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            <div>
              <h2 className="text-lg font-semibold text-text-primary leading-tight">
                {document.filename}
              </h2>
              <div className="flex items-center gap-2 mt-0.5">
                <p className="text-xs text-text-muted">
                  {(document.fileSize / 1024 / 1024).toFixed(2)} MB • {document.chunkCount} indexed
                  sections
                </p>
                {isOcr && typeof ocrConfidence === "number" && (
                  <span className="text-[10px] bg-amber-500/20 text-amber-400 px-2 py-0.5 rounded-full border border-amber-500/30 font-medium">
                    OCR ({Math.round(ocrConfidence)}% confidence)
                  </span>
                )}
              </div>
            </div>
          </div>
          <button
            onClick={onClose}
            className="rounded-full p-2 text-text-muted hover:bg-white/5 hover:text-white transition-colors"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none">
              <path
                d="M18 6L6 18M6 6L18 18"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </button>
        </div>

        <div className="flex h-full min-h-0">
          {/* Sidebar */}
          <div className="w-48 border-r border-border bg-[#151514] p-3 flex flex-col gap-1">
            <button
              onClick={() => setActiveTab("summary")}
              className={cn(
                "px-3 py-2 text-sm text-left rounded-md transition-colors font-medium",
                activeTab === "summary"
                  ? "bg-accent text-white"
                  : "text-text-secondary hover:text-white hover:bg-white/5",
              )}
            >
              AI Summary
            </button>
            <button
              onClick={() => setActiveTab("search")}
              className={cn(
                "px-3 py-2 text-sm text-left rounded-md transition-colors font-medium",
                activeTab === "search"
                  ? "bg-accent text-white"
                  : "text-text-secondary hover:text-white hover:bg-white/5",
              )}
            >
              Semantic Search
            </button>
            {isOcr && (
              <button
                onClick={() => setActiveTab("ocr")}
                className={cn(
                  "px-3 py-2 text-sm text-left rounded-md transition-colors font-medium",
                  activeTab === "ocr"
                    ? "bg-accent text-white"
                    : "text-text-secondary hover:text-white hover:bg-white/5",
                )}
              >
                OCR Details
              </button>
            )}
          </div>

          {/* Main Content Area */}
          <div className="flex-1 overflow-y-auto p-6 bg-surface">
            {activeTab === "summary" && (
              <div className="max-w-3xl flex flex-col h-full">
                <div className="flex items-center justify-between mb-6">
                  <h3 className="text-xl font-serif font-semibold text-text-primary">
                    Document Summary
                  </h3>
                  {!summary && !isGeneratingSummary && (
                    <button
                      onClick={handleGenerateSummary}
                      className="px-4 py-2 bg-accent text-white text-sm rounded-md font-medium hover:bg-accent/90 transition-colors shadow-sm flex items-center gap-2"
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
                        <path
                          d="M13 2L3 14H12L11 22L21 10H12L13 2Z"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      </svg>
                      Generate Summary
                    </button>
                  )}
                </div>

                {summaryError && (
                  <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 text-red-400 text-sm rounded-md">
                    {summaryError}
                  </div>
                )}

                {!summary && !isGeneratingSummary && !summaryError && (
                  <div className="flex-1 flex flex-col items-center justify-center text-center opacity-60">
                    <svg width="48" height="48" viewBox="0 0 24 24" fill="none" className="mb-4">
                      <path
                        d="M21 15V19C21 19.5304 20.7893 20.0391 20.4142 20.4142C20.0391 20.7893 19.5304 21 19 21H5C4.46957 21 3.96086 20.7893 3.58579 20.4142C3.21071 20.0391 3 19.5304 3 19V15M7 10L12 15M12 15L17 10M12 15V3"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                    <p className="text-lg font-medium text-text-primary">
                      No summary generated yet
                    </p>
                    <p className="text-sm text-text-muted max-w-sm mt-1">
                      Click the button above to generate a concise AI summary of this document using
                      your local LLM.
                    </p>
                  </div>
                )}

                {(summary || isGeneratingSummary) && (
                  <div className="prose prose-sm prose-invert max-w-none text-text-primary p-5 bg-[#151514] rounded-lg border border-white/5 relative">
                    <p className="whitespace-pre-wrap leading-relaxed">{summary}</p>
                    {isGeneratingSummary && (
                      <span className="ml-1 inline-block h-4 w-0.5 animate-pulse align-text-bottom bg-accent" />
                    )}
                  </div>
                )}
              </div>
            )}

            {activeTab === "search" && (
              <div className="max-w-3xl flex flex-col h-full">
                <h3 className="text-xl font-serif font-semibold text-text-primary mb-4">
                  Semantic Search
                </h3>

                <form onSubmit={handleSearch} className="flex gap-2 mb-6">
                  <div className="relative flex-1">
                    <div className="absolute inset-y-0 left-3 flex items-center pointer-events-none opacity-50">
                      <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                        <path
                          d="M21 21L15 15M17 10C17 13.866 13.866 17 10 17C6.13401 17 3 13.866 3 10C3 6.13401 6.13401 3 10 3C13.866 3 17 6.13401 17 10Z"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      </svg>
                    </div>
                    <input
                      type="text"
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      placeholder="Search for concepts, facts, or keywords in this document..."
                      className="w-full pl-10 pr-4 py-2.5 bg-[#151514] border border-border rounded-lg text-sm text-white focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent transition-all placeholder:text-text-muted"
                    />
                  </div>
                  <button
                    type="submit"
                    disabled={isSearching || !searchQuery.trim()}
                    className="px-5 py-2.5 bg-accent text-white font-medium text-sm rounded-lg hover:bg-accent/90 disabled:opacity-50 disabled:cursor-not-allowed transition-all shadow-sm"
                  >
                    {isSearching ? "Searching..." : "Search"}
                  </button>
                </form>

                {searchError && (
                  <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 text-red-400 text-sm rounded-md">
                    {searchError}
                  </div>
                )}

                <div className="flex-1 overflow-y-auto space-y-4 pb-4">
                  {searchResults.length === 0 && !isSearching && searchQuery && !searchError && (
                    <div className="text-center py-10 text-text-muted italic">
                      No matching chunks found.
                    </div>
                  )}

                  {searchResults.map((chunk, idx) => (
                    <div
                      key={chunk.id}
                      className="p-4 bg-[#151514] border border-white/5 rounded-lg hover:border-white/10 transition-colors"
                    >
                      <div className="flex items-center justify-between mb-2 pb-2 border-b border-white/5 text-[11px] font-mono text-accent">
                        <span>Match #{idx + 1}</span>
                        {chunk.pageNumber && <span>Page {chunk.pageNumber}</span>}
                      </div>
                      <p className="text-sm text-text-primary leading-relaxed whitespace-pre-wrap">
                        {chunk.content}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeTab === "ocr" && (
              <div className="max-w-3xl flex flex-col h-full space-y-6">
                <div>
                  <h3 className="text-xl font-serif font-semibold text-text-primary">
                    Optical Character Recognition (OCR) Details
                  </h3>
                  <p className="text-sm text-text-muted mt-1">
                    This document was processed using Tesseract OCR fallback because the original
                    PDF contains images or scanned text without embedded vector fonts.
                  </p>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                  <div className="p-4 bg-[#151514] border border-white/5 rounded-lg flex flex-col justify-center">
                    <span className="text-[11px] font-mono uppercase tracking-wider text-text-muted">
                      Average OCR Confidence
                    </span>
                    <span className="text-3xl font-bold text-accent mt-1">
                      {typeof ocrConfidence === "number" ? `${Math.round(ocrConfidence)}%` : "N/A"}
                    </span>
                  </div>
                  <div className="p-4 bg-[#151514] border border-white/5 rounded-lg flex flex-col justify-center">
                    <span className="text-[11px] font-mono uppercase tracking-wider text-text-muted">
                      Total Pages Scanned
                    </span>
                    <span className="text-3xl font-bold text-text-primary mt-1">
                      {ocrConfidencePerPage.length}
                    </span>
                  </div>
                </div>

                <div className="space-y-3">
                  <h4 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                    Page-by-Page Confidence Matrix
                  </h4>
                  <div className="border border-border rounded-lg overflow-hidden bg-[#151514]">
                    <div className="grid grid-cols-3 bg-[#1d1d1c] p-3 text-xs font-semibold text-text-secondary border-b border-border">
                      <div>Page Number</div>
                      <div>Confidence Score</div>
                      <div>Status / Quality</div>
                    </div>
                    {ocrConfidencePerPage.length === 0 ? (
                      <div className="p-4 text-center text-text-muted italic">
                        No page-by-page confidence data available for this document.
                      </div>
                    ) : (
                      <div className="max-h-[280px] overflow-y-auto divide-y divide-white/5">
                        {ocrConfidencePerPage.map((conf: number, idx: number) => {
                          const pageNum = idx + 1;
                          const isLow = conf < 70;
                          const isExcellent = conf >= 90;

                          return (
                            <div
                              key={idx}
                              className="grid grid-cols-3 p-3 text-xs items-center text-text-secondary animate-fade-in"
                            >
                              <div className="font-medium text-text-primary">Page {pageNum}</div>
                              <div className="font-mono">{Math.round(conf)}%</div>
                              <div>
                                {isLow ? (
                                  <span className="inline-flex items-center gap-1 text-[10px] bg-red-500/10 text-red-400 px-2 py-0.5 rounded-full border border-red-500/20 font-medium">
                                    ⚠ Low Quality (Review Required)
                                  </span>
                                ) : isExcellent ? (
                                  <span className="inline-flex items-center gap-1 text-[10px] bg-green-500/10 text-green-400 px-2 py-0.5 rounded-full border border-green-500/20 font-medium">
                                    ✓ Excellent
                                  </span>
                                ) : (
                                  <span className="inline-flex items-center gap-1 text-[10px] bg-blue-500/10 text-blue-400 px-2 py-0.5 rounded-full border border-blue-500/20 font-medium">
                                    Good
                                  </span>
                                )}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
