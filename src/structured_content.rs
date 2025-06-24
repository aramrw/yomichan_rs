use std::{fmt, fs::File, hash::Hash, io::BufReader};

use indexmap::IndexMap;
use serde::{
    de::{self, Error, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_untagged::UntaggedEnumVisitor;
use serde_with::skip_serializing_none;

use crate::{database::dictionary_database::DatabaseTermEntry, test_utils};

/// The object holding all html & information about an entry.
/// _There is only 1 per entry_.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredContent {
    /// Identifier to mark the start of each entry's content.
    ///
    /// This should _always_ be `"type": "structured-content"` in the file.
    /// If not, the dictionary is not valid.
    #[serde(rename = "type")]
    pub content_type: String,
    /// Contains the main content of the entry.
    /// _(see: [`ContentMatchType`] )_.
    ///
    /// Will _always_ be either an `Element (obj)` or a `Content (array)` _(ie: Never a String)_.
    pub content: ContentMatchType,
}

/// A match type to deserialize any `Content` type.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ContentMatchType {
    /// A single html element.
    /// See: [`HtmlTag`].
    ///
    /// Most likely a [`HtmlTag::Anchor`] element.
    /// If so, the definition contains a reference to another entry.
    Element(Box<Element>),
    /// An array of html elements.
    /// See: [`HtmlTag`].
    ///
    Content(Vec<ContentMatchType>),
    String(String),
}

