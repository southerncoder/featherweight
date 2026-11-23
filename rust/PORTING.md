# Featherweight (Rust)

Turn a screenplay PDF into Fountain, FDX, or structured JSON. This crate is a
behavioral port of the TypeScript library in `../src`: for the same PDF and page
routes it produces byte-identical JSON, Fountain, and FDX, and its CLI matches
the TypeScript CLI's output, messages, and exit codes.

Requires Rust 1.88+. Pure Rust, no system libraries.

## Using as a library

```toml
[dependencies]
# Pin a commit (or tag) of https://github.com/southerncoder/featherweight:
featherweight = { git = "https://github.com/southerncoder/featherweight", rev = "<sha>" }
# or a local checkout:
# featherweight = { path = "path/to/featherweight/rust/crates/featherweight" }
```

### `import_screenplay`

The entry point for applications. It inspects the PDF once, extracts the pages
with embedded text in batches, recognizes the screenplay, and never panics.

```rust
use std::sync::atomic::AtomicBool;
use featherweight::{ImportError, ImportOptions, ImportWarning, Progress, ScreenplayElement};

let pdf = std::fs::read("script.pdf")?;
let cancel = AtomicBool::new(false);            // set from another thread to cancel
let on_progress = |progress: Progress| { /* phase, done, total */ };

match featherweight::import_screenplay_with(&pdf, ImportOptions { on_progress: Some(&on_progress), cancel: Some(&cancel) }) {
    Ok(import) => {
        for warning in &import.warnings {
            let ImportWarning::PagesNeedOcr { pages } = warning; // 0-based page indexes
        }
        for element in &import.document.elements {
            match element {
                ScreenplayElement::SceneHeading { text, scene_number, page } => {} // page: 0-based source page
                ScreenplayElement::Character { text, .. } => {}
                _ => {}
            }
        }
    }
    Err(error) => {
        // error.code(): NOT_A_PDF, ENCRYPTED, CORRUPT, NO_EMBEDDED_TEXT, CANCELLED, INTERNAL
        // error.to_string(): a message for end users; std::error::Error::source() has the cause.
    }
}
```

- `import_screenplay(&pdf)` is the same with default options.
- **Progress:** `Phase::Inspecting`, `Extracting` (reported per batch of
  `EXTRACTION_BATCH_PAGES` pages, with `done`/`total` counting pages),
  `Normalizing`, `Recognizing` (each once, at its start, `0/1`). Callbacks run on
  the importing thread; run the import off your UI thread.
- **Cancel:** checked before each batch and phase; returns `ImportError::Cancelled`.
- **Scanned pages:** listed in `Import::inspection.pages_needing_ocr` and in an
  `ImportWarning::PagesNeedOcr` warning, and left out of the document. A PDF with
  no embedded text at all is `ImportError::NoEmbeddedText`, not an empty document.
- **Errors:** `ImportError` serializes as `{ "code": "...", "message": "..." }`.
  `NotAPdf` (empty or no `%PDF-` header), `Encrypted` (a password is required;
  owner-password-only PDFs open normally), `Corrupt` (anything else the parser
  rejects), `NoEmbeddedText`, `Cancelled`, `Internal` (a bug; panics are caught).
- **Provenance:** every element from `import_screenplay` carries `page` (0-based
  source page). The older functions leave it unset, so their JSON is unchanged.
- **Helpers:** `StyledText::plain_text()`; `Character::split()` returns a
  `CharacterCue { name, extensions, continued }` (`BEN (V.O.) (CONT'D)` →
  `BEN`, `["V.O."]`, `true`).
- **JSON and TypeScript:** every type serializes in the TypeScript port's JSON
  shape. `rust/bindings/featherweight.d.ts` declares them (generated with the
  `typescript` feature); `test/brick-and-steel.json` and
  `test/brick-and-steel.import.json` are golden samples.

### Features

| Feature | Default | Enables |
|---|---|---|
| `fountain` | yes | `screenplay_to_fountain`, the `featherweight` binary |
| `fdx` | yes | `screenplay_to_fdx`, the `featherweight` binary |
| `typescript` | no | `featherweight::typescript::declarations()` via `ts-rs` |

`default-features = false` keeps `import_screenplay` and the document types.
Dependencies: `pdf-inspector` (pure-Rust parsing via `lopdf`, with its AES/MD5/SHA
crates for protected PDFs, `rayon`, `env_logger`), `regex`, `serde`,
`serde_json`, `thiserror`; no OCR, ML, image, or network crates.

### Lower-level API (as in the TypeScript port)

