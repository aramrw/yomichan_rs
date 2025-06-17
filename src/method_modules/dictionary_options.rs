use std::collections::HashSet;

use super::create_method_module_mut;
use crate::{
    settings::{self, DictionaryOptions},
    Yomichan,
};
use indexmap::IndexMap;
use module_macros::{ref_variant, skip_ref};

//#[derive(CreateMutModuleVariant)]
//create_method_module_mut!(Yomichan, ModDictionaryOptionsMut, dictionary_options);

#[ref_variant]
impl<'a> ModDictionaryOptionsMut<'a> {
    pub fn get_all_mut(&'a mut self) -> &'a mut IndexMap<String, DictionaryOptions> {
        &mut self
            .ycd
            .backend
            .options
            .get_current_profile_mut()
            .options
            .dictionaries
    }

    pub fn get_by_names_mut(
        &'a mut self,
        names: HashSet<impl AsRef<str>>,
    ) -> Vec<(usize, &'a str, &'a mut DictionaryOptions)> {
        let mut pointers: Vec<(usize, *const str, *mut DictionaryOptions)> =
            Vec::with_capacity(names.len());
        let all = self.get_all_mut();
        for name in names {
            let name = name.as_ref();
            if let Some((index, map_key, value)) = all.get_full_mut(name) {
                pointers.push((
                    index,
                    map_key.as_str() as *const str,
                    value as *mut DictionaryOptions,
                ));
            }
        }
        pointers
            .into_iter()
            .map(|(i, key_ptr, value_ptr)| {
                // SAFETY: This is safe because:
                // 1. The pointers are valid as they were obtained from `all`, which is tied to 'a.
                // 2. The pointers are guaranteed to be non-aliasing because the `names`
                //    argument is a `HashSet`, which ensures every key we look up is unique.
                unsafe { (i, &*key_ptr, &mut *value_ptr) }
            })
            .collect()
    }

    pub fn get_by_index_mut(
        &'a mut self,
        indexes: HashSet<usize>,
    ) -> Vec<(usize, &'a str, &'a mut DictionaryOptions)> {
        let mut pointers: Vec<(usize, *const str, *mut DictionaryOptions)> =
            Vec::with_capacity(indexes.len());
        let all = self.get_all_mut();
        for i in indexes {
            if let Some((map_key, value)) = all.get_index_mut(i) {
                pointers.push((
                    i,
                    map_key.as_str() as *const str,
                    value as *mut DictionaryOptions,
                ));
            }
        }
        pointers
            .into_iter()
            .map(|(i, key_ptr, value_ptr)| {
                // SAFETY: This is safe because:
                // 1. The pointers are valid as they were obtained from `all`, which is tied to 'a.
                // 2. The pointers are guaranteed to be non-aliasing because the `names`
                //    argument is a `HashSet`, which ensures every key we look up is unique.
                unsafe { (i, &*key_ptr, &mut *value_ptr) }
            })
            .collect()
    }
}

impl<'a> ModDictionaryOptionsRef<'a> {
    pub fn get_by_names_owned(
        &'a mut self,
        names: &[impl AsRef<str>],
    ) -> Vec<(usize, String, DictionaryOptions)> {
        let all = self.get_all();
        names
            .iter()
            .filter_map(|n| {
                let n = n.as_ref();
                let (i, key, val) = all.get_full(n)?;
                Some((i, key.clone(), val.clone()))
            })
            .collect()
    }
    pub fn get_by_index_owned(
        &'a mut self,
        indexes: &[usize],
    ) -> Vec<(usize, String, DictionaryOptions)> {
        let all = self.get_all();
        indexes
            .iter()
            .filter_map(|i| {
                let (key, val) = all.get_index(*i)?;
                Some((*i, key.clone(), val.clone()))
            })
            .collect()
    }
}
