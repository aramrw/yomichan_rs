use std::{fmt, fs::File, hash::Hash, io::BufReader, marker::PhantomData};

use indexmap::IndexMap;
use serde::{
    de::{self, Error, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
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
        // Step 1: Deserialize into a generic Value.
        let value = serde_json::Value::deserialize(deserializer).map_err(|e| {
            de::Error::custom(format!(
                "Failed to deserialize into intermediate Value: {}",
                e
            ))
        })?;

        // We'll store errors from each attempt
        let mut errors = Vec::new();

        // Step 2: Try as Element (expects an object or array representing a tag).
        if value.is_object() || value.is_array() {
            match Element::deserialize(value.clone()) {
                Ok(element) => return Ok(ContentMatchType::Element(Box::new(element))),
                Err(e) => errors.push(format!("[Attempted as Element] {}", e)),
            }
        }

        // Step 3: Try as Vec<ContentMatchType> (expects an array).
        if value.is_array() {
            match <Vec<ContentMatchType>>::deserialize(value.clone()) {
                Ok(content_vec) => return Ok(ContentMatchType::Content(content_vec)),
                Err(e) => errors.push(format!("[Attempted as Vec<ContentMatchType>] {}", e)),
            }
        }

        // Step 4: Try as String.
        if value.is_string() {
            match String::deserialize(value.clone()) {
                Ok(s) => return Ok(ContentMatchType::String(s)),
                Err(e) => errors.push(format!("[Attempted as String] {}", e)),
            }
        }

        // Step 5: If all attempts failed, report everything.
        Err(de::Error::custom(format!(
            "Data did not match any variant of ContentMatchType (Element, Vec, or String).\n\
            Problematic value: {}\n\n\
            Errors:\n- {}",
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| format!("{:?}", value)),
            errors.join("\n- ")
        )))
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TermGlossaryDeinflection {
    pub form_of: String,
    pub rules: Vec<String>,
}

impl<'de> Deserialize<'de> for TermGlossary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize into a generic Value to inspect it multiple times.
        let value = serde_json::Value::deserialize(deserializer).map_err(de::Error::custom)?;

        // Attempt to parse as both variants.
        let deinflection_result = TermGlossaryDeinflection::deserialize(value.clone());
        let content_result = TermGlossaryContent::deserialize(value.clone());

        match (deinflection_result, content_result) {
            // Case 1: Parsed as both (the ambiguity we need to solve).
            (Ok(deinflection), Ok(content)) => {
                // This is where we apply our tie-breaker rule.
                // We inspect the raw `value` that caused the ambiguity.
                // If it's an array and its first element is the specific string "structured-content",
                // we definitively choose the `Content` variant.
                if let Some(arr) = value.as_array() {
                    if let Some(first_elem) = arr.first() {
                        if first_elem.as_str() == Some("structured-content") {
                            // This is the binary representation of a StructuredContent enum,
                            // so we MUST choose the Content path.
                            return Ok(TermGlossary::Content(content));
                        }
                    }
                }

                // If the tie-breaker rule doesn't apply (e.g., it was some other
                // ambiguous structure), we have to make a choice. Prioritizing
                // Deinflection might be a reasonable default if such a case could exist.
                // Or, you could panic here if this state is considered impossible.
                // For now, let's assume the rule above is sufficient and prioritize Deinflection otherwise.
                Ok(TermGlossary::Content(content))
            }

            // Case 2: Only parsed as Deinflection.
            (Ok(deinflection), Err(_)) => Ok(TermGlossary::Deinflection(deinflection)),

            // Case 3: Only parsed as Content. This is the normal, correct path for your JSON.
            (Err(_), Ok(content)) => Ok(TermGlossary::Content(content)),

            // Case 4: Failed to parse as either.
            (Err(de), Err(co)) => Err(de::Error::custom(format!(
                "Data did not match any variant of TermGlossary.\n\
                    Deinflection Error: {}\n\
                    Content Error: {}\n\
                    Value: {:#?}",
                de, co, value
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum TermGlossaryContent {
    Tagged(TaggedContent),
    String(String),
}

impl<'de> Deserialize<'de> for TermGlossaryContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Step 1: Deserialize into a generic Value.
        let value = serde_json::Value::deserialize(deserializer).map_err(|e| {
            de::Error::custom(format!(
                "Failed to deserialize into intermediate Value: {}",
                e
            ))
        })?;

        // Step 2: Try to deserialize as `TaggedContent` (expects a map or a sequence).
        // We'll capture the error if it fails.
        let tagged_error = match TaggedContent::deserialize(value.clone()) {
            Ok(tagged) => return Ok(TermGlossaryContent::Tagged(tagged)),
            Err(e) => e.to_string(), // Keep the error message
        };

        // Step 3: Try to deserialize as `String`. This is where your error likely originates.
        // If `value` is a sequence, this will fail with "invalid type: sequence, expected a string".
        let string_error = match String::deserialize(value.clone()) {
            Ok(s) => return Ok(TermGlossaryContent::String(s)),
            Err(e) => e.to_string(), // Keep the error message
        };

        // Step 4: If both attempts failed, report everything.
        Err(de::Error::custom(format!(
            "Data did not match any variant of TermGlossaryContent (Tagged or String).\n\
            Problematic value: {}\n\n\
            Attempt 1 (as TaggedContent) failed with: {}\n\
            Attempt 2 (as String) failed with: {}",
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| format!("{:?}", value)),
            tagged_error,
            string_error
        )))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
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

impl<'de> Deserialize<'de> for TaggedContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TaggedContentVisitor;

        impl<'de> Visitor<'de> for TaggedContentVisitor {
            type Value = TaggedContent;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a map with a 'type' key (JSON format) or a [tag, payload] sequence (MessagePack format)",
                )
            }

            /// Handles the MessagePack format: `["tag", payload]`
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                // The first element is the tag string.
                let tag: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &"a [tag, payload] sequence"))?;

                // The second element is the payload, which depends on the tag.
                let content = match tag.as_str() {
                    "text" => {
                        let text: String = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(1, &"a text payload"))?;
                        TaggedContent::Text { text }
                    }
                    "img" => {
                        let image_payload: Box<ImageElement> = seq
                            .next_element()?
                            .ok_or_else(|| de::Error::invalid_length(1, &"an image payload"))?;
                        TaggedContent::Image(image_payload)
                    }
                    "structured-content" => {
                        let content: ContentMatchType = seq.next_element()?.ok_or_else(|| {
                            de::Error::invalid_length(1, &"a structured-content payload")
                        })?;
                        TaggedContent::StructuredContent { content }
                    }
                    _ => {
                        return Err(de::Error::unknown_variant(
                            &tag,
                            &["text", "img", "structured-content"],
                        ))
                    }
                };

                // Ensure there are no more elements in the sequence.
                if seq.next_element::<de::IgnoredAny>()?.is_some() {
                    return Err(de::Error::invalid_length(3, &self));
                }

                Ok(content)
            }

            /// Handles the JSON format: `{"type": "tag", ...}`
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                // To handle an internally tagged enum from a map, the easiest
                // way is to deserialize into a generic value and then use
                // from_value, which re-applies Serde's `#[serde(tag = "...")]` logic.
                let value =
                    serde_json::Value::deserialize(de::value::MapAccessDeserializer::new(map))?;

                // This helper struct allows us to leverage Serde's derived logic for internally tagged enums.
                #[derive(Deserialize)]
                #[serde(tag = "type")]
                enum Helper {
                    #[serde(rename = "text")]
                    Text { text: String },
                    #[serde(rename = "img")]
                    Image(Box<ImageElement>),
                    #[serde(rename = "structured-content")]
                    StructuredContent {
                        #[serde(rename = "content")]
                        content: ContentMatchType,
                    },
                }

                // Deserialize from the intermediate `serde_json::Value`.
                let helper = Helper::deserialize(value).map_err(de::Error::custom)?;

                // Convert from the helper enum back to our main TaggedContent enum.
                Ok(match helper {
                    Helper::Text { text } => TaggedContent::Text { text },
                    Helper::Image(img) => TaggedContent::Image(img),
                    Helper::StructuredContent { content } => {
                        TaggedContent::StructuredContent { content }
                    }
                })
            }
        }

        deserializer.deserialize_any(TaggedContentVisitor)
    }
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
    Img,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    Baseline,
    Sub,
    Super,
    #[serde(rename = "text-bottom")]
    TextTop,
    #[serde(rename = "text-bottom")]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeUnits {
    Px,
    Em,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

