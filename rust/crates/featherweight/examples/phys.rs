use std::env; use std::fs;
fn main() {
    let path=env::args().nth(1).expect("pdf");
    let bytes=fs::read(&path).unwrap();
    let insp=featherweight::inspect_screenplay_pdf(&bytes).unwrap();
    let idx:Vec<usize>=(0..insp.page_count).collect();
    let json=featherweight::physical_text_json(&bytes,&idx).unwrap();
    println!("{}",json);
}