# Featherweight (Rust)

Turn a screenplay PDF into Fountain, FDX, or structured JSON. This crate is a
behavioral port of the TypeScript library in `../src`: for the same PDF and page
routes it produces byte-identical JSON, Fountain, and FDX, and its CLI matches
the TypeScript CLI's output, messages, and exit codes.

Requires Rust 1.88+. Pure Rust, no system libraries.

## In an editor

```toml
[dependencies]
featherweight = { path = "path/to/featherweight/rust/crates/featherweight" }
```

```rust
use featherweight::{ScreenplayElement, screenplay_document};

let pdf = std::fs::read("script.pdf")?;
let document = screenplay_document(&pdf)?; // inspects and routes pages for you

for field in &document.title_page { /* field.key, field.values */ }
for element in &document.elements {
    match element {
        ScreenplayElement::SceneHeading { text, scene_number } => {}
        ScreenplayElement::Action { text, alignment } => {}
        ScreenplayElement::Character { text } => {}
        ScreenplayElement::Parenthetical { text } | ScreenplayElement::Dialogue { text } => {}
        ScreenplayElement::DualDialogue { left, right } => {}
        ScreenplayElement::Lyric { text } | ScreenplayElement::Transition { text } => {}
        ScreenplayElement::PageBreak => {}
    }
}
```

Text is `StyledText { runs: Vec<TextRun { text, styles }> }`, with styles from
`Bold`, `Italic`, `Underline`, `Strikeout`. Every document type implements serde
`Serialize`/`Deserialize` in the TypeScript JSON shape.

Explicit page routing and serializations, as in the TypeScript API:

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
release build); run it off your UI thread.

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
```

`scripts/parity.mjs` runs each Rust stage on the TypeScript output of the stage
before it (physical lines, normalization, layout, recognition, Fountain, FDX),
so a failure names the stage that diverges, then compares the public API end to
end. `scripts/parity-replay.mjs` runs the TypeScript test suite with every
pipeline stage wrapped (`scripts/parity-capture.ts`), then replays each recorded
call through Rust, so all TypeScript unit and integration cases hold for the
port too.
