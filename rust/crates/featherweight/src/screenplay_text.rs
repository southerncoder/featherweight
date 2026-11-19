pub fn is_uppercase_text(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() { return false; }
    t.chars().all(|c| !c.is_lowercase())
}