struct ElementVisitor;

impl<'de> Visitor<'de> for ElementVisitor {
    type Value = Element;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .write_str("a map with a 'tag' key (for JSON) or a sequence/tuple (for MessagePack)")
    }

    // This method will be called by `rmp_serde` when it sees an array.
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // Here we recreate your logic for the array format.
        // The idea is to deserialize the entire sequence into a serde_json::Value
        // ONLY because the rest of your logic depends on it. A more performant
        // solution would deserialize field-by-field.
        let value_seq: Vec<Value> =
            de::Deserialize::deserialize(de::value::SeqAccessDeserializer::new(&mut seq))?;
        let value = Value::Array(value_seq);

        // Now you can call a helper function with your existing logic
        deserialize_element_from_value(value).map_err(de::Error::custom)
    }

    // This method will be called by `serde_json` when it sees an object.
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // Same principle: reconstruct a `serde_json::Value` and use your existing logic.
        let value_map: serde_json::Map<String, Value> =
            de::Deserialize::deserialize(de::value::MapAccessDeserializer::new(&mut map))?;
        let value = Value::Object(value_map);

        // Call the same helper function
        deserialize_element_from_value(value).map_err(de::Error::custom)
    }
}

fn deserialize_element_from_value(value: Value) -> Result<Element, String> {
    // Determine the tag from either the map or array structure.
    let tag_str = if let Some(obj) = value.as_object() {
        obj.get("tag")
            .and_then(Value::as_str)
            .ok_or("Element map is missing a 'tag' field")?
    } else if let Some(arr) = value.as_array() {
        if arr.is_empty() {
            return Err("Element array cannot be empty".to_string());
        }
        arr[0]
            .as_str()
            .ok_or("First element of Element array must be a tag string".to_string())?
    } else {
        return Err(format!(
            "Element must be a map or an array, but was: {:?}",
            value
        ));
    };

    // Use `serde_json::from_value` to deserialize into the correct concrete struct.
    // We must clone `value` because `from_value` consumes it.
    let result = match tag_str {
        "a" => serde_json::from_value(value.clone()).map(Element::Link),
        "div" | "span" | "ol" | "ul" | "li" | "details" | "summary" => {
            serde_json::from_value(value.clone()).map(Element::Styled)
        }
        "ruby" | "rt" | "rp" | "t" | "table" | "thead" | "tbody" | "tfoot" | "tr" | "tb" | "tf" => {
            serde_json::from_value(value.clone()).map(Element::Unstyled)
        }
        "td" | "th" => serde_json::from_value(value.clone()).map(Element::Table),
        "br" => serde_json::from_value(value.clone()).map(Element::LineBreak),
        "img" => serde_json::from_value(value.clone()).map(Element::Image),
        unknown_tag => {
            // Replicate the behavior of `Error::unknown_variant` by creating a useful error message.
            let known_variants = &[
                "a", "div", "span", "ol", "ul", "li", "details", "summary", "ruby", "rt", "rp",
                "t", "table", "thead", "tbody", "tfoot", "tr", "tb", "tf", "td", "th", "br", "img",
            ];
            // We need to return a `Result<_, serde_json::Error>` to match the other arms.
            // A simple way is to create an `io::Error`.
            return Err(format!(
                "unknown variant `{}`, expected one of {:?}",
                unknown_tag, known_variants
            ));
        }
    };

    // Add the detailed final error message, which is very helpful for debugging.
    result.map_err(|e| {
        format!(
            "Failed to deserialize Element with tag '{}'. Error: {}. Original value was: {}",
            tag_str, e, value
        )
    })
}

