use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Block, Expr, GenericArgument, Ident, ImplItem, ImplItemFn, ItemImpl, Lifetime, Pat,
    PathArguments, Receiver, ReturnType, Type, parse_macro_input,
    visit::Visit,
    visit_mut::{self, VisitMut},
};

// A simple utility to convert CamelCase to snake_case.
// e.g., "ModDictionaryOptions" -> "mod_dictionary_options"
fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in s.char_indices() {
        if i > 0 && ch.is_uppercase() {
            snake.push('_');
        }
        snake.push(ch.to_ascii_lowercase());
    }
    snake
}

/// Checks if a type is a "simple reference" that can be auto-converted to an owned type.
/// A simple reference is `&'a T`, `Option<&'a T>`, or `Result<&'a T, E>`
/// where `T` itself does not contain any borrowed data.
fn is_simple_ref_to_owned(ret_type: &ReturnType) -> bool {
    if let ReturnType::Type(_, ty) = ret_type {
        if let Some(inner) = get_inner_type_if_simple_ref(ty) {
            // It's a simple reference wrapper. Now, ensure the inner type `T` has no lifetimes.
            return !contains_lifetime(inner);
        }
    }
    false
}

/// A visitor to detect if a `Type` contains any explicit lifetimes.
struct LifetimeVisitor {
    found: bool,
}
impl<'ast> Visit<'ast> for LifetimeVisitor {
    fn visit_lifetime(&mut self, _i: &'ast Lifetime) {
        self.found = true;
    }
}
fn contains_lifetime(ty: &Type) -> bool {
    let mut visitor = LifetimeVisitor { found: false };
    visitor.visit_type(ty);
    visitor.found
}

