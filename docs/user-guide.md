# User Guide — ORVIKA AI

Welcome to the ORVIKA AI (ORVIKA AI). This guide details how to navigate the local-first security-hardened desktop application for analyzing documents, transcripts, and cloud-assisted research.

## Table of Contents
1. [Getting Started](#getting-started)
2. [Core Features](#core-features)
   - [Local Chat](#local-chat)
   - [Document Management & RAG](#document-management--rag)
   - [Media & Audio Transcription](#media--audio-transcription)
   - [Dual-Source Research Assistant](#dual-source-research-assistant)
3. [Privacy & Security Configuration](#privacy--security-configuration)
4. [Frequently Asked Questions (FAQ)](#faq)

---

## Getting Started

### Installation
ORVIKA AI runs as a standalone desktop app on your device. Run the installer and complete the setup wizard.

### First-Time Onboarding
1. Select your local LLM model (e.g., Gemma 2 9B GGUF).
2. Allow the system to perform local hardware checks for GPU optimization.
3. Configure your API keys in the settings page if you wish to use Cloud Research Mode.

---

## Core Features

### Local Chat
Type prompts to query the local LLM. Chats stream in real-time and are persisted locally using a encrypted SQLite database.

### Document Management & RAG
- Drag-and-drop PDF/DOCX files.
- Files are parsed, chunked, and embedded into local FAISS vector databases.
- Chat queries retrieve relevant sections and display clickable page highlights.

### Media & Audio Transcription
- Upload video/audio files.
- Faster-Whisper performs offline transcription.
- Transcripts are synchronized with playback timestamps.

---

## Privacy & Security Configuration

You can configure three levels of privacy in Settings:
- **Maximum Privacy (Local Only)**: No outbound requests.
- **Moderate Privacy (Sanitized Cloud)**: Cloud requests run after stripping PII and content overlap.
- **Low Privacy (Standard Cloud)**: External queries are sent directly.

---

## FAQ
**Q: Where is my data stored?**  
A: All documents, embeddings, and chat histories remain strictly on your local machine.
