//! Port of `src/core/screenplay-document.ts`.
//!
//! Field order matters: it is the JSON key order `screenplay_to_json` emits, and
//! it matches the TypeScript object literals.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDocument {
    pub title_page: Vec<TitlePageField>,
    pub elements: Vec<ScreenplayElement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TitlePageField {
    pub key: String,
    pub values: Vec<StyledText>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledText {
    pub runs: Vec<TextRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRun {
    pub text: String,
    pub styles: Vec<TextStyle>,
}

/// Declaration order is the canonical style order (`styleOrder` in `styled-text.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextStyle {
    Bold,
    Italic,
    Underline,
    Strikeout,
}

impl TextStyle {
    pub const ORDER: [TextStyle; 4] = [Self::Bold, Self::Italic, Self::Underline, Self::Strikeout];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionAlignment {
    Standard,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ScreenplayElement {
    SceneHeading {
        text: StyledText,
        #[serde(rename = "sceneNumber")]
        scene_number: Option<String>,
    },
    Action {
        text: StyledText,
        alignment: ActionAlignment,
    },
    Character {
        text: StyledText,
    },
    Parenthetical {
        text: StyledText,
    },
    Dialogue {
        text: StyledText,
    },
    DualDialogue {
        left: DialogueSequence,
        right: DialogueSequence,
    },
    Lyric {
        text: StyledText,
    },
    Transition {
        text: StyledText,
    },
    PageBreak,
}

/// The `character` of a [`DialogueSequence`]: always a `{"type":"character"}` element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "character")]
pub struct Character {
    pub text: StyledText,
}

/// `Parenthetical | Dialogue`, the content of a [`DialogueSequence`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DialogueContent {
    Parenthetical { text: StyledText },
    Dialogue { text: StyledText },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueSequence {
    pub character: Character,
    pub content: Vec<DialogueContent>,
}
