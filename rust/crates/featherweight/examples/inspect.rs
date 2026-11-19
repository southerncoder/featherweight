use std::env; use std::fs;
fn main() {
    let path=env::args().nth(1).expect("pdf");
    let bytes=fs::read(&path).unwrap();
    let insp=featherweight::inspect_screenplay_pdf(&bytes).unwrap();
    println!("pages={} ocr={:?}", insp.page_count, insp.pages_needing_ocr);
    let native: Vec<usize>=(0..insp.page_count).collect();
    let json=featherweight::screenplay_to_json(&bytes,&native).unwrap();
    let v: serde_json::Value=serde_json::from_str(&json).unwrap();
    println!("title_keys={:?}", v["titlePage"].as_array().map(|a| a.iter().filter_map(|f| f["key"].as_str()).collect::<Vec<_>>()));
    let elems=v["elements"].as_array().unwrap();
    println!("elements={}", elems.len());
    let mut c=std::collections::BTreeMap::new();
    for e in elems {
        *c.entry(e["type"].as_str().unwrap()).or_insert(0)+=1;
    }
    println!("counts={:?}", c);
}