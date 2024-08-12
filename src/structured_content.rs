use serde::{de, Deserialize, Deserializer, Serialize};
use serde_untagged::UntaggedEnumVisitor;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HtmlTag {
    #[serde(rename = "ruby")]
    Ruby,
    #[serde(rename = "rt")]
    RubyText,
    #[serde(rename = "rp")]
    RubyParenthesis,
    Table,
    #[serde(rename = "td")]
    TableData,
    #[serde(rename = "th")]
    TableHeader,
    #[serde(rename = "tb")]
    TableBody,
    #[serde(rename = "tf")]
    TableFooter,
    #[serde(rename = "tr")]
    TableRow,
    #[serde(rename = "a")]
    Anchor,
    Span,
    Div,
    #[serde(rename = "ol")]
    OrderedList,
    #[serde(rename = "ul")]
    UnorderedList,
    #[serde(rename = "li")]
    ListItem,
    Details,
    Summary,
    #[serde(rename = "br")]
    Break,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TextDecorationLineOrNone {
    None,
    TextDecorationLine(TextDecorationLine),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Normal,
    Bold,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SizeUnits {
    Px,
    Em,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

/// A match type to deserialize any `Content` type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentMatchType {
    String(String),
    /// A single html element.
    /// See: [`HtmlTag`].
    ///
    /// Most likely a [`HtmlTag::Anchor`] element.
    /// If so, the definition contains a reference to another entry.
    Element(Box<Element>),
    /// An array of html elements.
    /// See: [`HtmlTag`].
    ///
    Content(Vec<Element>),
}

// daijisen: ~6.35s WITHOUT custom deserialization.
// daijisen: ~7.13 WITH custom deserialization.

// impl<'de> Deserialize<'de> for ContentMatchType {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         UntaggedEnumVisitor::new()
//             .string(|single| Ok(ContentMatchType::String(single.to_string())))
//             .map(|map| map.deserialize().map(ContentMatchType::Element))
//             .seq(|seq| seq.deserialize().map(ContentMatchType::Content))
//             .deserialize(deserializer)
//     }
// }

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_untagged::UntaggedEnumVisitor::new()
            .string(|unkown_string| Ok(Element::UnknownString(unkown_string.to_string())))
            .map(|map| {
                let value = map.deserialize::<serde_json::Value>()?;
                let tag = match value.get("tag") {
                    Some(tag) => tag
                        .as_str()
                        .ok_or_else(|| serde::de::Error::custom("tag is not a string")),
                    None => Err(serde::de::Error::custom("missing tag")),
                }?;

                let element = match tag {
                    "a" => serde_json::from_value(value).map(Element::Link),
                    "div" => serde_json::from_value(value).map(Element::Styled),
                    "span" => serde_json::from_value(value).map(Element::Styled),
                    "br" => serde_json::from_value(value).map(Element::LineBreak),
                    "img" => serde_json::from_value(value).map(Element::Image),
                    "ruby" => serde_json::from_value(value).map(Element::Unstyled),
                    "rt" => serde_json::from_value(value).map(Element::Unstyled),
                    "rp" => serde_json::from_value(value).map(Element::Unstyled),
                    "t" => serde_json::from_value(value).map(Element::Unstyled),
                    "th" => serde_json::from_value(value).map(Element::Unstyled),
                    "tb" => serde_json::from_value(value).map(Element::Unstyled),
                    "tf" => serde_json::from_value(value).map(Element::Unstyled),
                    "ol" => serde_json::from_value(value).map(Element::Styled),
                    "ul" => serde_json::from_value(value).map(Element::Styled),
                    "li" => serde_json::from_value(value).map(Element::Styled),
                    "details" => serde_json::from_value(value).map(Element::Styled),
                    "summary" => serde_json::from_value(value).map(Element::Styled),
                    "table" => serde_json::from_value(value).map(Element::Unstyled),
                    "thead" => serde_json::from_value(value).map(Element::Unstyled),
                    "tbody" => serde_json::from_value(value).map(Element::Unstyled),
                    "tfoot" => serde_json::from_value(value).map(Element::Unstyled),
                    "tr" => serde_json::from_value(value).map(Element::Unstyled),
                    "td" => serde_json::from_value(value).map(Element::Table),
                    "th" => serde_json::from_value(value).map(Element::Table),
                    _ => {
                        if let serde_json::Value::String(s) = value {
                            Ok(Element::UnknownString(s.to_string()))
                        } else {
                            Err(serde::de::Error::custom(format!(
                                "unexpected value: {tag};"
                            )))
                        }
                    }
                };

                element.map_err(|err| {
                    serde::de::Error::custom(format!("failed to deserialize element: {}", err))
                })
            })
            .deserialize(deserializer)
    }
}

