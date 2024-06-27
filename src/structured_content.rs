use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub enum ContentMatchType {
    String(String),
    Element(Box<Element>),
    Content(Vec<ContentMatchType>),
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
pub struct UnstyledElement {
    /// `UnstyledElements`' tags are:
    /// `Ruby`, `RubyTag` `RubyParenthesis`, `Table`, `TableHeader`, `TableBody`, `TableFooter`, `TableRow`.
    tag: HtmlTag,
    content: Option<ContentMatchType>,
    data: Option<HashMap<String, String>>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableElement {
    /// `TableElement`'s tags are:
    /// `TableData`, `TableHeader` .
    tag: HtmlTag,
    content: Option<ContentMatchType>,
    data: Option<HashMap<String, String>>,
    col_span: u16,
    row_span: u16,
    style: Option<StructuredContentStyle>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledElement {
    /// `StyledElement`'s tags are:
    /// `Span`, `Div`, `OrderedList`, `UnorderedList`, `ListItem`, `Details`, `Summary`.
    tag: HtmlTag,
    content: Option<ContentMatchType>,
    data: Option<HashMap<String, String>>,
    style: Option<StructuredContentStyle>,
    /// Hover text for the element.
    title: Option<String>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkElement {
    /// `LinkElement`'s tags are:
    /// `Anchor`.
    tag: HtmlTag,
    content: Option<ContentMatchType>,
    /// The URL for the link.
    /// URLs starting with a `?` are treated as internal links to other dictionary content.
    href: String,
    /// Defines the language of an element in the format defined by RFC 5646.
    ///yomichan_rs will **only** ever support `ja` & `ja-JP`.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageElementBase {
    data: Option<HashMap<String, String>>,
    /// Path to the image file in the archive.
    path: String,
    /// Preferred width of the image.
    width: Option<u16>,
    /// Preferred height of the image.
    height: Option<u16>,
    /// Preferred width of the image.
    /// This is only used in the internal database.
    preferred_width: Option<u16>,
    /// Preferred height of the image.
    /// This is only used in the internal database.
    preferred_height: Option<u16>,
    /// Hover text for the image.
    title: Option<String>,
    /// Alt text for the image.
    alt: Option<String>,
    /// Description of the image.
    description: Option<String>,
    /// Whether or not the image should appear pixelated at sizes larger than the image's native resolution.
    pixelated: Option<bool>,
    /// Controls how the image is rendered. The value of this field supersedes the pixelated field.
    image_rendering: Option<ImageRendering>,
    /// Controls the appearance of the image. The 'monochrome' value will mask the opaque parts of the image using the current text color.
    appearance: Option<ImageAppearance>,
    /// Whether or not a background color is displayed behind the image.
    background: Option<bool>,
    /// Whether or not the image is collapsed by default.
    collapsed: Option<bool>,
    /// Whether or not the image can be collapsed.
    collapsible: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageElement {
    /// The base properties of the image element.
    base: ImageElementBase,
    /// This element doesn't support children.
    content: Option<()>,
    /// The vertical alignment of the image.
    vertical_align: Option<VerticalAlign>,
    /// Shorthand for border width, style, and color.
    border: Option<String>,
    /// Roundness of the corners of the image's outer border edge.
    border_radius: Option<String>,
    /// The units for the width and height.
    size_units: Option<SizeUnits>,
}