Text is `StyledText { runs: Vec<TextRun { text, styles }> }`, with styles from
`Bold`, `Italic`, `Underline`, `Strikeout`. Every document type implements serde
`Serialize`/`Deserialize` in the TypeScript JSON shape.

```rust
let inspection = featherweight::inspect_screenplay_pdf(&pdf)?;
let (native, ocr) = inspection.page_routes(); // scanned pages -> OCR (empty until OCR exists)
let document = featherweight::screenplay_to_document(&pdf, &native, &ocr)?;
let fountain = featherweight::screenplay_to_fountain(&pdf, &native, &ocr)?;
let fdx = featherweight::screenplay_to_fdx(&pdf, &native, &ocr)?;
let json = featherweight::screenplay_to_json(&pdf, &native, &ocr)?;
```

Errors: `ScreenplayConversionError` (`.code()` is the TypeScript code:
`INVALID_PAGE_INDEX`, `DUPLICATE_PAGE_INDEX`, `OVERLAPPING_PAGE_INDEX`,
`PDF_PROCESSING_FAILED`, `INVALID_FDX_TEXT`) and `PdfInspectionError`.

Scanned pages: there is no OCR engine built in (nor in the TypeScript port).
To support them, OCR those pages yourself and pass the positioned text to
`create_screenplay_document(&native_pages, &ocr_pages)`, which takes
`PositionedTextPage { page_index, items }` per page and runs the same pipeline.

Conversion is synchronous and CPU-bound (a 130-page feature takes ~250 ms in a
release build, several times longer in a debug build); run it off your UI thread.
`screenplay_document` predates `import_screenplay` and is kept for existing
callers; new code should use `import_screenplay`.

## Command line

```sh
cargo install --path rust/crates/featherweight
featherweight script.pdf                      # Fountain to stdout
featherweight script.pdf --format fdx --output script.fdx
```

## Layout

Modules mirror the TypeScript tree, one file per module:

| TypeScript | Rust |
|---|---|
| `src/pdf/extraction/pdf-inspector.ts`, `src/pdf/inspection.ts` | `extraction.rs` |
| `src/screenplay/*.ts` | `conversion.rs`, `error.rs` |
| `src/cli.ts`, `src/cli/*.ts` | `bin/featherweight.rs` |
| `src/core/*.ts` | `core/*.rs` |
| `src/core/layout/*.ts` | `core/layout/*.rs` (plus `line_set.rs`, the port of `Set<PhysicalTextLine>`) |
| `src/core/recognition/*.ts` | `core/recognition/*.rs` |
| `src/pdf/normalization/*.ts` | `normalization/*.rs` |
| `src/fountain/*.ts`, `src/fdx/*.ts` | `fountain/*.rs`, `fdx/*.rs` |
| (Rust only) | `import.rs`, `core/character_cue.rs`, `typescript.rs` |

Text extraction uses the `pdf-inspector` crate pinned to `=0.1.7`, the build
behind `@firecrawl/pdf-inspector@1.11.2` that the TypeScript port loads through
napi, so extracted items are identical. Bump both together.

Representation differences with no behavioral effect:
- Span `start`/`end` are UTF-8 byte offsets (TypeScript: UTF-16 units).
- JavaScript string semantics (`.length`, `.trim()`, `\s`) go through `core::text`.

Deliberate divergence: an action whose styled runs split `\r\n` across two runs
makes the TypeScript Fountain serializer throw; Rust serializes it instead.

## Changing behavior

Change the TypeScript first, then port the same change here; the harness must
stay green.

```sh
pnpm parity                                  # build TS, then both checks below
node scripts/parity.mjs                      # every fixture PDF, every stage, end to end
node scripts/parity.mjs path/to/script.pdf   # any other PDF
node scripts/parity.mjs --stage layout --verbose
node scripts/parity-replay.mjs               # every stage call the TS test suite makes
cargo test --manifest-path rust/Cargo.toml
cargo test --manifest-path rust/Cargo.toml -p featherweight --all-features   # incl. the .d.ts check
cargo run --manifest-path rust/Cargo.toml -p featherweight --example export_types --features typescript
```

`scripts/parity.mjs` runs each Rust stage on the TypeScript output of the stage
before it (physical lines, normalization, layout, recognition, Fountain, FDX),
so a failure names the stage that diverges, then compares the public API end to
end. `scripts/parity-replay.mjs` runs the TypeScript test suite with every
pipeline stage wrapped (`scripts/parity-capture.ts`), then replays each recorded
call through Rust, so all TypeScript unit and integration cases hold for the
port too.
