# Command completion budgets

Every leaf explicitly returns an advisory `elapsed_duration_warn_threshold`. The entrypoint measures invocation and rendering, warns only when elapsed is strictly greater than the budget, and preserves invocation/output errors. Budget metadata is separate from JSON/CSV/text reports, including structured `facet_rows` JSON and flat CSV. This is a completion warning, not a watchdog. Formulas use inventory/planned work, never actual elapsed or success counts. Existing phase-specific tracing remains intact.

- Home/cache/sync-dir show/set, API-key set, tracked-channel add: 1 second for local state inspection/write.
- Home/cache/sync-dir open: 3 seconds for detached folder opening.
- Cache clean: 1 second plus 250 ms per removed top-level cache directory (advisory deletion expectation).
- API-key validate and single-video metadata fetch: 35 seconds for one HTTP request and local setup/snapshot writes.
- Sync videos: 5 seconds setup + 20 ms per inventory candidate + 30 seconds per planned metadata request.
- Sync thumbnails: 5 seconds setup + 20 ms per inventory candidate + 30 seconds per planned thumbnail request. Refresh and limit options change that plan.
- Sync Takeout: 5 seconds setup + 100 ms per playlist CSV + 2 ms per source watch/playlist event for dry-run preview, or 20 ms per source event for database import.
- Sync all: sum the three stage budgets, retaining each stage's work model.
- Sync channel: 5 seconds setup + 20 ms per discovered video + 30 seconds per target discovery + 10 minutes per planned video download. Dry-run uses stored inventory only: 20 ms per target and no download component. The per-download allowance expresses an average video expectation; long videos may legitimately warn.
- Sync status: 5 seconds setup + 20 ms per combined video/thumbnail/channel inventory item; no service/download phase.
- Takeout inspect: 5 seconds selection + 2 ms per central-directory entry.
- Takeout read: 5 seconds selection + 1 second per whole MiB of history entry data + 2 ms per inventory/decoded/skipped record item. Existing ZIP selection and read limits remain intact.

Tests cover below/equal/above warnings and explicit exemptions, saturating work scaling, report JSON/CSV separation including facet_rows, and planned-request budget changes independent of completed results. Existing synthetic Takeout tests exercise representative archive/import commands.