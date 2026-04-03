# teamy-youtube

A command line interface for interacting with YouTube history, playlists, Google Takeout exports, and YouTube metadata using a filesystem-first data model.

## Current Direction

This repository is being shaped around a few working decisions:

- `figue` is the preferred CLI parser for this project.
- The canonical data store should live on disk in a dedicated sync directory rather than in Postgres.
- Existing Nanuak workspace tools are useful as reference material for data shapes and workflows, not as an active storage layer for this repository.
- The primary workflow is `sync`, with Google Takeout as the first datasource.

## Why Filesystem First

The older YouTube tooling in the workspace already proves out several useful pieces:

- Google Takeout watch history parsing
- YouTube Data API metadata population
- search and embedding workflows over video metadata

The main problem with the current setup is that the useful information is spread across multiple tools and is too tightly coupled to Postgres. For this repository, the filesystem is the primary storage layer so that:

- raw API responses and derived files can be inspected directly
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
├── videos/
│   └── <video-id>/
│       ├── event_<timestamp>_watched.json
│       ├── event_<timestamp>_added-to-playlist-<playlist-id>.json
│       ├── event_<timestamp>_fetch_video_data.json
│       ├── event_<timestamp>_fetch_video_data_missing.json
│       ├── event_<timestamp>_fetch_video_data_unavailable.json
│       ├── event_<timestamp>_observe_title_<title>.txt
│       ├── event_<timestamp>_thumbnail_120x90.jpg
│       ├── event_<timestamp>_thumbnail_120x90_unchanged.json
│       └── ...future generic events and assets...
```

The exact event shapes will keep evolving, but the stable direction is source-agnostic event files and assets under `videos/<video-id>/`. Thumbnail assets are keyed by their dimensions when known, and the event timestamp reflects when the thumbnail was observed or re-checked.

## Intended Command Groups

- `home`: show or open the roaming home directory
- `cache`: show, open, or clean the local cache directory
- `api`: persist and validate API credentials used by external metadata fetchers
- `fetch`: fetch raw external metadata events such as YouTube Data API video responses
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
cargo run -- sync
cargo run -- sync takeout --dry-run
cargo run -- sync takeout --dry-run --input-dir C:\Users\TeamD\OneDrive\Documents\Backups\takeout\takeout-20260326T232255Z-3-001
cargo run -- sync videos --fetch-limit 25
cargo run -- sync thumbnails
cargo run -- sync thumbnails --refresh-videos-newer-than 2d --refresh-thumbnails-older-than 6h
```

## Immediate Goal

The first implementation target is a filesystem-backed pipeline that can:

1. discover the latest Google Takeout history and playlist exports
2. read all playlist CSVs plus watch history
3. normalize those files into generic event records in the sync directory
4. preserve provenance back to the original takeout files
5. fetch and persist raw API data and derived assets in later sync stages

## Current Behavior

- `sync takeout`, `sync videos`, `sync thumbnails`, and bare `sync` require the sync dir to be configured first.
- `api key set <value>` persists the `YouTube` API key under the application home directory.
- `api key validate` checks that the configured `YouTube` API key can successfully call the `YouTube` Data API.
- `fetch video <id>` requires the sync dir to be configured and a usable `YouTube` API key to be available either from `YOUTUBE_API_KEY` or the persisted home-directory config.
- `fetch video <id>` writes a terminal raw fetch event for that video: either `event_<timestamp>_fetch_video_data.json`, `event_<timestamp>_fetch_video_data_missing.json`, or `event_<timestamp>_fetch_video_data_unavailable.json`.
- `sync videos` fetches raw video API responses for videos already referenced in the sync database and can stop early with `--fetch-limit`.
- `sync thumbnails` ensures thumbnails exist for each size variant discovered in the latest successful raw video fetch event.
- By default, `sync thumbnails` does not re-download a thumbnail size when a materialized thumbnail asset for that size already exists.
- `sync thumbnails --refresh-videos-newer-than <age> --refresh-thumbnails-older-than <age>` enables refresh mode for recently published videos whose latest thumbnail observation is old enough to justify another check.
- When a refresh finds the thumbnail bytes are unchanged, `sync thumbnails` writes `event_<timestamp>_thumbnail_<size>_unchanged.json` instead of duplicating the asset bytes.
- Bare `sync` runs `takeout`, `videos`, then `thumbnails` in that order with their default arguments.
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
- `Nanuak/nanuak-youtube-history-search`: older search workflow over previously fetched metadata
- `Nanuak/nanuak-youtube-embeddings`: older semantic workflow over fetched metadata
- `teamy-rust-cli`: preferred `figue`-based template and Tracey scaffolding source

## Specification

The initial requirements live under `docs/spec` and are wired through `.config/tracey/config.styx`.