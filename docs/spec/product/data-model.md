# Data Model

This specification describes the canonical filesystem-backed model for `teamy-youtube`.

## Storage Strategy

yt[storage.filesystem.canonical]
The canonical store for YouTube entities and derived state must be a structured directory tree rooted in the configured sync directory on the local filesystem.

yt[storage.imports.noncanonical]
Imported sources such as Google Takeout exports and `YouTube` Data API responses must be treated as inputs to the canonical store rather than the long-term source of truth.

## Entity Layout

yt[storage.video-root.layout]
Videos must be stored beneath `videos/<video-id>/` so that the canonical on-disk path is keyed only by the immutable video ID.

yt[storage.video-id-stable-key]
Each video's immutable YouTube video ID must be part of the path used to store that video's local records.

yt[storage.video-directory.exact-id]
Each canonical video directory name must exactly equal the immutable YouTube video ID.

yt[storage.video-directory.omits-event-verbs]
Video directory names must not be prefixed with event-specific verbs such as `watched`.

yt[storage.video-event-layout]
The sync directory must support a generic video-centric layout of `videos/<video-id>/event_<timestamp>_<event-id>.<ext>`.

yt[storage.video-metadata.fetch-layout]
Raw video fetch events must be stored at `videos/<video-id>/event_<timestamp>_fetch_video_data.json`.

yt[storage.video-metadata.negative-fetch-layout]
Terminal negative fetch outcomes must be stored at `videos/<video-id>/event_<timestamp>_fetch_video_data_missing.json` or `videos/<video-id>/event_<timestamp>_fetch_video_data_unavailable.json`.

yt[storage.video-title-observation.layout]
Derived title observations may be stored at `videos/<video-id>/event_<timestamp>_observe_title_<title>.txt` for local search visibility.

yt[storage.video-thumbnail.layout]
Downloaded thumbnail assets must be stored at `videos/<video-id>/event_<thumbnail-observed-at>_thumbnail_<size>.<ext>`.

yt[storage.video-thumbnail.unchanged-layout]
Unchanged thumbnail refresh observations must be stored at `videos/<video-id>/event_<thumbnail-observed-at>_thumbnail_<size>_unchanged.json`.

## Raw Events And Observations

yt[storage.video-metadata.raw-first]
Successful API fetches must preserve the raw response body in-kind so that downstream enrichments can be re-derived without re-fetching.

yt[storage.takeout.watch-history.events]
Watch history must be representable as timestamped events keyed by the referenced video ID.

yt[storage.playlist-membership.events]
Playlist membership must be represented as timestamped events or observations rather than as a single mutable membership file.

yt[storage.events.source-agnostic]
Event file shapes and paths must be source-agnostic so that the same database can accept takeout-derived observations and API-derived observations without separate storage hierarchies.

## Incremental Enrichment

yt[enrichment.accepts-partial-entities]
The local model must support partially known videos so that takeout data can be recorded before full API metadata has been fetched.

yt[enrichment.links-sources]
The local model must preserve provenance linking local records back to the source takeout export or import operation that produced them.