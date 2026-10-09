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
├── channels/
│   └── <channel-id>/
│       ├── target.json
│       └── event_<timestamp>_discover_videos.json
├── videos/
│   └── <video-id>/
│       ├── event_<timestamp>_watched.json
│       ├── event_<timestamp>_added-to-playlist-<playlist-id>.json
│       ├── event_<timestamp>_fetch_video_data.json
│       ├── event_<timestamp>_fetch_video_data_missing.json
│       ├── event_<timestamp>_fetch_video_data_unavailable.json
│       ├── event_<timestamp>_download_video_requested.json
│       ├── event_<timestamp>_download_video_completed.json
│       ├── event_<timestamp>_download_video_failed.json
│       ├── event_<timestamp>_observe_title_<title>.txt
│       ├── event_<timestamp>_thumbnail_120x90.jpg
│       ├── event_<timestamp>_thumbnail_120x90_unchanged.json
│       └── ...future generic events and assets...
```

The exact event shapes will keep evolving, but the stable direction is source-agnostic event files and assets under `videos/<video-id>/`, plus durable channel targets under `channels/<channel-id>/`. Thumbnail assets are keyed by their dimensions when known, the event timestamp reflects when the thumbnail was observed or re-checked, and download-request plus terminal download-result events let future commands enqueue work by video ID without coupling discovery directly to download execution.

## Intended Command Groups

- `home`: show or open the roaming home directory
- `cache`: show, open, or clean the local cache directory
- `api`: persist and validate API credentials used by external metadata fetchers
- `fetch`: fetch raw external metadata events such as YouTube Data API video responses
- `sync`: show, open, or set the sync directory, inspect remaining work, ingest datasources into the filesystem database, and manage tracked channel downloads

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
cargo run -- sync --bail-if-mft-older-than 1day status
cargo run -- sync
cargo run -- sync takeout --dry-run
cargo run -- sync takeout --dry-run --input-dir C:\Users\TeamD\OneDrive\Documents\Backups\takeout\takeout-20260326T232255Z-3-001
cargo run -- sync videos --fetch-limit 25
cargo run -- sync thumbnails
cargo run -- sync thumbnails --limit 25
cargo run -- sync thumbnails --refresh-videos-newer-than 2d --refresh-thumbnails-older-than 6h
cargo run -- sync channel add https://www.youtube.com/@jblow888/videos I:\YouTube\jblow888
cargo run -- sync channel --dry-run --bail-if-mft-older-than 1day
cargo run -- sync channel --bail-if-mft-older-than 1day
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
- `sync status` requires the sync dir to be configured and summarizes missing fetches, thumbnail work, tracked channels, queued downloads, and teamy-mft cache freshness.
- `api key set <value>` persists the `YouTube` API key under the application home directory.
- `api key validate` checks that the configured `YouTube` API key can successfully call the `YouTube` Data API.
- `fetch video <id>` requires the sync dir to be configured and a usable `YouTube` API key to be available either from `YOUTUBE_API_KEY` or the persisted home-directory config.
- `fetch video <id>` writes a terminal raw fetch event for that video: either `event_<timestamp>_fetch_video_data.json`, `event_<timestamp>_fetch_video_data_missing.json`, or `event_<timestamp>_fetch_video_data_unavailable.json`.
- `sync videos` fetches raw video API responses for videos already referenced in the sync database and can stop early with `--fetch-limit`.
- `sync thumbnails` ensures thumbnails exist for each size variant discovered in the latest successful raw video fetch event.
- `sync thumbnails` can stop early with `--limit` to inspect only a bounded number of videos during testing.
- By default, `sync thumbnails` does not re-download a thumbnail size when a materialized thumbnail asset for that size already exists.
- `sync thumbnails --refresh-videos-newer-than <age> --refresh-thumbnails-older-than <age>` enables refresh mode for recently published videos whose latest thumbnail observation is old enough to justify another check.
- When a refresh finds the thumbnail bytes are unchanged, `sync thumbnails` writes `event_<timestamp>_thumbnail_<size>_unchanged.json` instead of duplicating the asset bytes.
- `sync channel add <url-or-handle> <dir>` persists a tracked channel target under `channels/<channel-id>/target.json` and remembers the preferred download directory for future runs.
- `sync channel --dry-run` previews the current plan without writing discovery or request events and without downloading anything.
- `sync channel` uses `yt-dlp` for channel discovery, writes per-run `event_<timestamp>_discover_videos.json` files under each tracked channel, enqueues missing videos by writing `event_<timestamp>_download_video_requested.json`, and writes terminal `download_video_completed` or `download_video_failed` events under each video.
- `sync channel` uses teamy-mft indexed path queries to skip videos whose media files already exist anywhere on indexed disks, even if they live outside the preferred download directory.
- `sync channel` also scans the preferred download directory during planning, so recent local downloads are recognized even when teamy-mft has not been refreshed yet.
- `sync --bail-if-mft-older-than <age>` overrides the default 2-hour freshness gate for teamy-mft-backed stages.
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
## Read Takeout ZIPs without importing

These commands inspect/read one ZIP directly and never extract it, import into the sync
directory, fetch metadata, acquire an API key, or refresh MFT. No sync-directory setting is required.

```powershell
teamy-youtube takeout inspect 'G:\exports\takeout-20260101T000000Z-001.zip'
teamy-youtube takeout read 'G:\exports\takeout-20260101T000000Z-001.zip' --output-format json
teamy-youtube takeout read --input-dir 'G:\exports' --output-format csv
teamy-youtube takeout inspect
```

An explicit path may have any filename. Omit it to select automatically. Repeat
`--input-dir` to recursively search multiple roots; symlink directories are skipped.
If no root is provided, `TEAMY_YOUTUBE_TAKEOUT_DIR` supplies one configured root.
Otherwise selection queries published MFT paths, subject to existing MFT filter rules.
MFT's existing snapshot freshness check defaults to two hours (`--mft-max-age 2h`).
Stale/unavailable indexes fail with instructions to use an explicit path or discovery root;
this command does not start the daemon or rebuild/refresh indexes. Recent or filtered-out
archives can be absent from published indexes, including when an overlay is newer than the base snapshot.

Automatic eligibility requires a standard `takeout-YYYYMMDDTHHMMSSZ-<numeric-part>.zip`
filename and a recognized YouTube `history/watch-history.json` central-directory entry.
The newest valid UTC filename timestamp wins; modification time is never a fallback.
The result explains discovery source, date basis, candidate eligibility, and selected file snapshot.
Ties fail, including same-date split exports: choose one ZIP explicitly. Split archives are
not merged. Renamed archives require explicit selection. Unreadable/invalid candidates are
reported as rejected; unrelated exports are ignored. Candidate discovery is capped at 256 ZIPs
and 100000 directory entries; narrow the root when exceeded.

`inspect` returns selection details and a sorted entry inventory without decoding watch rows.
`read` returns that inventory plus normalized watch records with video ID, RFC3339 watch time,
title, optional channel name/URL/ID, archive path, entry name, and zero-based record index.
Channel IDs are derived only from recognized YouTube `/channel/UC…` URLs; handles are retained
as URLs without network resolution. Watch events do not imply complete viewing. Missing URLs
and unsupported video URLs are skipped and counted; no channel identity is invented.
History-only JSON exports need no playlist files. Playlist CSV and HTML-history sections are
inventoried but not decoded in this milestone. `read` fails clearly when JSON history is absent.

ZIP reads enforce 100000 entries and a 256 MiB watch-entry limit by default. Override with
`--max-entries` and `--max-entry-bytes`. Only the recognized history entry is decompressed;
unrelated/media entries are not read. This is bounded **entry buffering**, then in-memory JSON
array decoding, not true record streaming or whole-archive buffering. Traversal/absolute entry
paths, duplicate normalized names, multiple history entries, malformed ZIP/JSON, checksum errors,
and oversized history entries fail. Entry paths are never extracted. File size/mtime snapshots
detect ordinary modifications during selection/read; they are not cryptographic identity checks.

Text and JSON include the full report. CSV contains flat inventory rows (`inspect`) or watch
rows (`read`), with provenance on each watch row. Existing global `--log-file` still writes logs
only when explicitly requested. Library consumers can use `ArchiveSelectionRequest`,
`select_archive_from_candidates`, `SelectedArchive`, `ArchiveReadRequest`, and `read_archive`
without involving the CLI, current event storage, or YouTube credentials. Storage-provider
decoupling and backend selection remain deferred.
