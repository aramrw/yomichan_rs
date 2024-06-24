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
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Start,
    End,
    Left,
    Right,
    Center,
    Justify,
    JustifyAll,
    MatchParent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeUnits {
    Px,
    Em,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredContentStyle {
    font_style: Option<FontStyle>,
    font_weight: Option<FontWeight>,
    font_size: Option<String>,
    color: Option<String>,
    background: Option<String>,
    background_color: Option<String>,
    text_decoration_line: Option<TextDecorationLineOrNone>,
    text_decoration_style: Option<TextDecorationStyle>,
    text_decoration_color: Option<String>,
    border_color: Option<String>,
    border_style: Option<String>,
    border_radius: Option<String>,
    border_width: Option<String>,
    clip_path: Option<String>,
    vertical_align: Option<VerticalAlign>,
    text_align: Option<TextAlign>,
    text_emphasis: Option<String>,
    text_shadow: Option<String>,
    margin: Option<String>,
    margin_top: Option<String>,
    margin_left: Option<String>,
    margin_right: Option<String>,
    margin_bottom: Option<String>,
    padding: Option<String>,
    padding_top: Option<String>,
    padding_left: Option<String>,
    padding_right: Option<String>,
    padding_bottom: Option<String>,
    word_break: Option<WordBreak>,
    white_space: Option<String>,
    cursor: Option<String>,
    list_style_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Element {
    LineBreak(LineBreak),
    UnstyledElement(UnstyledElement),
    TableElement(TableElement),
    StyledElement(StyledElement),
    ImageElement(ImageElement),
    LinkElement(LinkElement),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
/// This element doesn't support children or support language.
pub struct LineBreak {
    tag: HtmlTag,
    data: Option<HashMap<String, String>>,
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