impl<'de> Deserialize<'de> for Element {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // The deserializer now dispatches to the correct visitor method
        // based on the data format it is reading.
        deserializer.deserialize_any(ElementVisitor)
    }
}

/// Represents All `Content` elements that can
/// appear within a `"content":` section.
#[skip_serializing_none]
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
#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineBreak {
    /// The `LineBreak`' tag is:
    /// [`HtmlTag::Break`] | `"br"`.
    pub tag: HtmlTag,
    data: Option<IndexMap<String, String>>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
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

impl<'de> Deserialize<'de> for TableElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TableElementVisitor;

        impl<'de> Visitor<'de> for TableElementVisitor {
            type Value = TableElement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence for a TableElement")
            }

            // This is the method that will be called for your MessagePack data
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Field 1: Tag (required, always first)
                let tag: HtmlTag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                // Now, we handle the rest of the fields which might be optional or in any order.
                // The most robust way is to read them all as generic values and then pick them apart.
                let mut content = None;
                let mut row_span = None;
                let mut col_span = None;
                let mut style = None;
                let mut data = None;

                // Loop through the remaining elements in the sequence
                while let Some(value) = seq.next_element::<serde_json::Value>()? {
                    // Try to see if the value is a number (for row_span/col_span)
                    if let Some(num) = value.as_u64() {
                        // Business rule: assume the first number is row_span, second is col_span
                        if row_span.is_none() {
                            row_span = Some(num as u16);
                        } else if col_span.is_none() {
                            col_span = Some(num as u16);
                        }
                        continue; // Go to next item in sequence
                    }

                    // Try to see if it's a style object
                    if let Ok(s) = serde_json::from_value::<StructuredContentStyle>(value.clone()) {
                        style = Some(s);
                        continue;
                    }

                    // Try to see if it's a data object
                    if let Ok(d) = serde_json::from_value::<IndexMap<String, String>>(value.clone())
                    {
                        data = Some(d);
                        continue;
                    }

                    // If it's none of the above, it must be the content.
                    // We can only have one content field.
                    if content.is_none() {
                        content = Some(serde_json::from_value(value).map_err(de::Error::custom)?);
                    }
                }

                Ok(TableElement {
                    tag,
                    content,
                    data,
                    col_span,
                    row_span,
                    style,
                    lang: None, // lang is not in the sequence format
                })
            }

            // OPTIONAL: To maintain compatibility with JSON map format if needed
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                // This will deserialize from the map-based JSON format
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Helper {
                    tag: HtmlTag,
                    content: Option<ContentMatchType>,
                    data: Option<IndexMap<String, String>>,
                    col_span: Option<u16>,
                    row_span: Option<u16>,
                    style: Option<StructuredContentStyle>,
                    lang: Option<String>,
                }

                let helper = Helper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(TableElement {
                    tag: helper.tag,
                    content: helper.content,
                    data: helper.data,
                    col_span: helper.col_span,
                    row_span: helper.row_span,
                    style: helper.style,
                    lang: helper.lang,
                })
            }
        }

        // This allows Serde to call visit_seq for sequences and visit_map for maps
        deserializer.deserialize_any(TableElementVisitor)
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
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

