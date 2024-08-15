use std::{collections::HashMap, sync::LazyLock};

use super::{
    descriptors_d::{JapanesePreProcessors, LanguageDescriptor, PreAndPostProcessors},
    ja::{
        japanese::is_string_partially_japanese,
        text_preprocessors::{
            ALPHABETIC_TO_HIRAGANA, ALPHANUMERIC_WIDTH_VARIANTS, COLLAPSE_EMPHATIC_SEQUENCES,
            CONVERT_HALF_WIDTH_CHARACTERS, CONVERT_HIRAGANA_TO_KATAKANA,
            NORMALIZE_COMBINING_CHARACTERS,
        },
        transforms::JAPANESE_TRANSFORMS,
    },
};