impl<'de> Deserialize<'de> for ContentMatchType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First, deserialize the data into a generic, abstract value.
        let value = serde_json::Value::deserialize(deserializer)?;

        // --- Check the shape of the value to guide deserialization ---

        // Check 1: Is it a JSON array?
        // This is the most likely candidate for the `Content` variant.
        if value.is_array() {
            // Attempt to deserialize the array into Vec<ContentMatchType>.
            // This will recursively call this same function for each item in the array.
            match serde_json::from_value::<Vec<ContentMatchType>>(value) {
                Ok(content_vec) => return Ok(ContentMatchType::Content(content_vec)),
                Err(e) => {
                    // This could happen if the array contains something that doesn't fit ContentMatchType.
                    return Err(de::Error::custom(format!(
                        "failed to parse array as Content: {e}",
                    )));
                }
            }
        }

        // Check 2: Is it a JSON object?
        // This is the most likely candidate for the `Element` variant.
        if value.is_object() {
            // Attempt to deserialize the object into an Element.
            match serde_json::from_value::<Element>(value) {
                Ok(element) => return Ok(ContentMatchType::Element(Box::new(element))),
                Err(e) => {
                    return Err(de::Error::custom(format!(
                        "failed to parse object as Element: {e}",
                    )));
                }
            }
        }

        // Check 3: Is it a JSON string?
        // This is the most likely candidate for the `String` variant.
        if value.is_string() {
            // Attempt to deserialize the value as a String.
            match serde_json::from_value::<String>(value) {
                Ok(s) => return Ok(ContentMatchType::String(s)),
                Err(e) => {
                    // This is unlikely to fail if is_string() is true, but we handle it for correctness.
                    return Err(de::Error::custom(format!(
                        "failed to parse string as String: {e}"
                    )));
                }
            }
        }

        // If the value is not an array, object, or string, it's an unsupported type.
        Err(de::Error::custom(
            "data is not a valid ContentMatchType: expected an array, object, or string",
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TermGlossary {
    Content(TermGlossaryContent),
    //Content(TermGlossaryContent),
    /// This is a tuple struct in js.
    /// If you see an `Array.isArray()` check on a [TermGlossary], its looking for this.
    Deinflection(TermGlossaryDeinflection),
}

impl<'de> Deserialize<'de> for TermGlossary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First, deserialize the data into a generic, abstract value.
        let value = serde_json::Value::deserialize(deserializer)?;
        let val_clone = value.clone();

        // --- Attempt to match each variant ---

        // Attempt 2: Try to parse it as TermGlossaryContent (which can be an object or a string).
        // This will consume the value on the final attempt.
        if let Ok(content) = serde_json::from_value::<TermGlossaryContent>(value.clone()) {
            // If successful, we have our answer.
            return Ok(TermGlossary::Content(content));
        }

        // Attempt 1: Try to parse it as a Deinflection array.
        // This is a good first candidate because its shape (an array) is very specific.
        if let Ok(deinflection) = serde_json::from_value::<TermGlossaryDeinflection>(value) {
            // If successful, we have our answer.
            return Ok(TermGlossary::Deinflection(deinflection));
        }

        // If none of the above attempts worked, the data is in an unknown format.
        Err(de::Error::custom(
            format!(" {val_clone} did not match TermGlossary variants: expected deinflection array or content object/string"),
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermGlossaryDeinflection(pub String, pub Vec<String>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermGlossaryContent {
    Tagged(TaggedContent),
    String(String),
}

// impl<'de> Deserialize<'de> for TermGlossaryContent {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         // 1. Deserialize into a generic Value first.
//         let value = serde_json::Value::deserialize(deserializer)?;
//
//         // 2. Try to deserialize the Value as a TaggedContent object.
//         // We clone the value because from_value consumes it, and we might need it again.
//         if let Ok(tagged) = serde_json::from_value::<TaggedContent>(value.clone()) {
//             return Ok(TermGlossaryContent::Tagged(tagged));
//         }
//
//         // 3. If that failed, try to deserialize it as a String.
//         if let Ok(s) = serde_json::from_value::<String>(value) {
//             return Ok(TermGlossaryContent::String(s));
//         }
//
//         // 4. If all attempts fail, return a custom error.
//         Err(de::Error::custom(
//             "data did not match any variant of TermGlossaryContent",
//         ))
//     }
// }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaggedContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "img")]
    Image(Box<ImageElement>),
    #[serde(rename = "structured-content")]
    StructuredContent {
        // The payload is the value of the "content" field.
        #[serde(rename = "content")]
        content: ContentMatchType,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TermGlossaryText {
    pub text: String,
}

/// The 'header', and `structured-content`
/// of a `term_bank_${i}.json` entry item.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TermEntryItem {
    pub expression: String,
    pub reading: String,
    pub def_tags: Option<String>,
    pub rules: String,
    pub score: i128,
    pub structured_content: Vec<TermGlossary>,
    pub sequence: i128,
    pub term_tags: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
// This is now a tuple struct to match the JSON array `[...]`
pub struct TermEntryItemTuple(
    pub String,            // Corresponds to "expression"
    pub String,            // Corresponds to "reading"
    pub String,            // Corresponds to "definition_tags" (was def_tags)
    pub String,            // Corresponds to "rules"
    pub i128,              // Corresponds to "score"
    pub Vec<TermGlossary>, // Corresponds to "glossary" (was structured_content)
    pub i128,              // Corresponds to "sequence"
    pub String,            // Corresponds to "term_tags"
);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextDecorationLine {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SizeUnits {
    Px,
    Em,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
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
    margin: Option<NumberOrString>,
    margin_top: Option<NumberOrString>,
    margin_left: Option<NumberOrString>,
    margin_right: Option<NumberOrString>,
    margin_bottom: Option<NumberOrString>,
    padding: Option<NumberOrString>,
    padding_top: Option<NumberOrString>,
    padding_left: Option<NumberOrString>,
    padding_right: Option<NumberOrString>,
    padding_bottom: Option<NumberOrString>,
    word_break: Option<WordBreak>,
    white_space: Option<String>,
    cursor: Option<String>,
    list_style_type: Option<String>,
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

// impl<'de> Deserialize<'de> for Element {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         serde_untagged::UntaggedEnumVisitor::new()
//             .string(|unkown_string| Ok(Element::UnknownString(unkown_string.to_string())))
//             .map(|map| {
//                 let value = map.deserialize::<serde_json::Value>()?;
//                 let tag = match value.get("tag") {
//                     Some(tag) => tag
//                         .as_str()
//                         .ok_or_else(|| serde::de::Error::custom("tag is not a string")),
//                     None => Err(serde::de::Error::custom("missing tag")),
//                 }?;
//
//                 let element = match tag {
//                     "a" => serde_json::from_value(value).map(Element::Link),
//                     "div" => serde_json::from_value(value).map(Element::Styled),
//                     "span" => serde_json::from_value(value).map(Element::Styled),
//                     "br" => serde_json::from_value(value).map(Element::LineBreak),
//                     "img" => serde_json::from_value(value).map(Element::Image),
//                     "ruby" => serde_json::from_value(value).map(Element::Unstyled),
//                     "rt" => serde_json::from_value(value).map(Element::Unstyled),
//                     "rp" => serde_json::from_value(value).map(Element::Unstyled),
//                     "t" => serde_json::from_value(value).map(Element::Unstyled),
//                     //"th" => serde_json::from_value(value).map(Element::Unstyled),
//                     "tb" => serde_json::from_value(value).map(Element::Unstyled),
//                     "tf" => serde_json::from_value(value).map(Element::Unstyled),
//                     "ol" => serde_json::from_value(value).map(Element::Styled),
//                     "ul" => serde_json::from_value(value).map(Element::Styled),
//                     "li" => serde_json::from_value(value).map(Element::Styled),
//                     "details" => serde_json::from_value(value).map(Element::Styled),
//                     "summary" => serde_json::from_value(value).map(Element::Styled),
//                     "table" => serde_json::from_value(value).map(Element::Unstyled),
//                     "thead" => serde_json::from_value(value).map(Element::Unstyled),
//                     "tbody" => serde_json::from_value(value).map(Element::Unstyled),
//                     "tfoot" => serde_json::from_value(value).map(Element::Unstyled),
//                     "tr" => serde_json::from_value(value).map(Element::Unstyled),
//                     "td" => serde_json::from_value(value).map(Element::Table),
//                     "th" => serde_json::from_value(value).map(Element::Table),
//                     _ => {
//                         if let serde_json::Value::String(s) = value {
//                             Ok(Element::UnknownString(s.to_string()))
//                         } else {
//                             Err(serde::de::Error::custom(format!(
//                                 "unexpected value: {tag};"
//                             )))
//                         }
//                     }
//                 };
//
//                 element.map_err(|err| {
//                     serde::de::Error::custom(format!("failed to deserialize element: {err}"))
//                 })
//             })
//             .deserialize(deserializer)
//     }
// }

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Step 1: Deserialize into a generic Value. This is a single,
        // fast operation provided by serde_json.
        let value = serde_json::Value::deserialize(deserializer)?;
        let val_clone = value.clone();

        // Step 2: Get the tag.
        let tag = value
            .get("tag")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| Error::missing_field("tag"))?;

        // Step 3: Dispatch directly based on the tag. `from_value` is highly
        // optimized for this and does not re-parse the string.
        match tag {
            "a" => serde_json::from_value(value).map(Element::Link),
            "div" | "span" | "ol" | "ul" | "li" | "details" | "summary" => {
                serde_json::from_value(value).map(Element::Styled)
            }
            "ruby" | "rt" | "rp" | "t" | "table" | "thead" | "tbody" | "tfoot" | "tr" | "tb"
            | "tf" => serde_json::from_value(value).map(Element::Unstyled),
            "td" | "th" => serde_json::from_value(value).map(Element::Table),
            "br" => serde_json::from_value(value).map(Element::LineBreak),
            "img" => serde_json::from_value(value).map(Element::Image),
            unknown_tag => Err(Error::unknown_variant(unknown_tag, &["..."])),
        }
        .map_err(|e| {
            serde::de::Error::custom(format!(
                "serde_json::from_value failed with error: {e}. The JSON value was: {val_clone}"
            ))
        })
    }
}

/// Represents All `Content` elements that can
/// appear within a `"content":` section.
#[derive(Clone, Debug, PartialEq, Serialize)]
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
#[serde(rename_all = "camelCase")]
pub struct LineBreak {
    /// The `LineBreak`' tag is:
    /// [`HtmlTag::Break`] | `"br"`.
    tag: HtmlTag,
    data: Option<IndexMap<String, String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
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
    pub data: Option<IndexMap<String, String>>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct TableElement {
    /// `TableElement`'s tags could be the following:
    ///
    /// [`HtmlTag::TableData`],
    /// [`HtmlTag::TableHeader`].
    pub tag: HtmlTag,
    pub content: Option<ContentMatchType>,
    pub data: Option<IndexMap<String, String>>,
    pub col_span: Option<u16>,
    pub row_span: Option<u16>,
    pub style: Option<StructuredContentStyle>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
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
    pub data: Option<IndexMap<String, String>>,
    pub style: Option<StructuredContentStyle>,
    /// Hover text for the element.
    pub title: Option<String>,
    pub open: Option<bool>,
    /// Defines the language of an element in the format defined by RFC 5646.
    lang: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
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
#[serde(untagged)]
pub enum NumberOrString {
    Number(f64),
    String(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[skip_serializing_none]
#[serde(rename_all = "camelCase")]
pub struct ImageElement {
    pub tag: Option<HtmlTag>,
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
    pub data: Option<IndexMap<String, String>>,
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

#[test]
fn from_json() {
    let path = &test_utils::TEST_PATHS.tests_dir;
    let file = File::open(path.join("writing.json")).unwrap();
    let reader = BufReader::new(file);
    // Read the JSON contents of the file as an instance of `User`.
    let u: Vec<DatabaseTermEntry> = serde_json::from_reader(reader).unwrap();
    dbg!(&u[0]);
}
