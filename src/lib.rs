use proc_macro::TokenStream;
use syn::{parse_macro_input, visit_mut::VisitMut};

mod input;
mod utils;
mod edit;

// TODO: 
// - Add support for all generics including lifetimes
// - Add support for default values


#[proc_macro_attribute]
pub fn module_generics(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attribute = parse_macro_input!(attr as input::ModuleGenericAttribute);
    let generics_info = input::ModuleGenerics::from_attribute(attribute).unwrap();

    let mut item = parse_macro_input!(item as syn::ItemMod);
    if item.content.is_none() {
        return syn::Error::new_spanned(item, "Module must have a body").to_compile_error().into()
    }
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