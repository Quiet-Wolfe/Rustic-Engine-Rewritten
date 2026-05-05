# Event Bus Versions

The typed event bus declared in `rustic-core` is a forward-compatible API
because it is the v1 mod surface (`PLAN.md` Sections 5 and 13).

## Compatibility rules

- Public event structs and enums are `#[non_exhaustive]`.
- Adding a field or a variant is a non-breaking change. No version bump.
- Removing or renaming a field, variant, or event type is a breaking change.
  It requires a major event bus version bump and an entry in this file.
- Changing the meaning of an existing field counts as breaking. Add a new
  field instead and deprecate the old one in a minor entry.

## Current version

`v1` (in development).

## History

| Version | Date | Change |
| --- | --- | --- |
| v1 | TBD | Initial. Defines `EventBus` trait, `BusEvent` enum, asset reload events, conductor snapshots, gameplay/judgment events, and screen lifecycle events. |

When you cut a new major, copy the prior version row and prepend the new one
above it. Each row should explain the user-visible reason for the bump in one
sentence so old mods can find the migration note quickly.
