use std::collections::HashSet;

use syn::GenericParam;

use crate::{input, utils};


pub(crate) struct ItemExtendingVisit<'v>{
    mod_generics_infos: &'v input::ModuleGenerics,
    skip_mod_generics: HashSet<syn::Ident>
}

impl<'v> ItemExtendingVisit<'v> {
    pub fn new(bounds_infos: &'v input::ModuleGenerics) -> Self {
        Self {
            mod_generics_infos: bounds_infos,
            skip_mod_generics: HashSet::new()
        }
    }

    pub fn new_with_skip(bounds_infos: &'v input::ModuleGenerics, skip_dependencies: HashSet<syn::Ident>) -> Self {
        Self {
            mod_generics_infos: bounds_infos,
            skip_mod_generics: skip_dependencies
        }
    }

    fn generics_insert(&self, generics: &mut syn::Generics, mod_generic: &syn::Ident){
        if self.skip_mod_generics.contains(mod_generic) {
            return;
        }

        if generics.params.iter().any(|x| match x {
            syn::GenericParam::Type(t) => &t.ident == mod_generic,
            _ => false
        }) {
            return;
        }

        generics.params.push(syn::GenericParam::Type(syn::TypeParam {
            attrs: Vec::new(),
            ident: mod_generic.clone(),
            colon_token: Some(syn::token::Colon::default()),
            bounds: Default::default(),
            eq_token: None,
            default: None
        }));
    }

    fn expand_generics(&self, generics: &mut syn::Generics) {
        // Find all the module generics in the function signature, ...
        let mut mod_generics = utils::get_generics_mod_generics(&self.mod_generics_infos.generics, generics);
        
        // ...get the dependencies of theses module generics,...
        let mod_generics_deps = self.mod_generics_infos.get_dependencies(&mut mod_generics);
        
        // ... add them as generic parameters, ...
        mod_generics.extend(mod_generics_deps);
        for mod_generic in &mod_generics {
            self.generics_insert(generics, mod_generic);
        }

        // ... add the bounds
        for generic_param in generics.params.iter_mut() {
            let GenericParam::Type(t) = generic_param else {
                continue;
            };
            let Some(new_bounds) = self.mod_generics_infos.generics_bounds.get(&t.ident) else {
                continue;
            };
            let bounds = &t.bounds;
            let nalready_bound = |b: &&syn::TypeParamBound|{
                !bounds.iter().any(|x| utils::compare_bounds(x, b))
            };
            let new_bounds = new_bounds.iter()
                .filter(nalready_bound)
                .cloned()
                .collect::<Vec<_>>();

            t.bounds.extend(new_bounds);
        }

        // ... and add the predicates
        for (predicate, pred_generics) in self.mod_generics_infos.predicates.iter().zip(self.mod_generics_infos.predicates_generics.iter()) {
            if !pred_generics.is_subset(&mod_generics) {
                continue;
            }

            let wherec = generics.make_where_clause();
            wherec.predicates.push(predicate.clone());
        }

    }

    fn inner_skip_mod_generics(&self, generics: &syn::Generics) -> HashSet<syn::Ident> {
        generics.params.iter()
            .filter_map(|x| match x {
                syn::GenericParam::Type(t) => Some(t.ident.clone()),
                _ => None
            })
            .filter(|x| self.mod_generics_infos.generics.contains(x))
            .collect()
    }

    /// Returns a new instance of the visitor for the items inside a trait or an impl
    /// given the generics of the trait or the impl
    fn inner(&self, generics: &syn::Generics) -> Self {
        let skip = self.inner_skip_mod_generics(generics);
        Self::new_with_skip(self.mod_generics_infos, skip)
    }
    
}

impl<'v> syn::visit_mut::VisitMut for ItemExtendingVisit<'v> {

    fn visit_generics_mut(&mut self, i: &mut syn::Generics) {
        self.expand_generics(i);
    }


    fn visit_signature_mut(&mut self, i: &mut syn::Signature) {
        // Find all the module generics in the function signature...
        let mut mod_generics = HashSet::new();
        for input in i.inputs.iter() {
            let input = match input {
                syn::FnArg::Typed(t) => t,
                _ => continue
            };
            mod_generics.extend(utils::get_type_mod_generics(&self.mod_generics_infos.generics, &input.ty));
        }
        if let syn::ReturnType::Type(_, ty) = &i.output {
            mod_generics.extend(utils::get_type_mod_generics(&self.mod_generics_infos.generics, ty));
        }

        // and add them as generic parameters
        for mod_generic in mod_generics {
            self.generics_insert(&mut i.generics, &mod_generic);
        }

        // Expand the generics of the function
        self.expand_generics(&mut i.generics);
    }

    fn visit_item_trait_mut(&mut self, i: &mut syn::ItemTrait) {
        // TODO: Handle the impl restrictions

        // Expand the generics of the trait
        self.expand_generics(&mut i.generics);

        // Continue but skip the generics that are already defined
        let mut visitor = self.inner(&i.generics);
        
        // To avoid wastefull expansion of the generics.We will visit the items of the trait manually
        for item in i.items.iter_mut() {
            visitor.visit_trait_item_mut(item);
        }
    }

    fn visit_item_impl_mut(&mut self, i: &mut syn::ItemImpl) {
        let mut mod_generics = HashSet::new();
        // Get the generics of the trait if there is one
        if let Some(trait_path) = &i.trait_ {
            if let Some(trait_deps) = trait_path.1.segments.last()
                .map(|x| &x.arguments)
                .map(|x| utils::get_path_arguments_mod_generics(&self.mod_generics_infos.generics, x)) {  
                
                mod_generics.extend(trait_deps);
            }
        }
        // Get the generics of the self type
        mod_generics.extend(utils::get_type_mod_generics(&self.mod_generics_infos.generics, &i.self_ty));
        
        // Add them as generic parameters
        for mod_generic in mod_generics {
            self.generics_insert(&mut i.generics, &mod_generic);
        }

        // Expand the generics of the impl
        self.expand_generics(&mut i.generics);

        // List the generics of the impl & add them to the skip list
        let mut visitor = self.inner(&i.generics);

        // To avoid wastefull expansion of the generics.We will visit the items of the impl manually
        for item in i.items.iter_mut() {
            visitor.visit_impl_item_mut(item);
        }
    }

}