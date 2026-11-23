//! Splitting a character cue into a name and its extensions. Not in the
//! TypeScript API; a convenience for consumers mapping cues to their own model.
use serde::Serialize;

use crate::core::screenplay_document::Character;
use crate::core::text::{is_js_whitespace, js_trim};

/// A character cue split into its parts, e.g. `MRS. DOE (V.O.) (CONT'D)` into
/// name `MRS. DOE`, extensions `["V.O."]`, and `continued: true`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct CharacterCue {
    /// The name, trimmed, in its original casing. Parenthesized text that is not
    /// a known extension stays part of it (`JOHN (YOUNG)`).
    pub name: String,
    /// Known extensions other than continuation (`V.O.`, `O.S.`, `O.C.`), in
    /// order, as written.
    pub extensions: Vec<String>,
    /// Whether the cue carries a continuation extension (`CONT'D`, `CONT.`,
    /// `CONTINUED`, any case, straight or curly apostrophe).
    pub continued: bool,
}

impl Character {
    /// Splits the cue into its name and trailing parenthesized extensions. See
    /// [`CharacterCue`].
    pub fn split(&self) -> CharacterCue {
        split_character_cue(&self.text.plain_text())
    }
}

enum Extension {
    Continued,
    Other,
}

fn classify_extension(inner: &str) -> Option<Extension> {
    let normalized: String = inner
        .chars()
        .filter(|&c| !is_js_whitespace(c))
        .map(|c| if c == '\u{2019}' { '\'' } else { c })
        .collect::<String>()
        .to_uppercase();

    match normalized.trim_end_matches('.') {
        "CONT'D" | "CONT" | "CONTINUED" => Some(Extension::Continued),
        "V.O" | "O.S" | "O.C" => Some(Extension::Other),
        _ => None,
    }
}

/// Splits cue text; see [`Character::split`].
pub fn split_character_cue(cue: &str) -> CharacterCue {
    let mut rest = js_trim(cue);
    let mut extensions = Vec::new();
    let mut continued = false;

    while let Some(without_close) = rest.strip_suffix(')') {
        let Some(open) = without_close.rfind('(') else {
            break;
        };
        let inner = &without_close[open + 1..];
        if inner.contains(')') {
            break;
        }
        match classify_extension(inner) {
            Some(Extension::Continued) => continued = true,
            Some(Extension::Other) => extensions.push(js_trim(inner).to_owned()),
            None => break,
        }
        rest = js_trim(&without_close[..open]);
    }

    extensions.reverse();
    CharacterCue {
        name: rest.to_owned(),
        extensions,
        continued,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(name: &str, extensions: &[&str], continued: bool) -> CharacterCue {
        CharacterCue {
            name: name.to_owned(),
            extensions: extensions.iter().map(|extension| (*extension).to_owned()).collect(),
            continued,
        }
    }

    #[test]
    fn splits_names_and_extensions() {
        assert_eq!(split_character_cue("BEN"), cue("BEN", &[], false));
        assert_eq!(split_character_cue("BEN (V.O.)"), cue("BEN", &["V.O."], false));
        assert_eq!(split_character_cue("BEN (O.S.) (CONT'D)"), cue("BEN", &["O.S."], true));
        assert_eq!(split_character_cue("BEN (CONT\u{2019}D)"), cue("BEN", &[], true));
        assert_eq!(
            split_character_cue("MRS. DOE (V.O.)"),
            cue("MRS. DOE", &["V.O."], false)
        );
        assert_eq!(
            split_character_cue("  BEN  ( o.c. ) (cont.)  "),
            cue("BEN", &["o.c."], true)
        );
        assert_eq!(split_character_cue("ANNA (CONTINUED)"), cue("ANNA", &[], true));
        assert_eq!(split_character_cue(""), cue("", &[], false));
    }

    #[test]
    fn keeps_unknown_parentheses_in_the_name() {
        assert_eq!(split_character_cue("JOHN (YOUNG)"), cue("JOHN (YOUNG)", &[], false));
        assert_eq!(
            split_character_cue("JOHN (YOUNG) (V.O.)"),
            cue("JOHN (YOUNG)", &["V.O."], false)
        );
        assert_eq!(split_character_cue("(V.O.)"), cue("", &["V.O."], false));
    }

    #[test]
    fn splits_a_character_element() {
        let character: Character = serde_json::from_value(serde_json::json!({ "type": "character", "text": { "runs": [
                { "text": "BEN ", "styles": [] }, { "text": "(V.O.)", "styles": ["bold"] }
            ] } }))
        .unwrap();
        assert_eq!(character.split(), cue("BEN", &["V.O."], false));
    }
}
