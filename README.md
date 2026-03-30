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

## Sync Directory Layout

```text
TEAMY_YOUTUBE_SYNC_DIR/
├── channels/
│   └── <channel-slug>/
│       └── videos/
│           └── <video-slug>/
│               ├── <timestamp>-watched.json
│               ├── <timestamp>-added-to-playlist-<playlist-slug>.json
│               └── ...future generic events...
```

The exact event shapes will keep evolving, but the stable direction is source-agnostic event files under `channels/<channel>/videos/<video>/`.

## Intended Command Groups

- `home`: show or open the roaming home directory
- `cache`: show, open, or clean the local cache directory
- `sync`: show or set the sync directory, then ingest datasources into the filesystem database

## Example Usage

```powershell
cargo run -- home show
cargo run -- cache clean
cargo run -- sync dir set ~/Downloads/teamy-youtube-sync
cargo run -- sync dir show
cargo run -- sync run takeout --dry-run
cargo run -- sync run takeout --dry-run --input-dir C:\Users\TeamD\OneDrive\Documents\Backups\takeout\takeout-20260326T232255Z-3-001
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
- If `--input-dir` is omitted, takeout discovery uses the `teamy-mft` crate to query indexed files and pick the most recent `watch-history.json` plus the most recent version of each playlist CSV.
- `--dry-run` prints a count summary and skips writing event files.

## Related Workspace Repositories

- `Nanuak/nanuak-youtube-takeout-ingest`: existing takeout parsing into Postgres
- `Nanuak/nanuak-youtube-populate-details`: existing YouTube API metadata fetcher
- `Nanuak/nanuak-youtube-history-search`: older database-backed search workflow
- `Nanuak/nanuak-youtube-embeddings`: semantic workflows over fetched metadata
- `teamy-rust-cli`: preferred `figue`-based template and Tracey scaffolding source

## Specification

The initial requirements live under `docs/spec` and are wired through `.config/tracey/config.styx`.