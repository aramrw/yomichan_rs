use std::collections::HashMap;
mod freq;

/// Enum representing what database field was used to match the source term.
pub enum TermSourceMatchSource {
    Term,
    Reading,
    Sequence,
}
/// Enum representing how the search term relates to the final term.
pub enum TermSourceMatchType {
    Exact,
    Prefix,
    Suffix,
}

