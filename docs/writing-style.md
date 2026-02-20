# Documentation Writing Style

This project uses a consistent documentation voice: precise, pragmatic, mildly playful.

## Voice Contract

1. **Precise**: prefer concrete nouns, explicit commands, and real paths.
2. **Dryly human**: allow small bits of humor; do not turn docs into stand-up.
3. **Actionable**: every procedure includes commands and success criteria.
4. **Honest**: document limitations and sharp edges plainly.
5. **No hype**: avoid hyperbole and marketing tone.
6. **ASCII punctuation first**: prefer hyphen/comma/colon over typographic em-dashes.

## Tone Examples

Good:
- "Run `make check` before commit."
- "If this fails, capture exit code and first failing step."

Also good (with personality):
- "Boring checklists prevent exciting incidents."

Not good:
- "Everything should just work magically."
- "Trust the vibes."

## Formatting Rules

1. Use headings that match user intent (`Quickstart`, `Troubleshooting`, `Workflow`).
2. Use fenced code blocks for commands.
3. State expected outcomes, not just actions.
4. Link to adjacent docs instead of duplicating entire sections.
5. Do not use typographic em dash punctuation (`U+2014`) in operational docs.

## Maintenance Rules

1. Update docs in the same change set as behavior changes.
2. Remove stale TODO prose.
3. Favor short sections with clear scanability.
4. Run `make docs-check` before landing docs-heavy changes.

Yes, this file is a style guide. No, it does not issue lint errors (yet).
