export type MessageRole = "user" | "assistant" | "system";

export interface ChatSession {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  researchModeEnabled: boolean;
  cloudProvider?: string | null;
  privacyLevel: string;
  modelId: string;
  isActive: boolean;
  metadata?: string | null;
  systemPrompt?: string | null;
  messageCount: number;
}

export interface ChatMessage {
  id: string;
  sessionId: string;
  role: MessageRole;
  content: string;
  sourceType?: "local" | "research" | "mixed" | null;
  sources?: string | null;
  createdAt: string;
  tokensUsed?: number | null;
  latencyMs?: number | null;
  metadata?: string | null;
}

export interface SendMessageResult {
  userMessage: ChatMessage;
  assistantMessage: ChatMessage;
}

export interface DownloadedModel {
  id: string;
  modelName: string;
  modelPath: string;
  fileSize: number;
  quantization: string;
  isActive: boolean;
}

export interface ResearchSession {
  id: string;
  sessionId: string;
  messageId: string;
  status: "planning" | "approved" | "in_progress" | "completed" | "failed";
  totalQueries: number;
  completedQueries: number;
  knowledgeGaps?: string | null;
  createdAt: string;
}

export interface ResearchQuery {
  id: string;
  researchSessionId: string;
  queryIndex: number;
  topic: string;
  rawQuery?: string | null;
  sanitizedQuery: string;
  riskLevel: "low" | "medium" | "high";
  status: "pending" | "approved" | "rejected" | "sent" | "completed" | "failed" | "blocked";
  userApproved?: boolean | null;
  response?: string | null;
  createdAt: string;
}

export interface ResearchPlanResult {
  session: ResearchSession;
  queries: ResearchQuery[];
}
