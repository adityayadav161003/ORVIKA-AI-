import type { ChatMessage, ChatSession } from "../types";

export function formatChatForExport(session: ChatSession, messages: ChatMessage[]): string {
  let content = `# ${session.name}\n`;
  content += `Created: ${new Date(session.createdAt).toLocaleString()}\n`;
  content += `Messages: ${messages.length}\n`;
  if (session.systemPrompt) {
    content += `System Prompt: ${session.systemPrompt}\n`;
  }
  content += `\n---\n\n`;

  for (const msg of messages) {
    const roleName = msg.role === "user" ? "You" : msg.role === "assistant" ? "Assistant" : "System";
    const timestamp = new Date(msg.createdAt).toLocaleString();
    
    content += `### ${roleName} (${timestamp})\n\n`;
    content += `${msg.content}\n\n`;
  }

  return content;
}

export function downloadStringAsFile(content: string, filename: string) {
  const blob = new Blob([content], { type: "text/markdown;charset=utf-8;" });
  const url = URL.createObjectURL(blob);
  
  const link = document.createElement("a");
  link.href = url;
  link.setAttribute("download", filename);
  document.body.appendChild(link);
  link.click();
  
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
