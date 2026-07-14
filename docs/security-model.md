# Security Model — ORVIKA AI

ORVIKA AI operates under a zero-trust, local-first security architecture. This document outlines threat models, encryption policies, and data exfiltration preventions.

## Core Security Tenets
1. **Local-First Executions**: Database operations, document chunking, and llama.cpp inference execute locally.
2. **Encrypted Persistence**: All databases, settings, and documents are stored with hardware-bound AES-256-GCM.
3. **Outbound Data Scrubbing**: Outbound queries are passed through strict sanitization engines before transmission.

## Sanitization Engine
The sanitization pipeline employs regex and named entity recognition to block:
- Social Security Numbers (SSN)
- Phone Numbers and Emails
- Filenames and directory paths
- Verbatim document block overlaps