/// Represents All `Content` elements that can
/// appear within a `"content":` section.
#[derive(Clone, Debug, PartialEq, Serialize)]
//#[serde(tag = "tag")]
#[serde(untagged)]
pub enum Element {
    UnknownString(String),
    //#[serde(rename = "a")]
    Link(LinkElement),
    // #[serde(
    //     alias = "div",
    //     alias = "span",
    //     alias = "ol",
    //     alias = "ul",
    //     alias = "li",
    //     alias = "details",
    //     alias = "summary",
    //     alias = "th",
    //     alias = "td"
    // )]
    Styled(StyledElement),
    //     alias = "rt",
    //     alias = "rp",
    //     alias = "t",
    //     alias = "tb",
    //     alias = "tf",
    //     alias = "tr"
    // )]
    Unstyled(UnstyledElement),
    //#[serde(alias = "td", alias = "th")]
    Table(TableElement),
    //#[serde(rename = "br")]
    LineBreak(LineBreak),
    //#[serde(rename = "img")]
    Image(ImageElement),
}

/// This element doesn't support children or support language.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineBreak {
    /// The `LineBreak`' tag is:
    ///
    /// [`HtmlTag::Break`] | `"br"`.
    tag: HtmlTag,
    data: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnstyledElement {
    /// `UnstyledElements`'s' tags could be the following:
    ///
    /// [`HtmlTag::Ruby`],
    /// [`HtmlTag::RubyText`],
    /// [`HtmlTag::RubyParenthesis`],
    /// [`HtmlTag::Table`],
    /// [`HtmlTag::TableHeader`],
    /// [`HtmlTag::TableBody`],
    /// [`HtmlTag::TableFooter`],
    /// [`HtmlTag::TableRow`].
    pub tag: HtmlTag,
    pub content: Option<ContentMatchType>,
    pub data: Option<HashMap<String, String>>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableElement {
    /// `TableElement`'s tags could be the following:
    ///
    /// [`HtmlTag::TableData`],
    /// [`HtmlTag::TableHeader`].
    pub tag: HtmlTag,
    pub content: Option<ContentMatchType>,
    pub data: Option<HashMap<String, String>>,
    pub col_span: Option<u16>,
    pub row_span: Option<u16>,
    pub style: Option<StructuredContentStyle>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyledElement {
    /// `StyledElement`'s tags are:
    ///
    /// [`HtmlTag::Span`],
    /// [`HtmlTag::Div`],
    /// [`HtmlTag::OrderedList`],
    /// [`HtmlTag::UnorderedList`],
    /// [`HtmlTag::ListItem`],
    /// [`HtmlTag::Details`],
    /// [`HtmlTag::Summary`].
    pub tag: HtmlTag,
    pub content: Option<ContentMatchType>,
    pub data: Option<HashMap<String, String>>,
    pub style: Option<StructuredContentStyle>,
    /// Hover text for the element.
    pub title: Option<String>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinkElement {
    /// The `LinkElement`'s tag is:
    ///
    /// [`HtmlTag::Anchor`] | `"a"`.
    pub tag: HtmlTag,
    pub content: Option<ContentMatchType>,
    /// The URL for the link.
    ///
    /// URLs starting with a `?` are treated as internal links to other dictionary content.
    pub href: String,
    /// Defines the language of an element in the format defined by RFC 5646.
    ///
    ///yomichan_rs will currently only support `ja` & `ja-JP`.
    pub lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageElement {
    /// This element doesn't support children.
    pub content: Option<()>,
    /// The vertical alignment of the image.
    pub vertical_align: Option<VerticalAlign>,
    /// Shorthand for border width, style, and color.
    pub border: Option<String>,
    /// Roundness of the corners of the image's outer border edge.
    pub border_radius: Option<String>,
    /// The units for the width and height.
    pub size_units: Option<SizeUnits>,
    pub data: Option<HashMap<String, String>>,
    /// Path to the image file in the archive.
    pub path: String,
    /// Preferred width of the image.
    pub width: Option<f32>,
    /// Preferred height of the image.
    pub height: Option<f32>,
    /// Preferred width of the image.
    /// This is only used in the internal database.
    pub preferred_width: Option<f32>,
    /// Preferred height of the image.
    /// This is only used in the internal database.
    pub preferred_height: Option<f32>,
    /// Hover text for the image.
    pub title: Option<String>,
    /// Alt text for the image.
    pub alt: Option<String>,
    /// Description of the image.
    pub description: Option<String>,
    /// Whether or not the image should appear pixelated at sizes larger than the image's native resolution.
    pub pixelated: Option<bool>,
    /// Controls how the image is rendered. The value of this field supersedes the pixelated field.
    pub image_rendering: Option<ImageRendering>,
    /// Controls the appearance of the image. The 'monochrome' value will mask the opaque parts of the image using the current text color.
    appearance: Option<ImageAppearance>,
    /// Whether or not a background color is displayed behind the image.
    background: Option<bool>,
    /// Whether or not the image is collapsed by default.
    collapsed: Option<bool>,
    /// Whether or not the image can be collapsed.
    collapsible: Option<bool>,
}