/// A generic visitor that can deserialize a map directly, or convert a
/// sequence into a temporary map-like `Value` and deserialize from that.
pub struct FlexibleElementVisitor<T> {
    _marker: PhantomData<T>,
}

impl<T> FlexibleElementVisitor<T> {
    pub fn new() -> Self {
        FlexibleElementVisitor {
            _marker: PhantomData,
        }
    }
}

impl<'de, T> Visitor<'de> for FlexibleElementVisitor<T>
where
    T: de::DeserializeOwned, // The target type (e.g., TableElement)
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map or a sequence representing an element")
    }

    /// This is called for your database's sequence format.
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // 1. Build a serde_json::Map from the sequence. This is the adapter logic.
        let mut map = serde_json::Map::new();

        // Tag is always first and required.
        let tag: String = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &"tag"))?;
        map.insert("tag".to_string(), Value::String(tag));

        // Loop through the rest of the optional, unordered fields.
        let mut content_val: Option<Value> = None;
        while let Some(value) = seq.next_element::<Value>()? {
            // Check for number types (rowSpan, colSpan)
            if value.is_u64() {
                // Heuristic: first number is rowSpan, second is colSpan.
                if !map.contains_key("rowSpan") {
                    map.insert("rowSpan".to_string(), value);
                } else if !map.contains_key("colSpan") {
                    map.insert("colSpan".to_string(), value);
                }
                continue;
            }

            // Heuristic: A map is likely the 'data' field.
            if value.is_object() && !map.contains_key("data") {
                map.insert("data".to_string(), value);
                continue;
            }

            // Heuristic: An array could be 'style' or 'content'.
            // This is the trickiest part. A simple rule might be:
            // if it's an array of objects/strings, it's content.
            // if it's an array of simple values/specific objects, it's style.
            // For now, let's assume anything that isn't a known attribute is content.
            if content_val.is_none() {
                content_val = Some(value);
            } else if !map.contains_key("style") {
                // If content is already taken, this might be style.
                map.insert("style".to_string(), value);
            }
        }

        if let Some(content) = content_val {
            map.insert("content".to_string(), content);
        }

        // 2. Deserialize the target type T from the map we just built.
        T::deserialize(Value::Object(map)).map_err(de::Error::custom)
    }

    /// This is called for your JSON file's map format.
    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // Since the input is already a map, we can deserialize directly.
        T::deserialize(de::value::MapAccessDeserializer::new(map))
    }
}

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageElement {
    pub tag: HtmlTag,
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

// ===================================================================
//
//          MANUAL DESERIALIZE IMPLEMENTATIONS FOR ELEMENTS
//
// ===================================================================
//
// This section provides manual `Deserialize` implementations for all
// element structs. This is necessary because the database can store
// elements in a compact "sequence" format (e.g., ["span", ...])
// while the source JSON files use a "map" format (e.g., {"tag": "span", ...}).
//
// Each implementation uses a visitor that can handle BOTH formats,
// making the parsing logic robust across all data sources.
//
// ===================================================================

// --- Implementation for StyledElement ---

impl<'de> Deserialize<'de> for StyledElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StyledElementVisitor;

        impl<'de> Visitor<'de> for StyledElementVisitor {
            type Value = StyledElement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence for a StyledElement")
            }

            // Handles JSON map format: {"tag": "span", ...}
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // This part remains the same, it correctly handles JSON
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Helper {
                    tag: HtmlTag,
                    content: Option<ContentMatchType>,
                    data: Option<IndexMap<String, String>>,
                    style: Option<StructuredContentStyle>,
                    title: Option<String>,
                    open: Option<bool>,
                    lang: Option<String>,
                }

                let helper = Helper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(StyledElement {
                    tag: helper.tag,
                    content: helper.content,
                    data: helper.data,
                    style: helper.style,
                    title: helper.title,
                    open: helper.open,
                    lang: helper.lang,
                })
            }

            // Handles database sequence format: ["span", ...]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let tag: HtmlTag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let mut content = None;
                let mut data = None;
                let mut style = None;
                let mut title = None;
                let mut open = None;

                while let Some(value) = seq.next_element::<Value>()? {
                    // --- START OF MODIFIED LOGIC ---

                    // Is it a boolean? -> `open`
                    if let Some(b) = value.as_bool() {
                        if open.is_none() {
                            open = Some(b);
                        }
                        continue;
                    }

                    // Is it a map? -> `data`
                    if let Ok(d) = serde_json::from_value::<IndexMap<String, String>>(value.clone())
                    {
                        if data.is_none() {
                            data = Some(d);
                        }
                        continue;
                    }

                    // Is it a string that isn't content yet? -> `title`
                    if let Some(s) = value.as_str() {
                        if title.is_none() {
                            title = Some(s.to_string());
                            // Don't assume it's title, could be content. We'll let content take priority.
                        }
                    }

                    // Is it an array? THIS IS THE NEW PART. It could be `style` or `content`.
                    if let Some(arr) = value.as_array() {
                        // Heuristic: If all elements are numbers or CSS-like strings, it's a style array.
                        let is_likely_style_array = arr.iter().all(|v| {
                            v.is_number() || (v.is_string() && !v.as_str().unwrap().contains('【'))
                        });

                        if is_likely_style_array && style.is_none() {
                            // Convert the style array into a style map that StructuredContentStyle understands.
                            // This is a simplified conversion. You may need to make this more specific
                            // based on the exact format of the style array.
                            let mut style_map = serde_json::Map::new();
                            if arr.len() > 0 {
                                style_map.insert("fontSize".to_string(), arr[0].clone());
                            }
                            if arr.len() > 1 {
                                style_map.insert("verticalAlign".to_string(), arr[1].clone());
                            }
                            if arr.len() > 2 {
                                style_map.insert("marginLeft".to_string(), arr[2].clone());
                            }
                            if arr.len() > 3 {
                                style_map.insert("marginRight".to_string(), arr[3].clone());
                            }
                            if let Ok(s) = serde_json::from_value(Value::Object(style_map)) {
                                style = Some(s);
                            }
                            continue;
                        }
                    }

                    // If none of the above, it must be content.
                    if content.is_none() {
                        content = Some(serde_json::from_value(value).map_err(de::Error::custom)?);
                        // If we just assigned content, what we thought was title might have been content.
                        if title.is_some() {
                            if let Some(ContentMatchType::String(s)) = &content {
                                if s == title.as_ref().unwrap() {
                                    title = None;
                                }
                            }
                        }
                    }
                    // --- END OF MODIFIED LOGIC ---
                }

                Ok(StyledElement {
                    tag,
                    content,
                    data,
                    style,
                    title,
                    open,
                    lang: None,
                })
            }
        }

        deserializer.deserialize_any(StyledElementVisitor)
    }
}

