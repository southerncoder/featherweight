# Changelog

All notable changes to Featherweight are documented in this file.

## Unreleased

- Recover screenplay layout from PDFs that indent with literal spaces instead of
  positioning text: a line's leading whitespace now sets its position, measured
  in a per-font space width estimated from the document, and trailing padding
  no longer counts toward line extents.
- Join wrapped lines with a single space when a line ends in padding, applying
  the sentence-spacing convention only at sentence boundaries, and drop
  whitespace at element edges (no more trailing spaces in cues or headings).
- Separate a title block that shares the first page with the script instead of
  leaving the whole page unrecognized.
- Recognize title blocks centered with spaces in text-formatted PDFs, which a
  proportional font no longer renders geometrically centered.
- Accept a bare "by" title-page credit, and keep title-page lines that no field
  claims as Notes instead of dropping them.
- Fix the package-contract test on Windows, where `npm.cmd` needs a shell.
- Add a Rust port (`rust/`) with byte-identical output, a one-call API and a
  matching `featherweight` CLI.

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
