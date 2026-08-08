# Changelog

All notable changes to Featherweight are documented in this file.

## 0.1.3 - 2026-08-08

- Keep adjacent conventional scene headings separate instead of merging a
  full-width heading with its neighbor.
- Accept paired scene-number fragments only when they match the scene-number
  grammar.
- Require actual copyright text before recognizing an independent title-page
  copyright field.
- Join en- and em-dash action continuations across page breaks without an
  inserted space, matching the ASCII hyphen behavior.
- Retain continued character cues when the preceding dialogue does not end the
  prior page.
- Derive pagination and artifact detection from per-page content extents so
  mixed page sizes and shifted pages are handled correctly.
- Require page-boundary evidence before continuing dialogue across pages.
- Suppress recurring page artifacts only within page margin bands; recurring
  body text is retained and diagnosed instead of removed.
- Force Fountain action lines that would otherwise parse as title-page
  metadata at the start of a document, and stop double-marking scene headings
  that already begin with a period.
- Document that screenplay text resembling Fountain syntax is emitted
  verbatim.
- Warn on stderr when converted pages need OCR, and refuse output paths that
  refer to the input PDF through symlinks, hard links, or case-insensitive
  spellings.
- Validate native PDF classification results before trusting them.

## 0.1.2 - 2026-08-06

- Preserve adjacent inset action lines as newline-delimited blocks.
- Detect scene numbers printed on the left, right, or both sides of scene
  headings, including numbered nonstandard headings.

## 0.1.1 - 2026-08-05

- Correct the public conversion API to accept PDF bytes and native/OCR page
  routes while keeping positioned-text processing internal.

## 0.1.0 - 2026-08-05

First public release.

- Inspect PDFs for physical page count and pages without usable embedded text.
- Recover canonical screenplay documents from caller-routed positioned text.
- Recognize title pages, scene headings, action, dialogue, dual dialogue,
  parentheticals, lyrics, transitions, centered text, inline styling, and page
  boundaries.
- Serialize screenplay documents as JSON, Fountain, and FDX.
- Convert native-text PDFs from the `featherweight` command line.