// --- Implementation for UnstyledElement ---

impl<'de> Deserialize<'de> for UnstyledElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UnstyledElementVisitor;

        impl<'de> Visitor<'de> for UnstyledElementVisitor {
            type Value = UnstyledElement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence for an UnstyledElement")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Helper {
                    tag: HtmlTag,
                    content: Option<ContentMatchType>,
                    data: Option<IndexMap<String, String>>,
                    lang: Option<String>,
                }
                let helper = Helper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(UnstyledElement {
                    tag: helper.tag,
                    content: helper.content,
                    data: helper.data,
                    lang: helper.lang,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let tag: HtmlTag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let mut content = None;
                let mut data = None;

                while let Some(value) = seq.next_element::<Value>()? {
                    if let Ok(d) = serde_json::from_value::<IndexMap<String, String>>(value.clone())
                    {
                        if data.is_none() {
                            data = Some(d);
                        }
                        continue;
                    }
                    if content.is_none() {
                        content = Some(serde_json::from_value(value).map_err(de::Error::custom)?);
                    }
                }

                Ok(UnstyledElement {
                    tag,
                    content,
                    data,
                    lang: None,
                })
            }
        }

        deserializer.deserialize_any(UnstyledElementVisitor)
    }
}

// --- Implementation for LinkElement ---

