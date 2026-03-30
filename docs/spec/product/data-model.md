# Data Model

This specification describes the canonical filesystem-backed model for `teamy-youtube`.

## Storage Strategy

yt[storage.filesystem.canonical]
The canonical store for YouTube entities and derived state must be a structured directory tree rooted in the configured sync directory on the local filesystem.

yt[storage.imports.noncanonical]
Imported sources such as Google Takeout exports and Postgres exports must be treated as inputs to the canonical store rather than the long-term source of truth.

## Entity Layout

yt[storage.channel-video-hierarchy]
Videos must be stored beneath their owning channel so that a stable channel-oriented directory hierarchy exists on disk.

yt[storage.video-id-stable-key]
Each video's immutable YouTube video ID must be part of the path used to store that video's local records.

yt[storage.video-directory.id-prefix]
Each video directory name must begin with the immutable YouTube video ID and may append a human-readable slug suffix after it.

yt[storage.video-directory.omits-event-verbs]
Video directory names must not be prefixed with event-specific verbs such as `watched`.

yt[storage.channel-video-event-layout]
The sync directory must support a generic layout of `channels/<channel-slug>/videos/<video-id>-<video-slug>/event_<timestamp>_<event-id>.json`.

## Snapshots And Events

yt[storage.video-metadata.snapshots]
Video metadata observations must be stored as timestamped snapshots so that changes over time can be preserved.

yt[storage.channel-metadata.snapshots]
Channel metadata observations must be stored as timestamped snapshots so that changes over time can be preserved.

yt[storage.playlist-membership.events]
Playlist membership must be represented as timestamped events or observations rather than as a single mutable membership file.

yt[storage.watch-history.events]
Watch history must be representable as timestamped events keyed by the referenced video ID.

yt[storage.events.source-agnostic]
Event file shapes and paths must be source-agnostic so that the same database can later accept takeout, Postgres-derived, or API-derived observations.

## Incremental Enrichment

yt[enrichment.accepts-partial-entities]
The local model must support partially known videos so that takeout data can be recorded before full API metadata has been fetched.

yt[enrichment.links-sources]
The local model must preserve provenance linking local records back to the source takeout export or import operation that produced them.