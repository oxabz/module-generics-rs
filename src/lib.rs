/*!
module_generics is a procedural macro attribute that allows you to greatly reduce the generics bounds duplication in your code.
It allows you to define a module with generics and the macro will automatically add the generics bounds to the generics parameters.
It also adds the module generics to the generics parameters when they are used in function signatures and impl blocks..

As of now, the macro only supports type generics. It does not support lifetimes and const generics.

*/

use proc_macro::TokenStream;
use syn::{parse_macro_input, visit_mut::VisitMut};

mod input;
mod utils;
mod edit;

// TODO: 
// - Add support for all generics including lifetimes
// - Add support for default values

/**
This macro attribute allows you to define a module with generics.

**Example**
```rust, no_run
#[module_generics::module_generics(
    T: Clone + Display,
    U: Debug
)]
mod __ { // If the module name is __, the macro will return the content of the module
    // No need to repeat the bounds
    // The macro add the bounds to module generics
    struct CloneDisplay<T>(T);

    // The module generics in the impl self type and trait will be added
    // to the impl generics
    impl CloneDisplay<T> {
        fn new(t: T) -> Self {
            Self(t)
        }
    }
    // Similarly the macro add the module generics in function signature to the function generics
    fn display(t: T) {
        println!("{}", t);
    }

    // The macro also detects module generics in deep nested generics
    fn debug_several(u: impl IntoIterator<Item = U>) {
        println!("{:?}", u);
    }
}

 */
#[proc_macro_attribute]
pub fn module_generics(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute
    let attribute = parse_macro_input!(attr as input::ModuleGenericAttribute);
    let generics_info = input::ModuleGenerics::from_attribute(attribute).unwrap();
    // Parse the module
    let mut item = parse_macro_input!(item as syn::ItemMod);
    if item.content.is_none() {
        return syn::Error::new_spanned(item, "Module must have a body").to_compile_error().into()
    }
    // Visit the module and apply the module
    edit::ItemExtendingVisit::new(&generics_info).visit_item_mod_mut(&mut item);

    // Return the modified item
    if item.ident == "__" {
        let (_, content) = item.content.unwrap();
        quote::quote! {
            #( #content )*
        }.into()
    } else {
        quote::quote! {
            #item
        }.into()
    }
}