impl<'de> Deserialize<'de> for LinkElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LinkElementVisitor;

        impl<'de> Visitor<'de> for LinkElementVisitor {
            type Value = LinkElement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence for a LinkElement")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Helper {
                    tag: HtmlTag,
                    content: Option<ContentMatchType>,
                    href: String,
                    lang: Option<String>,
                }
                let helper = Helper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(LinkElement {
                    tag: helper.tag,
                    content: helper.content,
                    href: helper.href,
                    lang: helper.lang,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let tag: HtmlTag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                let mut content = None;
                let mut href = None;

                // For a link, we expect two more items: the content and the href string.
                // We can distinguish them heuristically: hrefs often start with '?' or 'http'.
                while let Some(value) = seq.next_element::<Value>()? {
                    if let Some(s) = value.as_str() {
                        if s.starts_with('?') || s.starts_with("http") {
                            if href.is_none() {
                                href = Some(s.to_string());
                            }
                            continue;
                        }
                    }
                    if content.is_none() {
                        content = Some(serde_json::from_value(value).map_err(de::Error::custom)?);
                    }
                }

                Ok(LinkElement {
                    tag,
                    content,
                    href: href.ok_or_else(|| de::Error::missing_field("href"))?,
                    lang: None,
                })
            }
        }

        deserializer.deserialize_any(LinkElementVisitor)
    }
}

// --- Implementation for ImageElement ---

