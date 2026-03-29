# CLI

This specification covers the planned command-line behavior for `teamy-youtube`.

## Command Surface

yt[command.surface.home]
The CLI must expose a `home` command group for showing and opening the roaming application-home directory.

yt[command.surface.cache]
The CLI must expose a `cache` command group for showing, opening, and cleaning the local throwaway cache directory.

yt[command.surface.sync]
The CLI must expose a `sync` command group for configuring the sync directory and ingesting datasources into the filesystem database.

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
The CLI must provide `sync dir set` and `sync dir show` commands.

yt[config.sync-directory.required-for-sync]
The `sync now` workflow must fail with a user-facing error if the sync directory is not configured.

yt[path.home.env-overrides-platform]
If `TEAMY_YOUTUBE_HOME` is set to a non-empty value, it must take precedence over the platform-derived application home directory.

yt[path.cache.env-overrides-platform]
If `TEAMY_YOUTUBE_CACHE_DIR` is set to a non-empty value, it must take precedence over the platform-derived cache directory.

yt[path.sync.env-overrides-config]
If `TEAMY_YOUTUBE_SYNC_DIR` is set to a non-empty value, it must take precedence over the persisted sync-directory setting.

## Sync Workflow

yt[sync.takeout.command]
The CLI must expose `sync now takeout` as the primary Google Takeout ingestion workflow.

yt[sync.takeout.default-discovery]
If `sync now takeout` is invoked without `--input-dir`, it must discover candidate takeout files using the `teamy-mft` crate rather than spawning the `teamy-mft` executable.

yt[sync.takeout.latest-history]
Default takeout discovery must select the most recent available `watch-history.json` candidate.

yt[sync.takeout.latest-playlists]
Default takeout discovery must select the most recent available CSV for each playlist.

yt[sync.takeout.dry-run]
`sync now takeout --dry-run` must print a short summary and must not write to the sync directory.

yt[sync.takeout.multiple-playlists]
Takeout sync must ingest playlist CSVs generically rather than being hard-coded only to Watch Later.