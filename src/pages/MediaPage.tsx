import { useEffect, useState, useRef } from "react";
import { invoke, Channel, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button, Select, Input, Spinner } from "@/components/ui";

interface Document {
  id: string;
  filename: string;
  fileSize: number;
  fileType: string;
  chunkCount: number;
  createdAt: string;
  parsedAt: string | null;
  filePath: string;
  metadata: string | null;
}

interface FrontEndSegment {
  start: number;
  end: number;
  text: string;
}

export function MediaPage() {
  const [mediaList, setMediaList] = useState<Document[]>([]);
  const [activeMedia, setActiveMedia] = useState<Document | null>(null);
  const [transcript, setTranscript] = useState<FrontEndSegment[]>([]);
  const [currentTime, setCurrentTime] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [isUploading, setIsUploading] = useState(false);
  const [isLoadingList, setIsLoadingList] = useState(true);
  const [isLoadingTranscript, setIsLoadingTranscript] = useState(false);
  const [summaryDetail, setSummaryDetail] = useState("Balanced");
  const [summaryText, setSummaryText] = useState("");
  const [isSummarizing, setIsSummarizing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mediaRef = useRef<HTMLVideoElement | HTMLAudioElement | null>(null);

  // Helper to format time (e.g. 75.5 -> "01:15")
  const formatTime = (time: number) => {
    if (isNaN(time)) return "00:00";
    const hrs = Math.floor(time / 3600);
    const mins = Math.floor((time % 3600) / 60);
    const secs = Math.floor(time % 60);

    const pad = (num: number) => String(num).padStart(2, "0");
    if (hrs > 0) {
      return `${pad(hrs)}:${pad(mins)}:${pad(secs)}`;
    }
    return `${pad(mins)}:${pad(secs)}`;
  };

  const loadMediaList = async () => {
    try {
      setIsLoadingList(true);
      const docs = await invoke<Document[]>("list_documents", { sessionId: null });

      // Filter for media documents based on extension or parsed metadata
      const filtered = docs.filter((doc) => {
        const ext = doc.fileType.toLowerCase();
        const isMediaExt = [
          "mp4",
          "mkv",
          "avi",
          "webm",
          "mp3",
          "wav",
          "m4a",
          "flac",
          "ogg",
        ].includes(ext);

        let isMediaMeta = false;
        if (doc.metadata) {
          try {
            const meta = JSON.parse(doc.metadata);
            if (meta.isMedia) isMediaMeta = true;
          } catch {
            // ignore
          }
        }
        return isMediaExt || isMediaMeta;
      });
      setMediaList(filtered);
    } catch (err) {
      console.error("Failed to load media list", err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsLoadingList(false);
    }
  };

  useEffect(() => {
    loadMediaList();
  }, []);

  // Poll list if there are transcribing items
  useEffect(() => {
    const hasTranscribing = mediaList.some((doc) => !doc.parsedAt);
    if (!hasTranscribing) return;

    const interval = setInterval(() => {
      loadMediaList();
    }, 4000);

    return () => clearInterval(interval);
  }, [mediaList]);

  // Load transcript when active media changes
  useEffect(() => {
    if (!activeMedia) {
      setTranscript([]);
      setSummaryText("");
      return;
    }

    const loadTranscript = async () => {
      try {
        setIsLoadingTranscript(true);
        setError(null);
        setSummaryText("");
        const segments = await invoke<FrontEndSegment[]>("get_media_transcript", {
          documentId: activeMedia.id,
        });

        // Sort segments chronologically
        segments.sort((a, b) => a.start - b.start);
        setTranscript(segments);
      } catch (err) {
        console.error("Failed to load transcript", err);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setIsLoadingTranscript(false);
      }
    };

    if (activeMedia.parsedAt) {
      loadTranscript();
    }
  }, [activeMedia]);

  const handleSelectFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Media Files",
            extensions: ["mp4", "mkv", "avi", "webm", "mp3", "wav", "m4a", "flac", "ogg"],
          },
        ],
      });

      if (!selected) return;

      const filePath = Array.isArray(selected) ? selected[0] : selected;

      setIsUploading(true);
      setError(null);

      const doc = await invoke<Document>("upload_document", {
        filePath: filePath,
        sessionId: null,
      });

      setActiveMedia(doc);
      await loadMediaList();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsUploading(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Are you sure you want to delete this media file?")) return;
    try {
      await invoke("delete_document", { documentId: id });
      if (activeMedia?.id === id) {
        setActiveMedia(null);
      }
      await loadMediaList();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleTimeUpdate = () => {
    if (mediaRef.current) {
      setCurrentTime(mediaRef.current.currentTime);
    }
  };

  const handleSegmentClick = (start: number) => {
    if (mediaRef.current) {
      mediaRef.current.currentTime = start;
      mediaRef.current.play().catch(() => {});
    }
  };

  // Convert local file path to Tauri asset source
  const getMediaSource = (filePath: string) => {
    // Tauri v2 convertFileSrc
    // It takes the raw path and returns an asset:// or https://asset.localhost protocol
    try {
      return convertFileSrc(filePath);
    } catch {
      // Fallback if imported/required differently
      return `https://asset.localhost/${filePath.replace(/\\/g, "/")}`;
    }
  };

  const isVideo = (type: string) => {
    return ["mp4", "mkv", "avi", "webm"].includes(type.toLowerCase());
  };

  const handleGenerateSummary = async () => {
    if (!activeMedia) return;
    try {
      setIsSummarizing(true);
      setSummaryText("");

      const channel = new Channel<string>();
      channel.onmessage = (token) => {
        setSummaryText((prev) => prev + token);
      };

      await invoke("generate_meeting_summary", {
        documentId: activeMedia.id,
        detailLevel: summaryDetail,
        onToken: channel,
      });
    } catch (err) {
      setError("Failed to generate summary: " + (err instanceof Error ? err.message : String(err)));
    } finally {
      setIsSummarizing(false);
    }
  };

  const handleExportText = () => {
    if (transcript.length === 0) return;
    const textContent = transcript
      .map((s) => `[${formatTime(s.start)} - ${formatTime(s.end)}] ${s.text}`)
      .join("\n");

    downloadFile(textContent, `${activeMedia?.filename || "transcript"}.txt`, "text/plain");
  };

  const handleExportSRT = () => {
    if (transcript.length === 0) return;

    const formatSrtTime = (seconds: number) => {
      const pad = (num: number, len = 2) => String(num).padStart(len, "0");
      const hrs = Math.floor(seconds / 3600);
      const mins = Math.floor((seconds % 3600) / 60);
      const secs = Math.floor(seconds % 60);
      const ms = Math.floor((seconds % 1) * 1000);

      return `${pad(hrs)}:${pad(mins)}:${pad(secs)},${pad(ms, 3)}`;
    };

    const srtContent = transcript
      .map((s, i) => {
        return `${i + 1}\n${formatSrtTime(s.start)} --> ${formatSrtTime(s.end)}\n${s.text}\n`;
      })
      .join("\n");

    downloadFile(srtContent, `${activeMedia?.filename || "transcript"}.srt`, "text/plain");
  };

  const downloadFile = (content: string, fileName: string, contentType: string) => {
    const a = document.createElement("a");
    const file = new Blob([content], { type: contentType });
    a.href = URL.createObjectURL(file);
    a.download = fileName;
    a.click();
    URL.revokeObjectURL(a.href);
  };

  const filteredTranscript = transcript.filter((s) =>
    s.text.toLowerCase().includes(searchQuery.toLowerCase()),
  );

  // Active segment detection
  const activeSegmentIndex = transcript.findIndex(
    (s) => currentTime >= s.start && currentTime <= s.end,
  );

  return (
    <div className="flex flex-col h-full overflow-y-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex flex-col gap-2">
        <h2 className="font-serif text-3xl font-bold text-text-primary">
          Media Intelligence Center
        </h2>
        <p className="text-text-secondary max-w-2xl">
          Process, transcribe, and query your local video and audio recordings locally. Outbound
          transcription queries never touch the internet.
        </p>
      </div>

      {error && (
        <div className="text-sm text-red-500 bg-red-500/10 px-4 py-3 rounded-md border border-red-500/20 relative">
          <span className="font-semibold">Error: </span> {error}
          <button
            onClick={() => setError(null)}
            className="absolute top-2 right-3 text-lg font-bold hover:text-red-700"
          >
            ×
          </button>
        </div>
      )}

      {/* Main Grid */}
      <div className="grid grid-cols-1 xl:grid-cols-4 gap-6 items-start">
        {/* Left Side: Upload & Media List */}
        <div className="xl:col-span-1 space-y-6">
          {/* Upload Widget */}
          <div className="p-5 border-2 border-dashed border-border rounded-lg bg-surface/50 text-center flex flex-col items-center">
            <span className="text-3xl mb-2">🎬</span>
            <h4 className="font-semibold text-text-primary mb-1">Process Local Media</h4>
            <p className="text-xs text-text-secondary mb-4 leading-relaxed">
              Supports MP4, MKV, AVI, WebM, MP3, WAV, M4A, FLAC, OGG.
            </p>
            <Button onClick={handleSelectFile} loading={isUploading} className="w-full">
              {isUploading ? "Uploading..." : "Select Media File"}
            </Button>
          </div>

          {/* Media List */}
          <div className="space-y-3">
            <h3 className="font-medium text-text-primary border-b border-border pb-1.5 text-sm uppercase tracking-wider">
              Local Media Files
            </h3>

            {isLoadingList ? (
              <div className="text-xs text-text-muted flex items-center gap-2">
                <Spinner /> Loading files...
              </div>
            ) : mediaList.length === 0 ? (
              <div className="text-xs text-text-muted italic py-4">No media uploaded yet.</div>
            ) : (
              <div className="space-y-2 max-h-[300px] overflow-y-auto pr-1">
                {mediaList.map((media) => {
                  const isActive = activeMedia?.id === media.id;
                  const isProcessing = !media.parsedAt;

                  return (
                    <div
                      key={media.id}
                      onClick={() => !isProcessing && setActiveMedia(media)}
                      className={`flex flex-col p-3 border rounded-lg transition-all relative group cursor-pointer ${
                        isActive
                          ? "border-accent bg-accent-light text-accent"
                          : "border-border bg-white hover:border-accent/40"
                      } ${isProcessing ? "opacity-75 cursor-not-allowed" : ""}`}
                    >
                      <div className="flex items-center justify-between mb-1.5 pr-6">
                        <h4
                          className="font-medium text-xs truncate max-w-[85%] font-mono"
                          title={media.filename}
                        >
                          {media.filename}
                        </h4>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDelete(media.id);
                          }}
                          className="text-text-muted hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity absolute right-2 top-2 p-1 text-base leading-none"
                          title="Delete File"
                        >
                          ×
                        </button>
                      </div>

                      <div className="flex justify-between items-center text-[10px] text-text-muted font-mono">
                        <span>
                          {media.fileType.toUpperCase()} •{" "}
                          {(media.fileSize / 1024 / 1024).toFixed(1)} MB
                        </span>
                        {isProcessing ? (
                          <span className="text-accent animate-pulse">Transcribing...</span>
                        ) : (
                          <span>Processed</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Right Side: Player & Transcript */}
        <div className="xl:col-span-3 grid grid-cols-1 lg:grid-cols-12 gap-6">
          {activeMedia ? (
            <>
              {/* Media Player Column */}
              <div className="lg:col-span-7 space-y-6">
                <div className="bg-white border border-border rounded-lg overflow-hidden shadow-sm">
                  {/* Player header */}
                  <div className="px-4 py-3 border-b border-border bg-surface flex items-center justify-between">
                    <span className="font-mono text-xs text-text-secondary truncate max-w-[80%] font-semibold">
                      {activeMedia.filename}
                    </span>
                    <span className="text-[10px] bg-accent text-white px-1.5 py-0.5 rounded font-mono uppercase font-bold">
                      {activeMedia.fileType}
                    </span>
                  </div>

                  {/* Video/Audio Element */}
                  <div className="bg-black aspect-video flex items-center justify-center relative">
                    {isVideo(activeMedia.fileType) ? (
                      <video
                        ref={mediaRef as React.RefObject<HTMLVideoElement>}
                        src={getMediaSource(activeMedia.filePath)}
                        controls
                        onTimeUpdate={handleTimeUpdate}
                        className="w-full h-full max-h-[400px] object-contain"
                      />
                    ) : (
                      <div className="flex flex-col items-center justify-center p-8 w-full h-full">
                        <span className="text-5xl mb-4">🎵</span>
                        <audio
                          ref={mediaRef as React.RefObject<HTMLAudioElement>}
                          src={getMediaSource(activeMedia.filePath)}
                          controls
                          onTimeUpdate={handleTimeUpdate}
                          className="w-4/5"
                        />
                        <span className="text-xs text-text-muted mt-3 font-mono">
                          Audio Playing • {formatTime(currentTime)}
                        </span>
                      </div>
                    )}
                  </div>
                </div>

                {/* Meeting Summary Widget */}
                {activeMedia.parsedAt && (
                  <div className="bg-white border border-border rounded-lg p-5 shadow-sm space-y-4">
                    <div className="flex items-center justify-between border-b border-border pb-3">
                      <h3 className="font-serif text-lg font-bold text-text-primary">
                        Meeting Summarizer
                      </h3>
                      <div className="flex items-center gap-2">
                        <Select
                          value={summaryDetail}
                          onChange={(e) => setSummaryDetail(e.target.value)}
                          options={[
                            { value: "Concise", label: "Concise Summary" },
                            { value: "Balanced", label: "Balanced Summary" },
                            { value: "Detailed", label: "Detailed Notes" },
                          ]}
                          className="text-xs py-1"
                        />
                        <Button onClick={handleGenerateSummary} loading={isSummarizing} size="sm">
                          Generate
                        </Button>
                      </div>
                    </div>

                    {isSummarizing && !summaryText && (
                      <div className="flex items-center gap-2 text-xs text-text-muted justify-center py-6">
                        <Spinner /> Consulting local LLM for summary notes...
                      </div>
                    )}

                    {summaryText && (
                      <div className="bg-surface/50 border border-border/60 rounded-md p-4 max-h-[300px] overflow-y-auto font-body text-sm leading-relaxed text-text-secondary whitespace-pre-line prose max-w-none">
                        {summaryText}
                      </div>
                    )}
                  </div>
                )}
              </div>

              {/* Transcript & Sync Column */}
              <div className="lg:col-span-5 space-y-4">
                <div className="bg-white border border-border rounded-lg p-4 shadow-sm flex flex-col h-[520px]">
                  {/* Sync Header */}
                  <div className="flex flex-col gap-3 pb-3 border-b border-border mb-3">
                    <div className="flex items-center justify-between">
                      <h3 className="font-serif text-base font-bold text-text-primary">
                        Transcript
                      </h3>
                      <div className="flex gap-1.5">
                        <Button
                          onClick={handleExportText}
                          disabled={transcript.length === 0}
                          variant="secondary"
                          size="sm"
                          className="text-xs px-2.5 py-1"
                          title="Export plain text (.txt)"
                        >
                          TXT
                        </Button>
                        <Button
                          onClick={handleExportSRT}
                          disabled={transcript.length === 0}
                          variant="secondary"
                          size="sm"
                          className="text-xs px-2.5 py-1"
                          title="Export subtitles (.srt)"
                        >
                          SRT
                        </Button>
                      </div>
                    </div>

                    <Input
                      placeholder="Search transcript..."
                      value={searchQuery}
                      onChange={(e) => setSearchQuery(e.target.value)}
                      className="text-xs py-1"
                    />
                  </div>

                  {/* Transcript segments scrollbox */}
                  <div className="flex-1 overflow-y-auto space-y-2 pr-1 select-text">
                    {isLoadingTranscript ? (
                      <div className="text-xs text-text-muted flex items-center justify-center gap-2 h-full">
                        <Spinner /> Loading transcript segments...
                      </div>
                    ) : filteredTranscript.length === 0 ? (
                      <div className="text-xs text-text-muted italic text-center py-8">
                        {searchQuery
                          ? "No matches found for search query."
                          : "No transcript segments found."}
                      </div>
                    ) : (
                      filteredTranscript.map((segment, index) => {
                        const originalIndex = transcript.indexOf(segment);
                        const isActive = originalIndex === activeSegmentIndex;

                        return (
                          <div
                            key={index}
                            onClick={() => handleSegmentClick(segment.start)}
                            className={`p-2.5 rounded border transition-all cursor-pointer text-xs ${
                              isActive
                                ? "bg-accent-light border-accent text-accent font-medium shadow-sm scale-[1.01]"
                                : "bg-surface/30 border-transparent hover:bg-surface text-text-secondary"
                            }`}
                          >
                            <div className="flex items-center gap-2 mb-1">
                              <span className="font-mono text-[10px] bg-border/40 text-text-muted px-1.5 py-0.5 rounded leading-none">
                                {formatTime(segment.start)}
                              </span>
                            </div>
                            <p className="leading-relaxed">{segment.text}</p>
                          </div>
                        );
                      })
                    )}
                  </div>
                </div>
              </div>
            </>
          ) : (
            <div className="lg:col-span-12 border border-dashed border-border rounded-lg bg-surface/20 flex flex-col items-center justify-center py-20 text-center text-text-muted">
              <span className="text-5xl mb-4 animate-bounce">🎬</span>
              <p className="font-serif text-lg font-semibold text-text-primary">
                No Media File Loaded
              </p>
              <p className="text-sm text-text-secondary max-w-sm mt-1">
                Select an processed file from the library sidebar or upload a new recording to begin
                analysis.
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
