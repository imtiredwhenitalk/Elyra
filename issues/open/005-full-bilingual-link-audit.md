# Full bilingual link and anchor audit

- Status: open
- Labels: documentation, i18n
- Area: `docs/en/**`, `docs/ua/**`, root READMEs

## Problem

The documentation set is bilingual with reciprocal language switchers, but new
pages and future edits can drift: English-compatible anchors must exist on
Ukrainian pages, and every relative link must resolve in both trees.

## Proposed work

- Add a script that walks both documentation trees and validates relative links,
  fragments, and language switchers.
- Run it in CI for documentation changes.
- Report cross-language pages that lack matching anchors.

## Acceptance criteria

- [ ] The script fails on broken links, missing switchers, or missing anchors.
- [ ] Both languages pass with the current 21-page-per-language set.
- [ ] EN/UA docs explain how to run the check locally.
