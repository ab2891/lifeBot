# Future OpenClaw Integration

Lifebot does not require OpenClaw, NanoClaw, or OpenClaw-lite to run. MVP 1 includes `packages/openclaw-adapter`, a mock local boundary that documents how an external orchestrator could call Lifebot's assistant tools later.

Lifebot also now includes an `Integrations` screen with a mock install-and-test flow so a future packaged release can offer a coordinator-friendly "click to enable" experience instead of asking users to clone or wire adapters by hand.

## Current contract

- Tool registration metadata
- Tool invocation envelope
- Structured JSON results
- Health check method

## Integration approach later

1. Replace the mock adapter transport with the real orchestrator transport.
2. Keep `packages/assistant-tools` as the stable internal tool surface.
3. Let the packaged app install or enable the adapter from the UI and persist the endpoint in app-managed settings.
4. Avoid coupling core business logic to a specific external runtime.

## Local development

The mock adapter can be used for local testing or future MCP/OpenClaw bridging work without affecting the desktop app runtime. The current install flow is intentionally a local mock so the UX path can be validated before introducing any real external dependency.
