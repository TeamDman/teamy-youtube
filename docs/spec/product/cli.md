# CLI

This specification covers the command-line behavior for `teamy-youtube`.

## Command Surface

yt[command.surface.home]
The CLI must expose a `home` command group for showing and opening the roaming application-home directory.

yt[command.surface.cache]
The CLI must expose a `cache` command group for showing, opening, and cleaning the local throwaway cache directory.

yt[command.surface.api]
The CLI must expose an `api` command group for persisting and validating external API credentials.

yt[command.surface.sync]
The CLI must expose a `sync` command group for configuring the sync directory and ingesting datasources into the filesystem database.

yt[command.surface.fetch]
The CLI must expose a `fetch` command group for retrieving raw external metadata events.

## Parser Model

yt[parser.args-consistent]
The structured CLI model must serialize to command line arguments consistently for parse-safe values.

yt[parser.roundtrip]
The structured CLI model must roundtrip through argument serialization and parsing for parse-safe values.

## Configuration

yt[config.home-directory]
The CLI must resolve a configured home directory for roaming preferences and small config files.

yt[config.sync-directory]
The CLI must support a dedicated sync directory that contains the canonical on-disk YouTube dataset.

yt[config.sync-directory.set-show]
The CLI must provide `sync dir set`, `sync dir show`, and `sync dir open` commands.

yt[config.sync-directory.open-opens-manager]
`sync dir open` must open the configured sync directory in the platform file manager.

yt[config.sync-directory.required-for-sync]
The `sync` workflow must fail with a user-facing error if the sync directory is not configured.

yt[path.home.env-overrides-platform]
If `TEAMY_YOUTUBE_HOME` is set to a non-empty value, it must take precedence over the platform-derived application home directory.

yt[path.cache.env-overrides-platform]
If `TEAMY_YOUTUBE_CACHE_DIR` is set to a non-empty value, it must take precedence over the platform-derived cache directory.

yt[path.sync.env-overrides-config]
If `TEAMY_YOUTUBE_SYNC_DIR` is set to a non-empty value, it must take precedence over the persisted sync-directory setting.

yt[path.youtube-api-key.env-overrides-config]
If `YOUTUBE_API_KEY` is set to a non-empty value, it must take precedence over the persisted `YouTube` API key.

yt[api.key.set.command]
The CLI must expose `api key set <value>` for persisting a `YouTube` API key under the application home directory.

yt[api.key.validate.command]
The CLI must expose `api key validate` for validating the configured `YouTube` API key.

yt[api.key.validate.uses-configured-key]
`api key validate` must use the configured `YouTube` API key from either `YOUTUBE_API_KEY` or the persisted home-directory configuration.

yt[fetch.video.requires-api-key]
`fetch video <id>` must fail with a user-facing error if no usable `YouTube` API key is available from either `YOUTUBE_API_KEY` or the persisted home-directory configuration.

yt[fetch.video.sync-dir-required]
`fetch video <id>` must fail with a user-facing error if the sync directory is not configured.

yt[fetch.video.command]
The CLI must expose `fetch video <id>` for retrieving metadata for a specific YouTube video ID.

yt[fetch.video.writes-terminal-events]
`fetch video <id>` must write a terminal fetch result beneath the video's sync-directory folder.

yt[fetch.video.raw-response-persistence]
When a `YouTube` Data API fetch succeeds, `fetch video <id>` must persist the raw API response body as `event_<timestamp>_fetch_video_data.json`.

yt[fetch.video.negative-result-persistence]
When a `YouTube` Data API fetch reports that a video is missing or unavailable, `fetch video <id>` must persist a timestamped terminal event describing that outcome.

## Sync Workflow

yt[sync.takeout.command]
The CLI must expose `sync takeout` as the primary Google Takeout ingestion workflow.

yt[sync.fetch-videos.command]
The CLI must expose `sync videos` for fetching raw video API responses for videos already present in the filesystem database.

yt[sync.fetch-videos.limit]
`sync videos` may accept `--fetch-limit` to stop after fetching a bounded number of missing videos for testing.

yt[sync.fetch-videos.terminal-skip]
`sync videos` must skip videos that already have any terminal fetch event recorded in the filesystem database.

yt[sync.fetch-videos.raw-persistence]
`sync videos` must persist raw API response bodies and terminal negative results into the filesystem database.

yt[sync.thumbnails.command]
The CLI must expose `sync thumbnails` for downloading thumbnail assets from previously fetched raw video data.

yt[sync.thumbnails.limit]
`sync thumbnails` may accept `--limit` to stop after inspecting a bounded number of videos for testing.

yt[sync.thumbnails.latest-fetch]
`sync thumbnails` must derive thumbnails from the latest successful raw fetch event recorded for each video.

yt[sync.thumbnails.event-assets]
`sync thumbnails` must write thumbnail files using the thumbnail-observation timestamp, with a canonical shape of `event_<timestamp>_thumbnail_<size>.<ext>`.

yt[sync.thumbnails.size-keyed-assets]
`sync thumbnails` must key thumbnail asset filenames by the thumbnail dimensions when those dimensions are available.

yt[sync.thumbnails.default-no-refetch]
Without explicit refresh arguments, `sync thumbnails` must only ensure that a thumbnail asset exists and must not re-download sizes that already have a materialized thumbnail asset.

yt[sync.thumbnails.refresh-video-age]
`sync thumbnails` may accept `--refresh-videos-newer-than <age>` to limit refresh attempts to recently published videos.

yt[sync.thumbnails.refresh-thumbnail-age]
`sync thumbnails` may accept `--refresh-thumbnails-older-than <age>` to limit refresh attempts to thumbnails whose latest observation is old enough.

yt[sync.thumbnails.refresh-requires-both-ages]
If either thumbnail-refresh age argument is provided, the other must also be provided.

yt[sync.thumbnails.unchanged-event]
If a thumbnail refresh fetches bytes identical to the latest materialized thumbnail asset for that size, `sync thumbnails` must record an unchanged event instead of duplicating the asset contents.

yt[sync.all.command]
The CLI must expose bare `sync` with no nested sync subcommand, and it must run `sync takeout`, then `sync videos`, then `sync thumbnails` with default arguments.

yt[sync.takeout.default-discovery]
If `sync takeout` is invoked without `--input-dir`, it must discover candidate takeout files using the `teamy-mft` crate rather than spawning the `teamy-mft` executable.

yt[sync.takeout.latest-history]
Default takeout discovery must select the most recent available `watch-history.json` candidate.

yt[sync.takeout.latest-playlists]
Default takeout discovery must select the most recent available CSV for each playlist.

yt[sync.takeout.dry-run]
`sync takeout --dry-run` must print a short summary, must not write to the sync directory, and must include a sample preview of the canonical output paths for a video that appears in both watch history and at least one playlist when such an overlap exists.

yt[sync.takeout.multiple-playlists]
Takeout sync must ingest playlist CSVs generically rather than being hard-coded only to Watch Later.

yt[sync.takeout.playlist-membership-events]
Takeout sync must write playlist membership events for parsed playlist CSV rows in addition to watch-history events.