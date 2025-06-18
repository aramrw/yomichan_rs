use crate::{
    settings::{self, DictionaryOptions},
    Yomichan,
};
use indexmap::{map::MutableKeys, IndexMap};
use module_macros::{ref_variant, skip_ref};
use std::collections::HashSet;

#[ref_variant]
impl<'b> ModDictionaryOptionsMut<'b> {
    pub fn get_all_mut(&'b mut self) -> &'b mut IndexMap<String, DictionaryOptions> {
        &mut self
            .ycd
            .backend
            .options
            .get_current_profile_mut()
            .options
            .dictionaries
    }
    pub fn get_by_names_mut(
        &'b mut self,
        names: HashSet<&'b str>,
    ) -> Vec<(usize, &'b str, &'b mut DictionaryOptions)> {
        let all = self.get_all_mut();
        let mut found = Vec::new();
        for (i, (key, val)) in all.iter_mut().enumerate() {
            found.push((i, key.as_str(), val));
        }
        found
    }
    #[deprecated(note = "
WARNING: THIS FUNCTION IS UNSOUND AND MAY RESULT IN UNDEFINED BEHAVIOR.
This function attempts an O(1) mutable lookup for performance, but it is PROVEN UNSOUND by Miri
and may cause Undefined Behavior by violating Rust's aliasing rules.

Its behavior is NOT guaranteed and may break unpredictably with any compiler update,
dependency change, or shift in architecture. This can result in silent data corruption.

It is recommended to use the safe O(n) version of the function.
By using this function, you are knowingly accepting the risk of Undefined Behavior.")]
    pub unsafe fn get_by_names_mut_unsound(
        &'b mut self,
        names: HashSet<&'b str>,
    ) -> Vec<(usize, &'b str, &'b mut DictionaryOptions)> {
        let mut pointers: Vec<(usize, *const str, *mut DictionaryOptions)> =
            Vec::with_capacity(names.len());
        let all = self.get_all_mut();
        for name in names {
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
            .map(|(i, key_ptr, value_ptr)| unsafe { (i, &*key_ptr, &mut *value_ptr) })
            .collect()
    }
    pub fn get_by_index_mut(
        &'b mut self,
        indexes: HashSet<usize>,
    ) -> Vec<(usize, &'b str, &'b mut DictionaryOptions)> {
        let all = self.get_all_mut();
        let mut found = Vec::new();
        for (i, (key, val)) in all.iter_mut().enumerate() {
            found.push((i, key.as_str(), val));
        }
        found
    }
    #[deprecated(note = "
WARNING: THIS FUNCTION IS UNSOUND AND MAY RESULT IN UNDEFINED BEHAVIOR.
This function attempts an O(1) mutable lookup for performance, but it is PROVEN UNSOUND by Miri
and may cause Undefined Behavior by violating Rust's aliasing rules.

Its behavior is NOT guaranteed and may break unpredictably with any compiler update,
dependency change, or shift in architecture. This can result in silent data corruption.

It is recommended to use the safe O(n) version of the function.
By using this function, you are knowingly accepting the risk of Undefined Behavior.")]
    pub unsafe fn get_by_index_mut_unsound(
        &'b mut self,
        indexes: HashSet<usize>,
    ) -> Vec<(usize, &'b str, &'b mut DictionaryOptions)> {
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
            .map(|(i, key_ptr, value_ptr)| unsafe { (i, &*key_ptr, &mut *value_ptr) })
            .collect()
    }
}

impl<'a, 'data> ModDictionaryOptionsRef<'a, 'data>
where
    'data: 'a,
{
    pub fn get_by_names_owned(
        &mut self,
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
        &mut self,
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

// used for testing in miri without IO
unsafe fn get_by_index_mut_unsound<'b>(
    all: &mut IndexMap<String, DictionaryOptions>,
    indexes: HashSet<usize>,
) -> Vec<(usize, &'b str, &'b mut DictionaryOptions)> {
    let mut pointers: Vec<(usize, *const str, *mut DictionaryOptions)> =
        Vec::with_capacity(indexes.len());
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
        .map(|(i, key_ptr, value_ptr)| unsafe { (i, &*key_ptr, &mut *value_ptr) })
        .collect()
}

#[cfg(test)]
mod mod_dict_opts {
    use std::collections::HashSet;

    use indexmap::IndexMap;

    use crate::{settings::DictionaryOptions, test_utils};

    use super::get_by_index_mut_unsound;

    #[test]
    #[ignore]
    fn get_dictionary_index_unsound() {
        let mut map = IndexMap::new();
        map.insert(
            "dict1".to_string(),
            DictionaryOptions {
                enabled: true,
                ..Default::default()
            },
        );
        map.insert(
            "dict2".to_string(),
            DictionaryOptions {
                enabled: true,
                ..Default::default()
            },
        );
        map.insert(
            "dict3".to_string(),
            DictionaryOptions {
                enabled: true,
                ..Default::default()
            },
        );
        unsafe {
            let found = get_by_index_mut_unsound(&mut map, HashSet::from([0, 2]));
            for (i, _, opts) in found {
                println!("Index {}: before mut: {}", i, opts.enabled);
                opts.enabled = !opts.enabled;
                println!("Index {}: after mut:  {}", i, opts.enabled);
            }
        }
    }

    #[test]
    fn get_mut_index_unsound() {
        let mut ycd = test_utils::YCD.write().unwrap();
        let mut optmod = ycd.mod_dictionary_options_mut();
        let found = optmod.get_by_index_mut(HashSet::from([0, 1]));
        for (i, _, opts) in found {
            println!("Index {}: before mut: {}", i, opts.enabled);
            opts.enabled = !opts.enabled;
            println!("Index {}: after mut:  {}", i, opts.enabled);
        }
        // this saves to the db
        ycd.update_options().unwrap();
    }
}