/// If `ty` is `&T`, `Option<&T>`, or `Result<&T, E>`, returns `Some(T)`. Otherwise `None`.
fn get_inner_type_if_simple_ref(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Reference(type_ref) => Some(&*type_ref.elem),
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last()?;
            let type_name = last_segment.ident.to_string();

            if type_name == "Option" || type_name == "Result" {
                if let PathArguments::AngleBracketed(args) = &last_segment.arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        if let Type::Reference(inner_ref) = inner_ty {
                            return Some(&*inner_ref.elem);
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Removes the outer reference from a type. `&'a T` -> `T`.
fn get_owned_return_type(ret_type: &ReturnType) -> ReturnType {
    if let ReturnType::Type(arrow, ty) = ret_type {
        let new_ty = match ty.as_ref() {
            Type::Reference(type_ref) => *type_ref.elem.clone(),
            Type::Path(type_path) => {
                let mut new_path = type_path.clone();
                let last_segment = new_path.path.segments.last_mut().unwrap();
                if let PathArguments::AngleBracketed(args) = &mut last_segment.arguments {
                    let first_arg = args.args.first_mut().unwrap();
                    if let GenericArgument::Type(Type::Reference(inner_ref)) = first_arg {
                        *first_arg = GenericArgument::Type(*inner_ref.elem.clone());
                    }
                }
                Type::Path(new_path)
            }
            _ => ty.as_ref().clone(),
        };
        return ReturnType::Type(*arrow, Box::new(new_ty));
    }
    ret_type.clone()
}

// #[proc_macro_attribute]
// pub fn ref_variant(_attr: TokenStream, item: TokenStream) -> TokenStream {
//     let input_impl = parse_macro_input!(item as ItemImpl);
//
//     // --- 1. Derive All Names from the Mut Struct ---
//     let mut_struct_ident = if let Type::Path(type_path) = &*input_impl.self_ty {
//         type_path.path.segments.first().unwrap().ident.clone()
//     } else {
//         panic!("`ref_variant` must be on a struct impl.");
//     };
//
//     let base_name = mut_struct_ident
//         .to_string()
//         .strip_suffix("Mut")
//         .unwrap_or_else(|| panic!("Struct name for `ref_variant` must end in 'Mut'."))
//         .to_string();
//
//     let ref_struct_ident = Ident::new(&format!("{}Ref", base_name), mut_struct_ident.span());
//
//     // Convert base name to snake_case for the method names
//     let method_base_name_str = to_snake_case(&base_name);
//     let ref_accessor_ident = Ident::new(&method_base_name_str, mut_struct_ident.span());
//     let mut_accessor_ident = Ident::new(
//         &format!("{}_mut", method_base_name_str),
//         mut_struct_ident.span(),
//     );
//
//     let yomichan_ident = Ident::new("Yomichan", mut_struct_ident.span());
//
//     // --- 2. Transform Methods (Unchanged) ---
//     let mut transformed_methods = Vec::new();
//     // ... (rest of the transformation logic is the same)
//     for item in &input_impl.items {
//         if let ImplItem::Fn(method) = item {
//             let should_skip = method
//                 .attrs
//                 .iter()
//                 .any(|attr| attr.path().is_ident("skip_ref"));
//             if !should_skip {
//                 transformed_methods.push(transform_method_to_ref(method));
//             }
//         }
//     }
//
//     // --- 3. Generate All Boilerplate Code ---
//     let (impl_generics, _ty_generics, where_clause) = input_impl.generics.split_for_impl();
//     let generated_code = quote! {
//         // Part A: The mutable struct (previously from the declarative macro)
//         pub struct #mut_struct_ident #impl_generics #where_clause {
//             pub ycd: &'a mut #yomichan_ident #impl_generics,
//         }
//
//         // Part B: The immutable struct
//         pub struct #ref_struct_ident #impl_generics #where_clause {
//             ycd: &'a #yomichan_ident #impl_generics,
//         }
//
//         // Part C: Both accessor methods on Yomichan
//         impl #impl_generics #yomichan_ident #impl_generics #where_clause {
//             /// Accessor for the immutable variant.
//             pub fn #ref_accessor_ident(&'a self) -> #ref_struct_ident #impl_generics {
//                 #ref_struct_ident { ycd: self }
//             }
//
//             /// Accessor for the mutable variant.
//             pub fn #mut_accessor_ident(&'a mut self) -> #mut_struct_ident #impl_generics {
//                 #mut_struct_ident { ycd: self }
//             }
//         }
//
//         // Part D: The implementation for the immutable struct
//         impl #impl_generics #ref_struct_ident #impl_generics #where_clause {
//             #( #transformed_methods )*
//         }
//     };
//
//     // The final output is the user's manual `impl ...Mut` block AND all the code we just generated.
//     let output = quote! {
//         #input_impl
//         #generated_code
//     };
//
//     output.into()
// }

#[proc_macro_attribute]
pub fn ref_variant(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_impl = parse_macro_input!(item as ItemImpl);

    // --- 1. Derive All Names ---
    let mut_struct_ident = if let Type::Path(type_path) = &*input_impl.self_ty {
        type_path.path.segments.first().unwrap().ident.clone()
    } else {
        panic!("`ref_variant` must be on a struct impl.");
    };
    let base_name = mut_struct_ident
        .to_string()
        .strip_suffix("Mut")
        .unwrap_or_else(|| panic!("Struct name for `ref_variant` must end in 'Mut'."))
        .to_string();
    let ref_struct_ident = Ident::new(&format!("{}Ref", base_name), mut_struct_ident.span());
    let method_base_name_str = to_snake_case(&base_name);
    let ref_accessor_ident = Ident::new(&method_base_name_str, mut_struct_ident.span());
    let mut_accessor_ident = Ident::new(
        &format!("{}_mut", method_base_name_str),
        mut_struct_ident.span(),
    );
    let yomichan_ident = Ident::new("Yomichan", mut_struct_ident.span());

    // --- 2. Generate Ref and Owned Methods ---
    let mut ref_impl_items: Vec<ImplItem> = Vec::new();

    for item in &input_impl.items {
        if let ImplItem::Fn(method) = item {
            if method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("skip_ref"))
            {
                continue;
            }

            let ref_method = transform_method_to_ref(method);
            let should_skip_owned = method
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident("skip_owned"));

            if !should_skip_owned && is_simple_ref_to_owned(&ref_method.sig.output) {
                // --- This is the new, robust method construction ---
                let mut owned_method = ref_method.clone();

                // 1. Modify the signature
                owned_method.sig.ident = Ident::new(
                    &format!("{}_owned", ref_method.sig.ident),
                    ref_method.sig.ident.span(),
                );
                owned_method.sig.output = get_owned_return_type(&ref_method.sig.output);

                // 2. Build the new body: `self.ref_method_name(args...).clone()`
                let ref_method_name = &ref_method.sig.ident;
                let ref_method_args = ref_method.sig.inputs.iter().filter_map(|arg| match arg {
                    syn::FnArg::Typed(pt) => match &*pt.pat {
                        Pat::Ident(pi) => Some(pi.ident.clone()),
                        _ => None,
                    },
                    _ => None,
                });

                let body_suffix = match &ref_method.sig.output {
                    ReturnType::Type(_, ty) => match &**ty {
                        Type::Path(p) => {
                            match p.path.segments.last().unwrap().ident.to_string().as_str() {
                                "Option" => quote! { .cloned() },
                                "Result" => quote! { .map(|v| v.clone()) },
                                _ => quote! { .clone() },
                            }
                        }
                        _ => quote! { .clone() },
                    },
                    _ => quote! { .clone() },
                };

                let body_tokens =
                    quote! { { self.#ref_method_name(#(#ref_method_args),*)#body_suffix } };
                owned_method.block = syn::parse2::<Block>(body_tokens)
                    .expect("Internal macro error: Failed to parse generated block.");

                ref_impl_items.push(ImplItem::Fn(owned_method));
            }

            ref_impl_items.push(ImplItem::Fn(ref_method));
        }
    }

    // --- 3. Generate All Boilerplate ---
    let (impl_generics, _ty_generics, where_clause) = input_impl.generics.split_for_impl();
    let generated_code = quote! {
        pub struct #mut_struct_ident #impl_generics #where_clause { pub ycd: &'a mut #yomichan_ident #impl_generics, }
        pub struct #ref_struct_ident #impl_generics #where_clause { ycd: &'a #yomichan_ident #impl_generics, }
        impl #impl_generics #yomichan_ident #impl_generics #where_clause {
            pub fn #ref_accessor_ident(&'a self) -> #ref_struct_ident #impl_generics { #ref_struct_ident { ycd: self } }
            pub fn #mut_accessor_ident(&'a mut self) -> #mut_struct_ident #impl_generics { #mut_struct_ident { ycd: self } }
        }
        impl #impl_generics #ref_struct_ident #impl_generics #where_clause {
            #( #ref_impl_items )*
        }
    };

    // --- 4. Final Output ---
    let output = quote! {
        #input_impl
        #generated_code
    };
    output.into()
}

#[proc_macro_attribute]
pub fn skip_ref(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

// ===================================================================
//
// THIS IS THE CORRECTED TRANSFORMER LOGIC
//
// ===================================================================

struct ToRefTransformer;

impl VisitMut for ToRefTransformer {
    // Transforms function names like `get_all_mut` -> `get_all`
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        let ident_str = i.to_string();
        if let Some(base_name) = ident_str.strip_suffix("_mut") {
            *i = Ident::new(base_name, i.span());
        }
    }

    // Transforms `&mut self` -> `&self`
    fn visit_receiver_mut(&mut self, r: &mut Receiver) {
        if r.reference.is_some() {
            r.mutability = None;
        }
        visit_mut::visit_receiver_mut(self, r);
    }

    // NEW (MORE THOROUGH): Transforms all types recursively
    fn visit_type_mut(&mut self, t: &mut Type) {
        match t {
            // Change `&mut T` to `&T`
            Type::Reference(type_ref) => {
                type_ref.mutability = None;
            }
            // Change `*mut T` to `*const T`
            Type::Ptr(type_ptr) => {
                type_ptr.mutability = None;
            }
            _ => {}
        }
        // Recurse into nested types like `Vec<&mut T>`
        visit_mut::visit_type_mut(self, t);
    }

    // NEW: This is the key fix. It visits and transforms expressions
    // within the function body.
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        match e {
            // Change `&mut some_expr` to `& some_expr`
            Expr::Reference(expr_ref) => {
                expr_ref.mutability = None;
            }
            // Change `... as *mut T` to `... as *const T`
            Expr::Cast(expr_cast) => {
                // By visiting the type part of the cast, we reuse the
                // logic in `visit_type_mut` to handle the pointer change.
                self.visit_type_mut(&mut expr_cast.ty);
            }
            _ => {}
        }
        // Recurse into sub-expressions
        visit_mut::visit_expr_mut(self, e);
    }
}

/// Helper function to apply the transformer to a single method. (Unchanged)
fn transform_method_to_ref(method: &ImplItemFn) -> ImplItemFn {
    let mut ref_method = method.clone();

    // Transform the method's own name: `get_by_names_mut` -> `get_by_names`
    let ident_str = ref_method.sig.ident.to_string();
    if let Some(base_name) = ident_str.strip_suffix("_mut") {
        ref_method.sig.ident = Ident::new(base_name, ref_method.sig.ident.span());
    }

    // Create our transformer and apply it to the entire method.
    // This will visit the signature, body, types, and all expressions.
    let mut transformer = ToRefTransformer;
    transformer.visit_impl_item_fn_mut(&mut ref_method);

    ref_method
}
