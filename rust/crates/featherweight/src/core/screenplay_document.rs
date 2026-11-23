//! Port of `src/core/screenplay-document.ts`.
//!
//! Field order matters: it is the JSON key order `screenplay_to_json` emits, and
//! it matches the TypeScript object literals.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ScreenplayDocument {
    pub title_page: Vec<TitlePageField>,
    pub elements: Vec<ScreenplayElement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TitlePageField {
    pub key: String,
    pub values: Vec<StyledText>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct StyledText {
    pub runs: Vec<TextRun>,
}

impl StyledText {
    /// The text without styles: every run's text, concatenated.
    pub fn plain_text(&self) -> String {
        self.runs.iter().map(|run| run.text.as_str()).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct TextRun {
    pub text: String,
    pub styles: Vec<TextStyle>,
}

/// Declaration order is the canonical style order (`styleOrder` in `styled-text.ts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum ActionAlignment {
    Standard,
    Center,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ScreenplayElement {
    SceneHeading {
        text: StyledText,
        #[serde(rename = "sceneNumber")]
        scene_number: Option<String>,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    Action {
        text: StyledText,
        alignment: ActionAlignment,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    Character {
        text: StyledText,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    Parenthetical {
        text: StyledText,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    Dialogue {
        text: StyledText,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    DualDialogue {
        left: DialogueSequence,
        right: DialogueSequence,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    Lyric {
        text: StyledText,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    Transition {
        text: StyledText,
        /// 0-based source page. Set only by `import_screenplay`; omitted from
        /// JSON when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "typescript", ts(optional))]
        page: Option<usize>,
    },
    PageBreak,
}

/// The `character` of a [`DialogueSequence`]: always a `{"type":"character"}` element.
// TypeScript: `crate::typescript::CharacterJson` (a derive would name the type
// after the serde tag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename = "character")]
pub struct Character {
    pub text: StyledText,
}

/// `Parenthetical | Dialogue`, the content of a [`DialogueSequence`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DialogueContent {
    Parenthetical { text: StyledText },
    Dialogue { text: StyledText },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
pub struct DialogueSequence {
    #[cfg_attr(feature = "typescript", ts(as = "crate::typescript::CharacterJson"))]
    pub character: Character,
    pub content: Vec<DialogueContent>,
}
