//! Writes the TypeScript declarations for the crate's JSON to
//! `rust/bindings/featherweight.d.ts`:
//!
//!   cargo run -p featherweight --example export_types --features typescript
fn main() -> std::io::Result<()> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bindings/featherweight.d.ts");
    std::fs::create_dir_all(path.parent().expect("the path has a parent"))?;
    std::fs::write(&path, featherweight::typescript::declarations())?;
    println!("wrote {}", path.display());
    Ok(())
}
