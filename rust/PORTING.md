# Rust Porting Status

Current milestone: extraction + physical text lines.

Implemented:
- PDF inspection (`inspect_screenplay_pdf`)
- Positioned text extraction via pdf_oxide with geometry + font styling
- Physical line grouping (`model::group_into_physical_lines`)
- JSON document types: `screenplay::ScreenplayDocument`, elements, title fields, styled runs

Not yet ported:
- normalized text (`src/pdf/normalization/physical-text.ts`)
- layout inference (`src/core/layout/*`)
- recognition modules: title page, scene headings, action collection, dialogue occurrences, dual dialogue, sequences, transitions, centered action
- styled text conversion and sentence spacing
- title-page extraction helpers
- Fountain serializer
- FDX serializer
- JS-style JSON output (`screenplayToJSON`) parity
- OCR routing validation and error codes
- CLI (Rust)
- tests/parity fixtures

Approach:
Port modules in dependency order (types -> extraction -> physical lines -> normalized text -> layout -> recognition -> serializers), with parity tests comparing JSON output to TypeScript CLI for `test/brick-and-steel*.pdf`. Each ported module should remain on the shared cargo cache (`D:\cargo`, `D:\rustup`) and avoid redundant filesystem crawls.