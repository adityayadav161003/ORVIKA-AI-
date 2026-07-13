# Sprint 4 - Chat Core

**Deliverable:** Basic chat: create/switch/delete sessions, send a message, receive a streamed local response, and persist both user and assistant messages.

## Source Analysis

| Source              | Sprint 4 signal                                                                                                                                                                                 |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `0development.html` | Defines S4-T1 through S4-T6: session CRUD, message persistence, SessionSidebar, MessageInput, Tauri token streaming, MessageBubble with markdown rendering.                                     |
| `0mimo.html`        | Defines chat IPC contracts for `create_session`, `send_message`, `get_messages`, `list_sessions`, and `delete_session`; also requires persistent independent sessions and streaming local chat. |

## Tasks

| ID    | Task                | Implementation                                                                                                 | Status   |
| ----- | ------------------- | -------------------------------------------------------------------------------------------------------------- | -------- |
| S4-T1 | Session CRUD        | `db/session_repo.rs`, `commands/chat.rs` expose create/list/get/delete                                         | Complete |
| S4-T2 | Message persistence | `db/message_repo.rs` saves ordered user/assistant messages in SQLite                                           | Complete |
| S4-T3 | SessionSidebar      | `src/features/chat/components/SessionSidebar.tsx` lists, creates, switches, deletes                            | Complete |
| S4-T4 | MessageInput        | `src/features/chat/components/MessageInput.tsx` sends on Enter and keeps newlines with Shift+Enter             | Complete |
| S4-T5 | Token streaming     | `send_message` streams tokens through Tauri `Channel<String>` and persists the final assistant reply           | Complete |
| S4-T6 | MessageBubble       | `src/features/chat/components/MessageBubble.tsx` renders streaming assistant bubbles with safe markdown blocks | Complete |

## IPC Commands

- `create_session(name, modelId)`
- `list_sessions()`
- `get_session(sessionId)`
- `delete_session(sessionId)`
- `get_messages(sessionId, limit, offset)`
- `send_message(sessionId, content, maxTokens, temperature, onToken)`

## Notes

- `send_message` uses the existing Sprint 2 llama.cpp runtime and OpenAI-compatible streaming endpoint.
- The first user prompt auto-renames a default "New chat" session to a short title.
- Context building is intentionally simple for Sprint 4: the latest 40 messages plus `models/prompts/chat.system.txt`. Sprint 5 owns richer context-window behavior.
