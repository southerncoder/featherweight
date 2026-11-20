use std::env; use std::fs;
fn main() {
    let path=env::args().nth(1).expect("pdf");
    let bytes=fs::read(&path).unwrap();
    let insp=featherweight::inspect_screenplay_pdf(&bytes).unwrap();
    let idx:Vec<usize>=(0..insp.page_count).collect();
    let out=featherweight::screenplay_to_fdx(&bytes,&idx).unwrap();
    println!("{}", out);
}
