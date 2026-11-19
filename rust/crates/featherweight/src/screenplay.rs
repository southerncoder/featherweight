use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StyledRun {
    pub text: String,
    pub styles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StyledText {
    pub runs: Vec<StyledRun>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ScreenplayElement {
    #[serde(rename = "scene-heading")]
    SceneHeading { text: StyledText, #[serde(rename = "sceneNumber")] scene_number: Option<String> },
    #[serde(rename = "action")]
    Action { text: StyledText, alignment: String },
    #[serde(rename = "character")]
    Character { text: StyledText },
    #[serde(rename = "dialogue")]
    Dialogue { text: StyledText },
    #[serde(rename = "parenthetical")]
    Parenthetical { text: StyledText },
    #[serde(rename = "transition")]
    Transition { text: StyledText },
    #[serde(rename = "lyrics")]
    Lyrics { text: StyledText },
    #[serde(rename = "dual-dialogue")]
    DualDialogue { left: DialogueSequence, right: DialogueSequence },
    #[serde(rename = "page-break")]
    PageBreak {},
}

#[derive(Debug, Clone, Serialize)]
pub struct DialogueSequence {
    pub character: Box<ScreenplayElement>,
    pub content: Vec<ScreenplayElement>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TitleField {
    pub key: String,
    pub values: Vec<StyledText>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenplayDocument {
    #[serde(rename = "titlePage")]
    pub title_page: Vec<TitleField>,
    pub elements: Vec<ScreenplayElement>,
}