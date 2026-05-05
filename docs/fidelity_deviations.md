# Fidelity Deviations

Intentional, documented deviations from base Friday Night Funkin' behavior.
The fidelity rule in `PLAN.md` Section 2 is "match first, improve only after
the mismatch is documented." This file is where the documentation lives.

A deviation belongs here only if:

1. We can reproduce the original behavior.
2. We have decided not to ship it.
3. We can articulate why.

Bug fixes for behaviors that were obviously unintended in the original (e.g.,
camera jitter from a typo in the source) belong in commit history, not here.

## Format

Each deviation is a `### <short title>` section with these fields:

- **Original behavior:** what base FNF does, with `// ref:` style citation.
- **RusticV3 behavior:** what we ship instead.
- **Reason:** the user-visible justification.
- **Affects fidelity contract:** yes / no, and how.

## Active deviations

(none yet — populate during gameplay porting)
