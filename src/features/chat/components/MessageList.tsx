import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { MessageBubble } from "./MessageBubble";
import type { ChatMessage } from "../types";

interface MessageListProps {
  messages: ChatMessage[];
  streaming: boolean;
}

export function MessageList({ messages, streaming }: MessageListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const isAtBottomRef = useRef(true);

  const virtualizer = useVirtualizer({
    count: messages.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 100, // Fallback estimate
    overscan: 5,
  });

  // Track if user has scrolled up
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const handleScroll = () => {
      // 10px threshold
      const isAtBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 10;
      isAtBottomRef.current = isAtBottom;
    };

    el.addEventListener("scroll", handleScroll);
    return () => el.removeEventListener("scroll", handleScroll);
  }, []);

  const lastMessageContent = messages[messages.length - 1]?.content;

  // Auto-scroll on new messages / streaming
  useEffect(() => {
    if (isAtBottomRef.current && virtualizer.scrollElement) {
      virtualizer.scrollToIndex(messages.length - 1, { align: "end" });
    }
  }, [messages.length, lastMessageContent, virtualizer, streaming]);

  return (
    <div ref={scrollRef} className="h-full overflow-y-auto px-4 py-5">
      <div
        className="relative mx-auto max-w-5xl"
        style={{
          height: `${virtualizer.getTotalSize()}px`,
        }}
      >
        <div
          className="absolute top-0 left-0 w-full"
          style={{
            transform: `translateY(${virtualizer.getVirtualItems()[0]?.start ?? 0}px)`,
          }}
        >
          {virtualizer.getVirtualItems().map((virtualItem) => {
            const message = messages[virtualItem.index];
            return (
              <div
                key={virtualItem.key}
                data-index={virtualItem.index}
                ref={virtualizer.measureElement}
                className="pb-4"
              >
                <MessageBubble
                  message={message}
                  streaming={streaming && message.id.startsWith("pending-assistant")}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
