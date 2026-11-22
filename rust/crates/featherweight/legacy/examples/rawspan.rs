use std::env; use std::fs;
fn main() {
    let path=env::args().nth(1).expect("pdf");
    let bytes=fs::read(&path).unwrap();
    let mut doc=pdf_oxide::PdfDocument::from_bytes(bytes).unwrap();
    let pc=doc.page_count().unwrap();
    println!("pages {}",pc);
    for p in 0..pc {
        let spans=doc.extract_spans(p).unwrap();
        let n=spans.iter().filter(|s| s.text.contains("SNIPER")).count();
        if n>0 { println!("page {} count {}",p,n); }
    }
}
