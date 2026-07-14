import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ResearchPlanResult } from "../types";
import { Button } from "@/components/ui/Button";

interface ResearchPlanModalProps {
  plan: ResearchPlanResult;
  onApprove: () => void;
  onCancel: () => void;
}

export function ResearchPlanModal({ plan, onApprove, onCancel }: ResearchPlanModalProps) {
  const [selectedQueryIds, setSelectedQueryIds] = useState<Set<string>>(
    new Set(plan.queries.map((q) => q.id))
  );
  const [isApproving, setIsApproving] = useState(false);
  const [error, setError] = useState<string>("");

  const toggleQuery = (id: string) => {
    const newSelected = new Set(selectedQueryIds);
    if (newSelected.has(id)) {
      newSelected.delete(id);
    } else {
      newSelected.add(id);
    }
    setSelectedQueryIds(newSelected);
  };

  const handleApprove = async () => {
    if (selectedQueryIds.size === 0) {
      setError("Please select at least one query to research.");
      return;
    }
    setIsApproving(true);
    setError("");
    try {
      await invoke("approve_research_plan", {
        researchSessionId: plan.session.id,
        approvedQueryIds: Array.from(selectedQueryIds),
      });
      onApprove();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setIsApproving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
      <div className="w-full max-w-2xl bg-[#1a1a19] rounded-xl border border-border shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        
        {/* Header */}
        <div className="p-5 border-b border-border bg-[#151514]">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-accent/20 text-accent rounded-lg">
              <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
                <path d="M21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                <path d="M12 8V12L15 15" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
            </div>
            <div>
              <h2 className="text-xl font-serif font-bold text-white">Research Plan Generated</h2>
              <p className="text-sm text-text-muted mt-0.5">Review the knowledge gaps and select queries for deep research.</p>
            </div>
          </div>
        </div>

        {/* Risk Banner */}
        {plan.queries.some(q => q.riskLevel === "high" || q.riskLevel === "medium") && (
          <div className="bg-red-500/10 border-y border-red-500/20 p-3 px-6 flex items-start gap-3">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" className="text-red-400 mt-0.5 shrink-0">
              <path d="M12 9V14M12 17.5V18M12 3L21 21H3L12 3Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
            <div>
              <h4 className="text-sm font-semibold text-red-200">Sensitive Information Redacted</h4>
              <p className="text-xs text-red-200/80 mt-1 leading-relaxed">
                We detected Personally Identifiable Information (PII) in the generated queries. 
                They have been automatically sanitized below. Please verify the <strong>Sanitized Query</strong> before approving.
              </p>
            </div>
          </div>
        )}

        {/* Content */}
        <div className="p-6 overflow-y-auto flex-1 space-y-6">
          
          {/* Knowledge Gaps */}
          {plan.session.knowledgeGaps && (
            <div className="space-y-3">
              <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider flex items-center gap-2">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" className="text-orange-400">
                  <path d="M12 9V14M12 17.5V18M21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
                </svg>
                Identified Knowledge Gaps
              </h3>
              <div className="p-4 bg-orange-500/10 border border-orange-500/20 rounded-lg text-sm text-orange-200 leading-relaxed">
                {plan.session.knowledgeGaps}
              </div>
            </div>
          )}

          {/* Queries List */}
          <div className="space-y-3">
            <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider flex items-center gap-2">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" className="text-blue-400">
                <path d="M21 21L15 15M17 10C17 13.866 13.866 17 10 17C6.13401 17 3 13.866 3 10C3 6.13401 6.13401 3 10 3C13.866 3 17 6.13401 17 10Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
              Proposed Web Queries
            </h3>
            
            <div className="space-y-2">
              {plan.queries.map((q) => (
                <label 
                  key={q.id} 
                  className={`flex items-start gap-3 p-4 border rounded-lg cursor-pointer transition-colors ${
                    selectedQueryIds.has(q.id) 
                      ? 'bg-blue-500/10 border-blue-500/30' 
                      : 'bg-surface border-border hover:border-white/20'
                  }`}
                >
                  <div className="pt-0.5">
                    <input 
                      type="checkbox" 
                      className="w-4 h-4 rounded border-border bg-[#151514] text-accent focus:ring-accent focus:ring-offset-0"
                      checked={selectedQueryIds.has(q.id)}
                      onChange={() => toggleQuery(q.id)}
                    />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between gap-3">
                      <div className="font-medium text-white text-sm">{q.topic}</div>
                      {q.riskLevel === "high" && (
                        <span className="shrink-0 px-2 py-0.5 rounded text-[10px] font-bold tracking-wider uppercase bg-red-500/20 text-red-400 border border-red-500/20">
                          High Risk
                        </span>
                      )}
                      {q.riskLevel === "medium" && (
                        <span className="shrink-0 px-2 py-0.5 rounded text-[10px] font-bold tracking-wider uppercase bg-yellow-500/20 text-yellow-400 border border-yellow-500/20">
                          Medium Risk
                        </span>
                      )}
                      {q.riskLevel === "low" && (
                        <span className="shrink-0 px-2 py-0.5 rounded text-[10px] font-bold tracking-wider uppercase bg-green-500/10 text-green-400 border border-green-500/10">
                          Safe
                        </span>
                      )}
                    </div>
                    <div className="text-xs mt-1.5 space-y-1">
                      <div className="text-text-muted font-mono break-words bg-black/20 p-2 rounded border border-white/5">
                        <span className="text-text-muted/50 select-none mr-2">Query:</span>
                        {q.sanitizedQuery}
                      </div>
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </div>

          {error && (
            <div className="p-3 bg-red-500/10 border border-red-500/20 text-red-400 text-sm rounded-md">
              {error}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-5 border-t border-border bg-[#151514] flex justify-end gap-3">
          <Button variant="secondary" onClick={onCancel} disabled={isApproving}>
            Cancel
          </Button>
          <Button onClick={handleApprove} loading={isApproving}>
            Approve {selectedQueryIds.size} Queries
          </Button>
        </div>

      </div>
    </div>
  );
}
