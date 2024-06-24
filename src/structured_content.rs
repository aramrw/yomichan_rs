use serde::{Serialize, Deserialize};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HtmlTag {
    Ruby,
    RubyText,
    RubyParenthesis,
    Table,
    TableData,
    TableHeader,
    TableBody,
    TableFooter,
    TableRow,
    Anchor,
    Span,
    Div,
    OrderedList,
    UnorderedList,
    ListItem,
    Details,
    Summary,
    Break,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    String(String),
    Element(Box<Element>),
    Content(Vec<Content>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerticalAlign {
    Baseline,
    Sub,
    Super,
    TextTop,
    TextBottom,
    Middle,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDecorationLineOrNone {
    None,
    TextDecorationLine(TextDecorationLine),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageElementBase {
    path: String,
    width: u16,
    height: u16,
    preferred_width: u16,
    preferred_height: u16,
    title: String,
    alt: String,
    description: String,
    pixelated: bool,
    image_rendering: ImageRendering,
    appearance: ImageAppearance,
    background: bool,
    collapsed: bool,
    collapsible: bool,
}
