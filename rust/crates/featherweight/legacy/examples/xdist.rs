use std::env; use std::fs;
fn main() {
    let path=env::args().nth(1).expect("pdf");
    let bytes=fs::read(&path).unwrap();
    let insp=featherweight::inspect_screenplay_pdf(&bytes).unwrap();
    let idx:Vec<usize>=(0..insp.page_count).collect();
    let json=featherweight::physical_text_json(&bytes,&idx).unwrap();
    let v: serde_json::Value=serde_json::from_str(&json).unwrap();
    let mut c=std::collections::BTreeMap::new();
    for l in v["lines"].as_array().unwrap() {
        *c.entry(l["x"].as_f64().unwrap().round() as i64).or_insert(0)+=1;
    }
    for (x,n) in c.iter() {
        println!("x={} n={}",x,n);
    }
}