impl<'de> Deserialize<'de> for ImageElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ImageElementVisitor;

        impl<'de> Visitor<'de> for ImageElementVisitor {
            type Value = ImageElement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence for an ImageElement")
            }

            // Handles JSON map format
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Helper {
                    tag: HtmlTag,
                    content: Option<()>,
                    vertical_align: Option<VerticalAlign>,
                    border: Option<String>,
                    border_radius: Option<String>,
                    size_units: Option<SizeUnits>,
                    data: Option<IndexMap<String, String>>,
                    path: String,
                    width: Option<f32>,
                    height: Option<f32>,
                    preferred_width: Option<f32>,
                    preferred_height: Option<f32>,
                    title: Option<String>,
                    alt: Option<String>,
                    description: Option<String>,
                    pixelated: Option<bool>,
                    image_rendering: Option<ImageRendering>,
                    appearance: Option<ImageAppearance>,
                    background: Option<bool>,
                    collapsed: Option<bool>,
                    collapsible: Option<bool>,
                }

                let helper = Helper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(ImageElement {
                    tag: helper.tag,
                    content: helper.content,
                    vertical_align: helper.vertical_align,
                    border: helper.border,
                    border_radius: helper.border_radius,
                    size_units: helper.size_units,
                    data: helper.data,
                    path: helper.path,
                    width: helper.width,
                    height: helper.height,
                    preferred_width: helper.preferred_width,
                    preferred_height: helper.preferred_height,
                    title: helper.title,
                    alt: helper.alt,
                    description: helper.description,
                    pixelated: helper.pixelated,
                    image_rendering: helper.image_rendering,
                    appearance: helper.appearance,
                    background: helper.background,
                    collapsed: helper.collapsed,
                    collapsible: helper.collapsible,
                })
            }

            // Handles database sequence format: ["img", "em", "path", 1.0, ...]
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                // Based on the log, the sequence appears to be:
                // [tag, size_units, path, width, height, alt, appearance, pixelated, collapsed, collapsible]
                let tag: HtmlTag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;

                // The rest of the fields have a fixed order in this compact format.
                let size_units: Option<SizeUnits> = seq.next_element()?.unwrap_or(None);
                let path: String = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(2, &self))?;
                let width: Option<f32> = seq.next_element()?.unwrap_or(None);
                let height: Option<f32> = seq.next_element()?.unwrap_or(None);
                let alt: Option<String> = seq.next_element()?.unwrap_or(None);
                let appearance: Option<ImageAppearance> = seq.next_element()?.unwrap_or(None);
                let pixelated: Option<bool> = seq.next_element()?.unwrap_or(None);
                let collapsed: Option<bool> = seq.next_element()?.unwrap_or(None);
                let collapsible: Option<bool> = seq.next_element()?.unwrap_or(None);

                Ok(ImageElement {
                    tag,
                    path,
                    size_units,
                    width,
                    height,
                    alt,
                    appearance,
                    pixelated,
                    collapsed,
                    collapsible,
                    // Fields not present in the sequence format
                    content: None,
                    vertical_align: None,
                    border: None,
                    border_radius: None,
                    data: None,
                    preferred_width: None,
                    preferred_height: None,
                    title: None,
                    description: None,
                    image_rendering: None,
                    background: None,
                })
            }
        }

        deserializer.deserialize_any(ImageElementVisitor)
    }
}
// --- Implementation for LineBreak ---

impl<'de> Deserialize<'de> for LineBreak {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct LineBreakVisitor;

        impl<'de> Visitor<'de> for LineBreakVisitor {
            type Value = LineBreak;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map or sequence for a LineBreak")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Helper {
                    tag: HtmlTag,
                    data: Option<IndexMap<String, String>>,
                }
                let helper = Helper::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(LineBreak {
                    tag: helper.tag,
                    data: helper.data,
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let tag: HtmlTag = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let data: Option<IndexMap<String, String>> = seq.next_element()?.unwrap_or(None);

                Ok(LineBreak { tag, data })
            }
        }

        deserializer.deserialize_any(LineBreakVisitor)
    }
}

#[test]
fn from_json() {
    let path = &test_utils::TEST_PATHS.tests_dir;
    let file = File::open(path.join("自業自得_rust.json")).unwrap();
    let reader = BufReader::new(file);
    let u: Vec<DatabaseTermEntry> = serde_json::from_reader(reader).unwrap();
    dbg!(&u[0]);
}
