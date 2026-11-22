//! Dump raw pdf-inspector items as JSON (same fields as the npm binding) for parity checks.
fn main() {
    let path = std::env::args().nth(1).expect("pdf path");
    let bytes = std::fs::read(path).unwrap();
    let items = pdf_inspector::extract_text_with_positions_mem(&bytes).unwrap();
    let out: Vec<_> = items.iter().map(|i| serde_json::json!({
        "text": i.text, "x": i.x, "y": i.y, "width": i.width, "height": i.height,
        "font": i.font, "fontSize": i.font_size, "page": i.page,
        "isBold": i.is_bold, "isItalic": i.is_italic, "isUnderline": i.is_underline,
        "isStrikeout": i.is_strikeout, "itemType": format!("{:?}", i.item_type),
    })).collect();
    println!("{}", serde_json::to_string(&out).unwrap());
}
