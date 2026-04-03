# teamy-youtube

A command line interface for interacting with YouTube history, playlists, Google Takeout exports, and YouTube metadata using a filesystem-first data model.

## Current Direction

This repository is being shaped around a few working decisions:

- `figue` is the preferred CLI parser for this project.
- The canonical data store should live on disk in a dedicated sync directory rather than in Postgres.
- Existing Postgres-backed and takeout-ingest work from the Nanuak workspace should be reused as import sources, not treated as the long-term source of truth.
- The primary workflow is `sync run`, with Google Takeout as the first datasource.

## Why Filesystem First

The older YouTube tooling in the workspace already proves out several useful pieces:

- Google Takeout watch history parsing
- YouTube Data API metadata population
- search and embedding workflows over video metadata

The main problem with the current setup is that the useful information is spread across multiple tools and is too tightly coupled to Postgres. For this repository, the filesystem is the primary storage layer so that:

- metadata snapshots can be inspected directly
- playlist membership can be modeled as append-only events
- local search tools can query the corpus without a database dependency
- importing from older tools is optional rather than required

## Directory Roles

- `TEAMY_YOUTUBE_HOME` stores small roaming preferences such as the configured sync directory.
- `TEAMY_YOUTUBE_CACHE_DIR` stores throwaway local cache content.
- `TEAMY_YOUTUBE_SYNC_DIR` or `teamy-youtube sync dir set <path>` selects the durable filesystem database.
- `YOUTUBE_API_KEY` overrides the persisted `YouTube` API key when set.

## Sync Directory Layout

```text
TEAMY_YOUTUBE_SYNC_DIR/
├── channels/
│   └── <channel-slug>/
│       ├── snapshot_<timestamp>_channel.json
│       └── videos/
│           └── <video-id>-<video-slug>/
│               ├── event_<timestamp>_watched.json
│               ├── event_<timestamp>_added-to-playlist-<playlist-id>.json
│               ├── snapshot_<timestamp>_video.json
│               └── ...future generic events...
```

The exact event shapes will keep evolving, but the stable direction is source-agnostic event files under `channels/<channel>/videos/<video>/`.

## Intended Command Groups

- `home`: show or open the roaming home directory
- `cache`: show, open, or clean the local cache directory
- `api`: persist and validate API credentials used by external metadata fetchers
- `fetch`: fetch metadata snapshots from external sources such as the YouTube Data API
- `sync`: show, open, or set the sync directory, then ingest datasources into the filesystem database

## Example Usage

```powershell
cargo run -- home show
cargo run -- cache clean
cargo run -- api key set your-youtube-api-key
cargo run -- api key validate
cargo run -- fetch video XfcLWVX-hCA
cargo run -- sync dir set ~/Downloads/teamy-youtube-sync
cargo run -- sync dir show
cargo run -- sync dir open
cargo run -- sync run takeout --dry-run
cargo run -- sync run takeout --dry-run --input-dir C:\Users\TeamD\OneDrive\Documents\Backups\takeout\takeout-20260326T232255Z-3-001
cargo run -- sync run postgres --database-url postgres://postgres:postgres@localhost/teamy_youtube
```

## Immediate Goal

The first implementation target is a filesystem-backed pipeline that can:

1. discover the latest Google Takeout history and playlist exports
2. read all playlist CSVs plus watch history
3. normalize those files into generic event records in the sync directory
4. preserve provenance back to the original takeout files
5. leave room for later sync sources such as Postgres or API-derived metadata

## Current Behavior

- `sync run takeout` requires the sync dir to be configured first.
- `sync run postgres` requires the sync dir to be configured first.
- `sync run postgres` syncs generic event files from the fsdb into a Postgres `youtube_video_events` table, then syncs rows from that table back into missing fsdb event files.
- `sync run postgres` resolves the Postgres connection string from `--database-url`, `TEAMY_YOUTUBE_DATABASE_URL`, or `DATABASE_URL`.
- `api key set <value>` persists the `YouTube` API key under the application home directory.
- `api key validate` checks that the configured `YouTube` API key can successfully call the `YouTube` Data API.
- `fetch video <id>` requires the sync dir to be configured and a usable `YouTube` API key to be available either from `YOUTUBE_API_KEY` or the persisted home-directory config.
- If `--input-dir` is omitted, takeout discovery uses the `teamy-mft` crate to query indexed files and pick the most recent `watch-history.json` plus the most recent version of each playlist CSV.
- `--dry-run` prints a count summary, skips writing event files, and previews the canonical output paths for a sample video that appears in both watch history and a playlist when such an overlap exists.

## PowerShell Helper

The repository includes [Get-YouTubeAPIKey.ps1](Get-YouTubeAPIKey.ps1), which will:

- run `cargo run -- api key validate`
- if no usable key is configured, read a key from 1Password via `op read`
- persist it with `cargo run -- api key set ...`
- validate the stored key
- print Google Cloud Console URLs for API-key and API-enablement management if validation fails

Set `TEAMY_YOUTUBE_1PASSWORD_YOUTUBE_API_KEY_REFERENCE` or pass `-OnePasswordReference` when calling the script.

## Related Workspace Repositories

- `Nanuak/nanuak-youtube-takeout-ingest`: existing takeout parsing into Postgres
- `Nanuak/nanuak-youtube-populate-details`: existing YouTube API metadata fetcher
- `Nanuak/nanuak-youtube-history-search`: older database-backed search workflow
- `Nanuak/nanuak-youtube-embeddings`: semantic workflows over fetched metadata
- `teamy-rust-cli`: preferred `figue`-based template and Tracey scaffolding source

## Specification

The initial requirements live under `docs/spec` and are wired through `.config/tracey/config.styx`.