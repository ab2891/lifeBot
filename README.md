# Lifebot MVP 1

Lifebot is a desktop-first aquatics operations assistant that works with Sling data instead of replacing Sling. MVP 1 runs fully in local demo mode with seeded data, a local SQLite database, mock provider boundaries, explainable scheduling decisions, and a plain-language assistant panel for non-technical staff.

## What is in this repo

- `apps/desktop`: Tauri desktop shell with a Svelte frontend
- `crates/core`: SQLite-backed domain models, migrations, seed data, and scheduling logic
- `packages/policies`: data-driven policy evaluation
- `packages/providers/sling-mock`: mock Sling import/export boundary with fixtures
- `packages/providers/messaging`: console, in-app, fake SMS, and fake GroupMe messaging providers with audit logging
- `packages/assistant-tools`: internal tool surface for UI and future orchestrators
- `packages/openclaw-adapter`: mock OpenClaw-compatible adapter boundary
- `packages/shared-types`: frontend shared TypeScript types
- `docs`: architecture and future integration notes
- `scripts/seed`: local demo bootstrap helpers

## MVP capabilities

- Guard profile management with certifications, roles, preferences, and notes
- Scheduling cycles, recurring shift templates, current assignments, rollover requests, and open request queues
- Data-driven policy checks for age, certifications, max hours, overlap, and time windows
- Draft next-cycle generation with structured decision traces
- Messaging provider abstraction with local audit/event log
- Mock Sling provider and future real Sling adapter boundary
- Plain-language assistant actions mapped to internal tools
- Fully local seeded demo mode with admin/demo toggle

## Quick start

1. Install JavaScript dependencies:

```bash
npm install
```

2. Start the desktop app:

```bash
npm run dev
```

This launcher clears stale Lifebot dev processes first, builds the frontend with relative local asset paths, and starts the Tauri desktop app without depending on a localhost dev server.

On Linux, Tauri also needs the usual system GTK/WebKit prerequisites. If `npm run dev` fails during Rust compilation with missing `glib-2.0`, `gobject-2.0`, or WebKitGTK packages, install the Tauri Linux dependencies first, then rerun.

3. Run tests:

```bash
npm test
```

4. Re-seed the demo database if needed:

```bash
npm run seed:demo
```

The app boots into local demo mode and does not require a Sling account. End users do not need to create a `.env` file. Developer-only defaults still exist in [.env.example](/home/agustin/projects/lifeBot/.env.example), but Sling and OpenClaw settings can now be entered and saved directly from the app's `Integrations` screen.

## Integration setup for non-technical users

Use the `Integrations` tab inside Lifebot to:

- paste a Sling API key and optional workspace ID
- save and test the Sling setup without editing files
- install the mock OpenClaw adapter from a button
- save the adapter endpoint and run a local contract test

These settings are stored by Lifebot in its local app data store so coordinators do not need repo access or shell commands once the desktop app is packaged.

## Linux desktop prerequisites for Tauri

Typical Ubuntu/Debian packages:

```bash
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

The backend tests and seeded demo database can still run before those packages are installed.

## Demo flow

Use the draft generator and request queue screens to walk through:

- an incumbent who requested before the rollover deadline and keeps a shift
- an incumbent who missed the deadline and loses priority
- a queue with multiple requesters ordered first-come-first-serve
- an ineligible requester skipped for missing certification
- an ineligible requester skipped for age/time restrictions
- structured reasoning in the decision trace drawer

## What is mocked

- Sling integration uses local fixtures and a typed provider boundary
- GroupMe and SMS are fake providers that still write to the audit log
- OpenClaw/NanoClaw integration is represented by a stable adapter boundary and mock local contract

## What needs real credentials later

- Real Sling adapter implementation, using the in-app saved credentials
- Real SMS provider implementation
- Real GroupMe provider implementation
- Optional future OpenClaw-compatible orchestrator connection behind the installable adapter flow

See [docs/architecture.md](/home/agustin/projects/lifeBot/docs/architecture.md), [docs/future-sling-integration.md](/home/agustin/projects/lifeBot/docs/future-sling-integration.md), and [docs/future-openclaw-integration.md](/home/agustin/projects/lifeBot/docs/future-openclaw-integration.md).
