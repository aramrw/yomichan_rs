// use std::{collections::HashMap, sync::LazyLock};
//
// use indexmap::IndexMap;
//
// use super::{
//     descriptors_d::{JapanesePreProcessors, LanguageDescriptor, PreAndPostProcessors},
//     ja::{
//         japanese::is_string_partially_japanese,
//         text_preprocessors::{
//             ALPHABETIC_TO_HIRAGANA, ALPHANUMERIC_WIDTH_VARIANTS, COLLAPSE_EMPHATIC_SEQUENCES,
//             CONVERT_HALF_WIDTH_CHARACTERS, CONVERT_HIRAGANA_TO_KATAKANA,
//             NORMALIZE_COMBINING_CHARACTERS,
//         },
//         transforms::JAPANESE_TRANSFORMS,
//     },
// };
//
// pub static LANGUAGE_DESCRIPTORS_MAP: LazyLock<
//     IndexMap<&str, LanguageDescriptor<JapanesePreProcessors<'static>, ()>>,
// > = LazyLock::new(|| {
//     IndexMap::from([(
//         "ja",
//         LanguageDescriptor {
//             iso: "ja".into(),
//             iso639_3: "jpn".into(),
//             name: "Japanese".into(),
//             example_text: "読め".into(),
//             is_text_lookup_worthy: Some(is_string_partially_japanese),
//             reading_normalizer: None,
//             text_processors: PreAndPostProcessors {
//                 pre: JapanesePreProcessors {
//                     convert_half_width_characters: CONVERT_HALF_WIDTH_CHARACTERS,
//                     alphabetic_to_hiragana: ALPHABETIC_TO_HIRAGANA,
//                     normalize_combining_characters: NORMALIZE_COMBINING_CHARACTERS,
//                     alphanumeric_width_variants: ALPHANUMERIC_WIDTH_VARIANTS,
//                     convert_hiragana_to_katakana: CONVERT_HIRAGANA_TO_KATAKANA,
//                     collapse_emphatic_sequences: COLLAPSE_EMPHATIC_SEQUENCES,
//                 },
//                 post: None,
//             },
//             language_transforms: Some(JAPANESE_TRANSFORMS.clone()),
//         },
//     )])
// });
