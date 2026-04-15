---
name: failure-observation-events
description: 'Model recoverable failures as persisted observation events instead of immediate terminal errors. Use when designing sync jobs, resumable imports, fetch pipelines, or idempotent workflows that should skip known failures for now while preserving future retry paths.'
argument-hint: 'Describe the workflow, failure mode, and desired retry behavior.'
---

# Failure Observation Events

## When to Use
- A batch or sync command can encounter expected remote failures such as `404`, `403`, rate limits, or "not yet available" states.
- Re-running the workflow immediately should not repeat the same failing work item over and over.
- The failure is meaningful domain data and should influence planning in later runs.
- The system still needs a path for future retries, such as refresh mode, policy changes, or a later observation window.
- Common cases in this repo include thumbnail URLs that return `404` or `403`, and fetch stages where an entity is unavailable for now but should not crash the whole sync.

## Goal
Capture a failure as its own event file or record, then make future planning logic treat that failure as an observed state instead of an unhandled crash.

This pattern keeps the workflow idempotent, preserves evidence, avoids wasteful immediate retries, and leaves room for deliberate retries later.

## Procedure
1. Classify the failure.
   Decide whether the failure is expected and domain-meaningful, or whether it is truly exceptional and should still fail the command.
2. Define a dedicated failure event type.
   Store the minimum data needed for future planning and debugging, such as the subject id, observed time, source URL or input, failure category, and status code or reason.
3. Persist the failure event in the same storage model as successful observations.
   Put it near the rest of the entity history so later planning can discover it without special-case external state.
4. Teach indexing and planning code to read the failure event.
   A future run should be able to say "this item is already known unavailable" and skip it in the default path.
5. Keep retry policy separate from persistence.
   The existence of a failure event should suppress immediate retries by default, but refresh or force modes should still be able to schedule the item again.
6. Continue the batch when the failure is recoverable.
   Record the failure, count it in summaries, and move on to the next work item.
7. Add regression tests.
   Cover at least one first-observation case and one subsequent-planning case where the prior failure changes behavior.

## Recommended Defaults
- Treat `404` and `403` from remote content hosts as unavailable observations, not terminal process errors.
- Store unavailable outcomes beside successful and unchanged observations so planning code sees one coherent history.
- In the normal sync path, skip items that already have a recent unavailable observation.
- In refresh or force paths, allow the same item to be scheduled again without deleting the older unavailable event.
- Count unavailable items explicitly in progress summaries so operators can distinguish them from crashes.

## Decision Points

### Persist Or Abort
- Persist as an event when the failure is expected from the problem domain and future runs can make a better decision because the observation exists.
- Abort when the failure indicates a broken invariant, corrupted local state, programmer error, or an unknown condition that needs investigation.

### Skip By Default Or Retry By Default
- Skip by default when immediate retries are unlikely to change the outcome and would waste time or spam logs.
- Retry by default only when the failure is clearly transient and low-cost, or when the workflow already has bounded backoff behavior.
- For this repo's sync commands, prefer skip-by-default after recording an unavailable observation, then re-check in explicit refresh flows.

### Reuse Existing Event Type Or Create A New One
- Reuse an existing event type only if the semantics already match.
- Create a new event type when the failure carries distinct meaning that planning, summaries, or later tooling must treat differently.

## Implementation Checklist
- Create a dedicated event schema for the failure observation.
- Add a canonical storage path for the event.
- Export the type from the storage module.
- Update index-loading logic so the event contributes to planning state.
- Update the executor to persist the event and continue for expected failures.
- If the workflow already has success and unchanged events, keep the new failure event adjacent to those patterns rather than inventing a separate persistence mechanism.
- Include the new outcome in progress summaries and end-of-run reporting.
- Add tests for persistence and skip-on-next-run behavior.

## Quality Bar
- Future runs can discover the prior failure without redoing the same failing work immediately.
- The persisted event contains enough metadata to explain what happened.
- Refresh or force flows can still retry later without deleting history.
- The command summary distinguishes successful, unchanged, skipped, and unavailable or failed-observation outcomes.
- Regression tests prove both the write path and the later planning behavior.

## Example Prompt Starters
- `/failure-observation-events Add unavailable-image handling to this sync stage.`
- `/failure-observation-events Review whether this 404 should become a persisted event instead of aborting.`
- `/failure-observation-events Design a retry policy for recorded remote failures in this importer.`
- `/failure-observation-events Make this sync command treat 403 and 404 as observation events and skip them on later default runs.`