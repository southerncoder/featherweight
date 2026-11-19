use std::env;
use std::fs;
fn main() {
    let path = env::args().nth(1).expect("pdf path");
    let bytes = fs::read(&path).expect("read");
    let json = featherweight::screenplay_to_json(&bytes, &(0..5).collect::<Vec<_>>()).unwrap();
    let insp = featherweight::inspect_screenplay_pdf(&bytes).unwrap();
    let json = featherweight::screenplay_to_json(&bytes, &(0..insp.page_count).collect::<Vec<_>>()).unwrap();
    println!("{}", json);
}
