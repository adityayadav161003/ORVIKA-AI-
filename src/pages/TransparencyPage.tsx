import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";

interface AuditLogEntry {
  id: number;
  timestamp: string;
  eventType: string;
  sessionId?: string;
  details: string;
  outboundContent?: string;
  destination?: string;
  responseSummary?: string;
  sanitizationResult?: string;
  riskLevel?: string;
}

interface AuditStats {
  totalRequests: number;
  blockedRequests: number;
  piiDetected: number;
  healthScore: number;
  riskBreakdown: {
    low: number;
    medium: number;
    high: number;
  };
}

interface CompliancePrinciple {
  name: string;
  status: string;
  description: string;
}

interface ComplianceReport {
  regulation: string;
  principles: CompliancePrinciple[];
  summary: Record<string, string | number>;
}

export function TransparencyPage() {
  const [activeTab, setActiveTab] = useState<"logs" | "compliance" | "settings">("logs");
  
  // Logs Tab
  const [logs, setLogs] = useState<AuditLogEntry[]>([]);
  const [stats, setStats] = useState<AuditStats>({
    totalRequests: 0,
    blockedRequests: 0,
    piiDetected: 0,
    healthScore: 100,
    riskBreakdown: { low: 0, medium: 0, high: 0 }
  });
  const [selectedLog, setSelectedLog] = useState<AuditLogEntry | null>(null);
  
  // Filters
  const [filterType, setFilterType] = useState<string>("");
  const [filterStartDate, setFilterStartDate] = useState<string>("");
  const [filterEndDate, setFilterEndDate] = useState<string>("");
  const [searchText, setSearchText] = useState<string>("");

  // Compliance Tab
  const [regulation, setRegulation] = useState<"gdpr" | "hipaa" | "sox">("gdpr");
  const [complianceReport, setComplianceReport] = useState<ComplianceReport | null>(null);
  const [generatingReport, setGeneratingReport] = useState<boolean>(false);

  // Settings Tab
  const [spendingLimit, setSpendingLimit] = useState<string>("50.0");
  const [currentSpending, setCurrentSpending] = useState<string>("0.0");
  const [autoApproveThreshold, setAutoApproveThreshold] = useState<string>("never");
  const [savingSettings, setSavingSettings] = useState<boolean>(false);
  const [settingsFeedback, setSettingsFeedback] = useState<string | null>(null);

  // Fetch initial logs, stats, and settings
  const loadData = useCallback(async () => {
    try {
      const dbStats = await invoke<AuditStats>("get_audit_stats");
      setStats(dbStats);

      const dbLogs = await invoke<AuditLogEntry[]>("list_audit_logs", {
        eventType: filterType || undefined,
        startDate: filterStartDate ? `${filterStartDate} 00:00:00` : undefined,
        endDate: filterEndDate ? `${filterEndDate} 23:59:59` : undefined,
      });
      setLogs(dbLogs);

      // Load Settings
      const limit = await invoke<string | null>("get_setting", { key: "security.api_spending_limit" });
      if (limit) setSpendingLimit(limit);

      const current = await invoke<string | null>("get_setting", { key: "security.api_spending_current" });
      if (current) setCurrentSpending(current);

      const threshold = await invoke<string | null>("get_setting", { key: "security.auto_approve_threshold" });
      if (threshold) setAutoApproveThreshold(threshold);

    } catch (err) {
      console.error("Error loading transparency data:", err);
    }
  }, [filterType, filterStartDate, filterEndDate]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  const handleClearLogs = async () => {
    if (!window.confirm("Are you sure you want to permanently clear all audit logs?")) return;
    try {
      await invoke("clear_audit_logs");
      await loadData();
    } catch (err) {
      console.error(err);
    }
  };

  const handleGenerateReport = async () => {
    setGeneratingReport(true);
    setComplianceReport(null);
    try {
      const res = await invoke<ComplianceReport>("generate_compliance_report", {
        regulation,
        startDate: filterStartDate ? `${filterStartDate} 00:00:00` : undefined,
        endDate: filterEndDate ? `${filterEndDate} 23:59:59` : undefined,
      });
      setComplianceReport(res);
    } catch (err) {
      console.error(err);
    } finally {
      setGeneratingReport(false);
    }
  };

  const handleSaveSettings = async () => {
    setSavingSettings(true);
    setSettingsFeedback(null);
    try {
      await invoke("set_setting", { key: "security.api_spending_limit", value: spendingLimit });
      await invoke("set_setting", { key: "security.auto_approve_threshold", value: autoApproveThreshold });
      setSettingsFeedback("Settings saved successfully.");
      setTimeout(() => setSettingsFeedback(null), 3000);
      await loadData();
    } catch (err) {
      setSettingsFeedback(`Error: ${String(err)}`);
    } finally {
      setSavingSettings(false);
    }
  };

  const handleResetSpending = async () => {
    if (!window.confirm("Reset monthly spending counter to $0.00?")) return;
    try {
      await invoke("reset_api_spending");
      await loadData();
    } catch (err) {
      console.error(err);
    }
  };

  const downloadReportFile = () => {
    if (!complianceReport) return;
    const jsonString = `data:text/json;charset=utf-8,${encodeURIComponent(
      JSON.stringify(complianceReport, null, 2)
    )}`;
    const downloadAnchor = document.createElement("a");
    downloadAnchor.setAttribute("href", jsonString);
    downloadAnchor.setAttribute("download", `compliance_report_${regulation}.json`);
    document.body.appendChild(downloadAnchor);
    downloadAnchor.click();
    downloadAnchor.remove();
  };

  // Filter logs locally by search text
  const filteredLogs = logs.filter(log => {
    if (!searchText) return true;
    const query = searchText.toLowerCase();
    return (
      log.details.toLowerCase().includes(query) ||
      (log.destination && log.destination.toLowerCase().includes(query)) ||
      (log.outboundContent && log.outboundContent.toLowerCase().includes(query)) ||
      (log.riskLevel && log.riskLevel.toLowerCase().includes(query))
    );
  });

  return (
    <div className="flex h-full flex-col overflow-y-auto px-8 py-6">
      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="font-serif text-3xl font-bold text-text-primary">🛡 Privacy & Transparency</h1>
          <p className="text-sm text-text-secondary">
            Audit logs of outbound traffic, compliance posture reports, and privacy policies.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={() => setActiveTab("logs")}
            className={`rounded-lg px-4 py-2 text-sm font-medium transition ${
              activeTab === "logs" ? "bg-accent text-white" : "bg-surface hover:bg-border text-text-primary"
            }`}
          >
            Outbound Logs
          </button>
          <button
            onClick={() => setActiveTab("compliance")}
            className={`rounded-lg px-4 py-2 text-sm font-medium transition ${
              activeTab === "compliance" ? "bg-accent text-white" : "bg-surface hover:bg-border text-text-primary"
            }`}
          >
            Compliance Reports
          </button>
          <button
            onClick={() => setActiveTab("settings")}
            className={`rounded-lg px-4 py-2 text-sm font-medium transition ${
              activeTab === "settings" ? "bg-accent text-white" : "bg-surface hover:bg-border text-text-primary"
            }`}
          >
            Settings
          </button>
        </div>
      </div>

      {activeTab === "logs" && (
        <div className="space-y-6">
          {/* Stats Bar */}
          <div className="grid grid-cols-4 gap-4">
            <div className="rounded-xl border border-border bg-white p-5 shadow-sm">
              <span className="text-xs font-mono uppercase tracking-wider text-text-muted">Privacy Health</span>
              <div className="mt-2 flex items-center gap-3">
                <span
                  className={`flex h-10 w-10 items-center justify-center rounded-full font-bold text-white text-sm ${
                    stats.healthScore >= 90
                      ? "bg-emerald-500"
                      : stats.healthScore >= 70
                      ? "bg-amber-500"
                      : "bg-red-500"
                  }`}
                >
                  {stats.healthScore}%
                </span>
                <div>
                  <div className="text-sm font-semibold text-text-primary">
                    {stats.healthScore >= 90 ? "Excellent" : stats.healthScore >= 70 ? "Needs Review" : "At Risk"}
                  </div>
                  <div className="text-xs text-text-muted">Zero unauthorized egress</div>
                </div>
              </div>
            </div>

            <div className="rounded-xl border border-border bg-white p-5 shadow-sm">
              <span className="text-xs font-mono uppercase tracking-wider text-text-muted">Outbound Queries</span>
              <div className="mt-2 flex items-baseline gap-2">
                <span className="text-3xl font-bold text-text-primary">{stats.totalRequests}</span>
                <span className="text-xs text-text-muted">calls logged</span>
              </div>
            </div>

            <div className="rounded-xl border border-border bg-white p-5 shadow-sm">
              <span className="text-xs font-mono uppercase tracking-wider text-text-muted">Redacted PII Incidents</span>
              <div className="mt-2 flex items-baseline gap-2">
                <span className="text-3xl font-bold text-amber-500">{stats.piiDetected}</span>
                <span className="text-xs text-text-muted">redacted</span>
              </div>
            </div>

            <div className="rounded-xl border border-border bg-white p-5 shadow-sm">
              <span className="text-xs font-mono uppercase tracking-wider text-text-muted">Blocked Leak Attempts</span>
              <div className="mt-2 flex items-baseline gap-2">
                <span className="text-3xl font-bold text-red-500">{stats.blockedRequests}</span>
                <span className="text-xs text-text-muted">halted</span>
              </div>
            </div>
          </div>

          {/* Filters & Actions */}
          <div className="flex flex-wrap items-center gap-3 rounded-lg border border-border bg-surface p-4">
            <div className="w-48">
              <Input
                placeholder="Search logs..."
                value={searchText}
                onChange={e => setSearchText(e.target.value)}
              />
            </div>
            <select
              value={filterType}
              onChange={e => setFilterType(e.target.value)}
              className="h-10 rounded-md border border-border bg-white px-3 text-sm text-text-primary focus:outline-none"
            >
              <option value="">All Events</option>
              <option value="cloud_call">Cloud Outbound Requests</option>
              <option value="blocked">Blocked Request Leaks</option>
              <option value="pii_detected">PII Redactions</option>
            </select>
            <div className="flex items-center gap-2">
              <span className="text-xs text-text-muted">From:</span>
              <input
                type="date"
                value={filterStartDate}
                onChange={e => setFilterStartDate(e.target.value)}
                className="h-10 rounded-md border border-border bg-white px-3 text-sm focus:outline-none"
              />
              <span className="text-xs text-text-muted">To:</span>
              <input
                type="date"
                value={filterEndDate}
                onChange={e => setFilterEndDate(e.target.value)}
                className="h-10 rounded-md border border-border bg-white px-3 text-sm focus:outline-none"
              />
            </div>
            <div className="ml-auto flex gap-2">
              <Button size="sm" variant="secondary" onClick={handleClearLogs} className="text-red-500 hover:bg-red-50">
                Clear Logs
              </Button>
            </div>
          </div>

          {/* Logs List */}
          <div className="overflow-hidden rounded-xl border border-border bg-white shadow-sm">
            <table className="w-full border-collapse text-left text-sm">
              <thead className="bg-surface font-mono text-xs uppercase tracking-wider text-text-muted">
                <tr>
                  <th className="px-6 py-3 font-semibold">Timestamp</th>
                  <th className="px-6 py-3 font-semibold">Event Type</th>
                  <th className="px-6 py-3 font-semibold">Destination</th>
                  <th className="px-6 py-3 font-semibold">Risk Level</th>
                  <th className="px-6 py-3 font-semibold">Action Taken</th>
                  <th className="px-6 py-3 text-right font-semibold">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {filteredLogs.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="px-6 py-12 text-center text-text-muted">
                      No matching audit logs found. Outbound requests are protected.
                    </td>
                  </tr>
                ) : (
                  filteredLogs.map(log => (
                    <tr key={log.id} className="hover:bg-surface/50">
                      <td className="whitespace-nowrap px-6 py-4 font-mono text-xs text-text-muted">
                        {log.timestamp}
                      </td>
                      <td className="px-6 py-4">
                        <span
                          className={`inline-flex rounded-full px-2 py-0.5 text-xs font-medium ${
                            log.eventType === "blocked"
                              ? "bg-red-50 text-red-700"
                              : log.eventType === "pii_detected"
                              ? "bg-amber-50 text-amber-700"
                              : "bg-blue-55 text-blue-700"
                          }`}
                        >
                          {log.eventType === "blocked"
                            ? "Leak Blocked"
                            : log.eventType === "pii_detected"
                            ? "PII Redacted"
                            : "Cloud Call"}
                        </span>
                      </td>
                      <td className="px-6 py-4 font-mono text-xs text-text-primary">
                        {log.destination || "Local System"}
                      </td>
                      <td className="px-6 py-4">
                        <span
                          className={`inline-flex rounded-full px-2 py-0.5 text-xs font-semibold ${
                            log.riskLevel === "high"
                              ? "bg-red-100 text-red-800"
                              : log.riskLevel === "medium"
                              ? "bg-amber-100 text-amber-800"
                              : "bg-emerald-100 text-emerald-800"
                          }`}
                        >
                          {log.riskLevel ? log.riskLevel.toUpperCase() : "LOW"}
                        </span>
                      </td>
                      <td className="px-6 py-4 text-text-secondary truncate max-w-xs">{log.details}</td>
                      <td className="whitespace-nowrap px-6 py-4 text-right">
                        <Button size="sm" variant="ghost" onClick={() => setSelectedLog(log)}>
                          Details
                        </Button>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>

          {/* Details Modal */}
          {selectedLog && (
            <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/45 p-4 backdrop-blur-sm">
              <div className="w-full max-w-2xl rounded-xl border border-border bg-white p-6 shadow-xl">
                <div className="mb-4 flex items-center justify-between border-b border-border pb-3">
                  <h3 className="font-serif text-xl font-bold text-text-primary">Request Audit Details</h3>
                  <button onClick={() => setSelectedLog(null)} className="text-text-muted hover:text-text-primary">
                    ✕
                  </button>
                </div>
                <div className="space-y-4">
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <span className="block text-xs font-mono text-text-muted uppercase">Timestamp</span>
                      <span className="text-sm font-mono text-text-primary">{selectedLog.timestamp}</span>
                    </div>
                    <div>
                      <span className="block text-xs font-mono text-text-muted uppercase">Destination</span>
                      <span className="text-sm font-mono text-text-primary">
                        {selectedLog.destination || "Local Execution"}
                      </span>
                    </div>
                  </div>

                  <div>
                    <span className="block text-xs font-mono text-text-muted uppercase">System Details</span>
                    <p className="mt-1 text-sm text-text-primary bg-surface p-3 rounded border border-border">
                      {selectedLog.details}
                    </p>
                  </div>

                  {selectedLog.outboundContent && (
                    <div>
                      <span className="block text-xs font-mono text-text-muted uppercase">Original Input</span>
                      <pre className="mt-1 max-h-32 overflow-y-auto rounded border border-border bg-red-50/20 p-3 font-mono text-xs text-red-800">
                        {selectedLog.outboundContent}
                      </pre>
                    </div>
                  )}

                  {selectedLog.sanitizationResult && (
                    <div>
                      <span className="block text-xs font-mono text-text-muted uppercase">Sanitized / Egress Content</span>
                      <pre className="mt-1 max-h-32 overflow-y-auto rounded border border-border bg-emerald-50/20 p-3 font-mono text-xs text-emerald-800">
                        {selectedLog.sanitizationResult}
                      </pre>
                    </div>
                  )}

                  {selectedLog.responseSummary && (
                    <div>
                      <span className="block text-xs font-mono text-text-muted uppercase">Response Findings Summary</span>
                      <pre className="mt-1 max-h-32 overflow-y-auto rounded border border-border bg-surface p-3 font-mono text-xs text-text-secondary">
                        {selectedLog.responseSummary}
                      </pre>
                    </div>
                  )}
                </div>
                <div className="mt-6 flex justify-end">
                  <Button onClick={() => setSelectedLog(null)}>Close</Button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === "compliance" && (
        <div className="space-y-6">
          {/* Report Setup */}
          <div className="rounded-xl border border-border bg-white p-6 shadow-sm">
            <h3 className="mb-4 text-lg font-bold text-text-primary">Compliance Report Exporter</h3>
            <div className="flex flex-wrap items-end gap-4">
              <div className="flex-1 min-w-[200px]">
                <label className="mb-1.5 block text-xs font-medium text-text-secondary">Select Regulation Standard</label>
                <select
                  value={regulation}
                  onChange={e => setRegulation(e.target.value as "gdpr" | "hipaa" | "sox")}
                  className="h-10 w-full rounded-md border border-border bg-white px-3 text-sm focus:outline-none"
                >
                  <option value="gdpr">GDPR (General Data Protection Regulation)</option>
                  <option value="hipaa">HIPAA (Health Insurance Portability & Accountability)</option>
                  <option value="sox">SOX (Sarbanes-Oxley Act)</option>
                </select>
              </div>

              <div>
                <Button onClick={handleGenerateReport} disabled={generatingReport}>
                  {generatingReport ? "Generating Compliance Audit..." : "Generate Audit Report"}
                </Button>
              </div>
            </div>
          </div>

          {/* Compliance Report Output */}
          {complianceReport && (
            <div className="rounded-xl border border-border bg-white p-6 shadow-sm space-y-6">
              <div className="flex items-center justify-between border-b border-border pb-4">
                <div>
                  <h2 className="text-xl font-bold text-text-primary">{complianceReport.regulation}</h2>
                  <p className="text-xs text-text-muted">Generated on: {new Date().toLocaleString()}</p>
                </div>
                <div className="flex gap-2">
                  <Button size="sm" variant="secondary" onClick={downloadReportFile}>
                    Export JSON
                  </Button>
                </div>
              </div>

              {/* Status Checklist */}
              <div>
                <h4 className="mb-3 font-mono text-xs uppercase tracking-wider text-accent">Audited Compliance Standards</h4>
                <div className="space-y-3">
                  {complianceReport.principles?.map((p: CompliancePrinciple, idx: number) => (
                    <div key={idx} className="flex items-start gap-3 rounded-lg border border-border/80 bg-surface p-4">
                      <span className="rounded-md bg-emerald-100 px-2 py-0.5 font-mono text-xs font-semibold text-emerald-800">
                        {p.status}
                      </span>
                      <div>
                        <p className="text-sm font-semibold text-text-primary">{p.name}</p>
                        <p className="mt-1 text-xs text-text-secondary">{p.description}</p>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Summary Values */}
              <div>
                <h4 className="mb-3 font-mono text-xs uppercase tracking-wider text-accent">Audit Metrics Data</h4>
                <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                  {Object.entries(complianceReport.summary || {}).map(([key, val]) => (
                    <div key={key} className="rounded-lg border border-border bg-surface p-4">
                      <span className="text-xs font-mono text-text-muted uppercase capitalize">
                        {key.replace(/([A-Z])/g, " $1")}
                      </span>
                      <div className="mt-1 text-lg font-bold text-text-primary">{String(val)}</div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === "settings" && (
        <div className="space-y-6">
          <div className="rounded-xl border border-border bg-white p-6 shadow-sm space-y-6">
            <h3 className="text-lg font-bold text-text-primary">Policy & Egress Budget Settings</h3>

            {settingsFeedback && (
              <div className="rounded bg-accent/10 p-3 text-sm text-accent">
                {settingsFeedback}
              </div>
            )}

            <div className="space-y-4">
              {/* Spending Limit */}
              <div className="flex flex-wrap items-center justify-between gap-4 border-b border-border pb-4">
                <div>
                  <h4 className="text-sm font-semibold text-text-primary">API Monthly Spending Limit ($)</h4>
                  <p className="text-xs text-text-muted">
                    Set a hard cap limit for cloud intelligence queries. Calls will be blocked when exceeded.
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-xs text-text-muted">Limit:</span>
                  <div className="w-24">
                    <Input
                      type="number"
                      value={spendingLimit}
                      onChange={e => setSpendingLimit(e.target.value)}
                    />
                  </div>
                  <span className="text-xs text-text-muted">Spent:</span>
                  <span className="font-mono text-sm font-semibold text-text-primary">${parseFloat(currentSpending).toFixed(3)}</span>
                  <Button size="sm" variant="secondary" onClick={handleResetSpending} className="ml-2">
                    Reset Spent
                  </Button>
                </div>
              </div>

              {/* Auto Approve Risk Threshold */}
              <div className="flex flex-wrap items-center justify-between gap-4 border-b border-border pb-4">
                <div>
                  <h4 className="text-sm font-semibold text-text-primary">Auto-Approve Risk Policy</h4>
                  <p className="text-xs text-text-muted">
                    Automatically bypass query review screens if the generated plan risk matches this threshold.
                  </p>
                </div>
                <div>
                  <select
                    value={autoApproveThreshold}
                    onChange={e => setAutoApproveThreshold(e.target.value)}
                    className="h-10 rounded-md border border-border bg-white px-3 text-sm text-text-primary focus:outline-none"
                  >
                    <option value="never">Never (Always Manually Review Plan)</option>
                    <option value="low">Auto-Approve Low Risk Queries Only</option>
                    <option value="medium">Auto-Approve Low & Medium Risk Queries</option>
                    <option value="high">Auto-Approve All (Dangerous Outbound Allowed)</option>
                  </select>
                </div>
              </div>
            </div>

            <div className="flex justify-end">
              <Button onClick={handleSaveSettings} disabled={savingSettings}>
                {savingSettings ? "Saving Settings..." : "Save Settings"}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
