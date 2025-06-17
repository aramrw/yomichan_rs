mod dictionary_options;
mod options;

macro_rules! create_method_module {
    ($main_struct:ident, $query_struct:ident, $method_name:ident) => {
        // The method on the main struct to access the query object
        impl<'a> $main_struct<'a> {
            pub fn $method_name(&self) -> $query_struct<'_> {
                $query_struct { ycd: self }
            }
        }

        // The query struct itself
        pub struct $query_struct<'a> {
            ycd: &'a $main_struct<'a>,
        }
    };
}

pub(crate) use create_method_module;
