# Admin Guide — ORVIKA AI

This guide outlines enterprise deployment configurations, remote team synchronization services, and Single Sign-On (SSO) integration options.

## Team Sync Coordination

ORVIKA AI uses a push/pull replication sync protocol to keep team workspaces aligned.
Admins can deploy a remote coordinator service and configure clients with:

- **Sync Coordinator URL**: HTTPS endpoint for syncing database deltas.
- **Sync Interval**: Interval in seconds between synchronization sweeps.

## Enterprise SSO Authentication

Clients authenticate with OIDC/OAuth2 providers before establishing trust with sync coordinators.

- **OIDC Discovery URL**: OIDC provider discovery metadata.
- **SSO Authentication Token**: JWT token supplied to authorize client sync connections.

## Custom Model Registry

Deploy a local model repository directory to host approved GGUF files.
Configure clients with the **Custom Model Registry URL** to auto-list corporate-cleared models.
