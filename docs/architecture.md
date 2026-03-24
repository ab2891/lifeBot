# Architecture

Lifebot MVP 1 is a local-first desktop app with a Tauri shell, Svelte frontend, Rust domain layer, and SQLite persistence. The app is intentionally designed around replaceable provider boundaries so that future Sling, messaging, and orchestrator integrations can be added without reshaping the core scheduling engine.

## Layers

1. Desktop UI in `apps/desktop`
2. Tauri command boundary in `apps/desktop/src-tauri`
3. Domain and persistence logic in `crates/core`
4. Policy evaluation in `packages/policies`
5. Provider adapters in `packages/providers/*`
6. Assistant-tool facade in `packages/assistant-tools`
7. Future orchestration adapter in `packages/openclaw-adapter`

## Core design choices

- Desktop-first: the app runs locally and keeps its own SQLite database.
- Human-in-the-loop: schedule generation creates drafts for review rather than destructive automated sync.
- Explainable decisions: shift awards and rejections are recorded in `decision_traces`.
- Mockable boundaries: Sling, messaging, and orchestration all have stable interfaces and local mock implementations.
- Demo-safe: seeded data lets supervisors review realistic workflows without production credentials.
- App-managed integrations: end-user Sling and OpenClaw settings are stored by Lifebot itself through the `Integrations` screen rather than requiring manual `.env` editing.
