use super::create_method_module;
use crate::{
    settings::{self, DictionaryOptions},
    Yomichan,
};

#[derive(CreateMutModuleVariant)]
create_method_module!(Yomichan, ModDictionaryOptions, dictionary_options);

impl<'a> ModDictionaryOptions<'a> {
    pub fn get_all(
        &'a self,
    ) -> &'a indexmap::IndexMap<std::string::String, settings::DictionaryOptions> {
        &self
            .ycd
            .backend
            .options
            .get_current_profile()
            .options
            .dictionaries
    }
    pub fn get_by_names(
        &'a self,
        names: &'a [impl AsRef<str>],
    ) -> Vec<(&'a str, &'a DictionaryOptions)> {
        let all = self.get_all();
        names
            .iter()
            .filter_map(|key| {
                let key = key.as_ref();
                let opts = all.get(key)?;
                Some((key, opts))
            })
            .collect()
    }

    pub fn get_by_index(&'a self, indexes: &'a [usize]) -> Vec<(&'a str, &'a DictionaryOptions)> {
        let all = self.get_all();
        indexes
            .iter()
            .filter_map(|i| {
                let (key, opts) = all.get_index(*i)?;
                Some((key.as_str(), opts))
            })
            .collect()
    }
}
