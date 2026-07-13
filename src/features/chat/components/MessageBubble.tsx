import { useState } from "react";
import { cn } from "@/utils/cn";
import type { ChatMessage } from "../types";

interface MessageBubbleProps {
  message: ChatMessage;
  streaming?: boolean;
}

import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { vscDarkPlus } from "react-syntax-highlighter/dist/esm/styles/prism";

function MarkdownContent({ content }: { content: string }) {
  if (!content) return null;

  return (
    <div className="prose prose-sm max-w-none text-text-primary prose-p:leading-relaxed prose-pre:bg-text-primary prose-pre:text-white prose-pre:p-0 prose-pre:m-0">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          code(props) {
            // eslint-disable-next-line @typescript-eslint/no-unused-vars
            const { children, className, node, ref, ...rest } = props;
            const match = /language-(\w+)/.exec(className || "");
            return match ? (
              <div className="overflow-hidden rounded-md border border-border my-3">
                <div className="bg-[#1e1e1c] px-3 py-1.5 text-[10px] uppercase text-white/50 border-b border-white/10 font-mono flex justify-between items-center">
                  <span>{match[1]}</span>
                </div>
                <SyntaxHighlighter
                  {...rest}
                  PreTag="div"
                  children={String(children).replace(/\n$/, "")}
                  language={match[1]}
                  style={vscDarkPlus}
                  customStyle={{
                    margin: 0,
                    padding: "0.75rem",
                    background: "#1e1e1c",
                    fontSize: "0.75rem",
                  }}
                />
              </div>
            ) : (
              <code
                {...rest}
                className={cn("rounded bg-surface px-1.5 py-0.5 font-mono text-[0.9em]", className)}
              >
                {children}
              </code>
            );
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}

function formatTime(value: string) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "";
  return parsed.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

function parseThinkBlock(content: string): { think: string | null; response: string } {
  const thinkStart = content.indexOf("<think>");
  if (thinkStart === -1) {
    return { think: null, response: content };
  }

  const thinkEnd = content.indexOf("</think>");
  if (thinkEnd === -1) {
    return {
      think: content.substring(thinkStart + 7).trim(),
      response: content.substring(0, thinkStart).trim(),
    };
  }

  return {
    think: content.substring(thinkStart + 7, thinkEnd).trim(),
    response: (content.substring(0, thinkStart) + content.substring(thinkEnd + 8)).trim(),
  };
}

function ThinkCollapsible({ content, streaming }: { content: string; streaming: boolean }) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div className="mb-4 overflow-hidden rounded-md border border-border/60 bg-surface/30">
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex w-full items-center gap-2 px-3 py-2.5 text-xs font-semibold text-text-muted hover:bg-surface/50 hover:text-text-primary transition-colors"
      >
        <svg
          className={cn("h-3.5 w-3.5 transition-transform", isOpen ? "rotate-90" : "")}
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
          strokeWidth={2.5}
        >
          <path strokeLinecap="round" strokeLinejoin="round" d="M9 5l7 7-7 7" />
        </svg>
        {streaming ? "Thinking..." : "Thought Process"}
      </button>
      {isOpen && (
        <div className="border-t border-border/60 px-4 py-3 text-sm text-text-muted">
          <MarkdownContent content={content} />
        </div>
      )}
    </div>
  );
}

export function MessageBubble({ message, streaming = false }: MessageBubbleProps) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";

  if (isSystem) {
    return (
      <div className="mx-auto max-w-xl rounded-md border border-border bg-surface px-3 py-2 text-center text-xs text-text-muted">
        {message.content}
      </div>
    );
  }

  return (
    <article className={cn("flex", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "max-w-[min(760px,86%)] rounded-lg border px-4 py-3 text-sm leading-relaxed shadow-sm",
          isUser
            ? "border-accent bg-accent text-white"
            : "border-border bg-white text-text-primary",
        )}
      >
        <div
          className={cn(
            "mb-2 flex items-center justify-between gap-3 font-mono text-[11px]",
            isUser ? "text-white/70" : "text-text-muted",
          )}
        >
          <span>{isUser ? "You" : "Assistant"}</span>
          <span>{formatTime(message.createdAt)}</span>
        </div>
        {message.content ? (
          isUser ? (
            <p className="whitespace-pre-wrap">{message.content}</p>
          ) : (
            (() => {
              const { think, response } = parseThinkBlock(message.content);
              const isThinkingStreaming = streaming && !message.content.includes("</think>");
              return (
                <>
                  {think !== null && (
                    <ThinkCollapsible content={think} streaming={isThinkingStreaming} />
                  )}
                  {response ? (
                    <MarkdownContent content={response} />
                  ) : isThinkingStreaming ? null : (
                    <MarkdownContent content={message.content} />
                  )}
                </>
              );
            })()
          )
        ) : (
          <span className={cn("font-mono", isUser ? "text-white/70" : "text-text-muted")}>
            Thinking
          </span>
        )}
        {streaming && (
          <span
            className={cn(
              "ml-1 inline-block h-4 w-0.5 animate-pulse align-text-bottom",
              isUser ? "bg-white" : "bg-accent",
            )}
          />
        )}
        {!isUser && message.sources && (
          <div className="mt-3 flex flex-wrap gap-2 border-t border-border/50 pt-3">
            {(() => {
              try {
                const parsed = JSON.parse(message.sources);
                if (Array.isArray(parsed) && parsed.length > 0) {
                  return parsed.map((source: string, idx: number) => (
                    <div
                      key={idx}
                      className="inline-flex items-center gap-1.5 rounded-md bg-surface px-2 py-1 text-[10px] font-medium text-text-muted border border-border/60"
                      title={source}
                    >
                      <svg
                        width="12"
                        height="12"
                        viewBox="0 0 24 24"
                        fill="none"
                        xmlns="http://www.w3.org/2000/svg"
                        className="opacity-70"
                      >
                        <path
                          d="M13 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V9L13 2Z"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                        <path
                          d="M13 2V9H20"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      </svg>
                      <span className="max-w-[120px] truncate">{source}</span>
                    </div>
                  ));
                }
              } catch {
                // Not a valid JSON array or couldn't parse
              }
              return null;
            })()}
          </div>
        )}
      </div>
    </article>
  );
}
