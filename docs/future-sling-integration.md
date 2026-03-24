# Future Sling Integration

MVP 1 ships with `packages/providers/sling-mock`, which reads local fixtures and exposes a typed schedule import/export interface. No real Sling API calls are made in this repo.

Lifebot now includes an in-app `Integrations` screen where a coordinator can paste a Sling API key and optional workspace ID. Those values are stored by Lifebot in local app-managed settings so a packaged rollout does not depend on end users creating `.env` files.

## Current contract

- `import_schedule_snapshot()`
- `export_draft_schedule()`
- `get_sync_status()`

## For a future real adapter

1. Add a new provider crate beside `sling-mock`.
2. Implement the same provider trait.
3. Map Sling concepts into Lifebot's internal scheduling cycle, shift template, shift, and assignment models.
4. Read credentials from the app-managed settings store first, with `.env` as a developer fallback.
5. Keep sync non-destructive until an explicit approval step is introduced.

## Important guardrails

- Lifebot remains an overlay, not a Sling replacement.
- Draft schedules should stay reviewable before export.
- Import/export jobs should always log audit entries and sync state.
- Credential entry should stay inside the app for non-technical users.
