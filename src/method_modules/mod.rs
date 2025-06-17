mod dictionary_options;
mod options;

macro_rules! create_method_module_mut {
    ($main_struct:ident, $query_struct:ident, $method_name:ident) => {
        // The method on the main struct now takes and returns mutable references.
        impl<'a> $main_struct<'a> {
            pub fn $method_name(&'a mut self) -> $query_struct<'a> {
                $query_struct { ycd: self }
            }
        }

        // The query struct now holds a MUTABLE reference.
        pub struct $query_struct<'a> {
            pub ycd: &'a mut $main_struct<'a>,
        }
    };
}

pub(crate) use create_method_module_mut;
