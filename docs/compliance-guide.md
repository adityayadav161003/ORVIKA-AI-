# Compliance Guide — ORVIKA AI

ORVIKA AI is engineered for regulatory compliance in enterprise environments, offering auditability, data privacy safeguards, and customizable compliance verification check policies.

## Regulatory Framework Map

### HIPAA (Health Insurance Portability and Accountability Act)
- **Local-first isolation**: Health data parsed in the document pipeline never leaves the machine unless explicitly configured.
- **AES-256-GCM storage**: SQLite database records and key stores are fully encrypted at rest.
- **Outbound sanitizer**: The query sanitization engine scans for and redacts Protected Health Information (PHI) before any external API calls.

### GDPR (General Data Protection Regulation)
- **Right to be forgotten**: Delete commands completely purge files and FAISS embeddings.
- **Local diagnostic opt-in**: Telemetry collection is strictly voluntary and stored locally.
- **Outbound request controls**: Users can inspect outbound requests in the Transparency Dashboard.

### SOX (Sarbanes-Oxley Act)
- **Audit Logs**: Outbound cloud requests, spending limits, and API actions are logged to a tamper-resistant SQLite database.
- **Local-first verification**: Compliance reports compile and map local activity records to internal governance controls.

---

## Custom Compliance Audits
ORVIKA AI allows enterprise admins to upload customized JSON templates defining compliance checkpoints. The system evaluates these rules against telemetry logs to yield reports.
