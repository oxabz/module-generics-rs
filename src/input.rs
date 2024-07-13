use std::collections::{HashMap, HashSet};

use syn::punctuated::Punctuated;

use crate::utils;
use crate::utils::bail;

/// The macro attribute input
pub struct ModuleGenericAttribute {
    pub generics: Punctuated<syn::GenericParam, syn::Token![,]>,
    pub where_clause: Option<syn::WhereClause>,
}

impl syn::parse::Parse for ModuleGenericAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut generics = Punctuated::new();
        while !input.is_empty() && !input.peek(syn::Token![where]) {
            generics.push(input.parse()?);
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        let where_clause = input.parse()?;

        Ok(Self {
            generics,
            where_clause,
        })
    }
    
}

/// The macro input in a structured format
pub struct ModuleGenerics {
    /// List of generics for the module
    pub generics: Vec<syn::Ident>,
    /// List of dependencies for each generic
    pub generics_dependencies: HashMap<syn::Ident, Vec<syn::Ident>>,
    /// List of bounds for each generic
    pub generics_bounds: HashMap<syn::Ident, Punctuated<syn::TypeParamBound, syn::Token![+]>>,
    /// Predicates for the generics
    pub predicates: Vec<syn::WherePredicate>,
    /// Predicates module generics
    pub predicates_generics: Vec<HashSet<syn::Ident>>,
}

impl ModuleGenerics {
    /// Parse the input from the macro attribute
    pub fn from_attribute(attr: ModuleGenericAttribute) -> syn::Result<Self> {
        let mut generics = Vec::new();
        let mut generics_dependencies = HashMap::new();
        let mut generics_bounds = HashMap::new();

        for generic in attr.generics {
            let syn::GenericParam::Type(generic) = generic else {
                bail!(generic, "Only type generics are supported")
            };
            let ident = generic.ident;
            generics.push(ident.clone());

            let dependencies = utils::get_bounds_mod_generics(&generics, &generic.bounds).into_iter().collect();
            generics_dependencies.insert(ident.clone(), dependencies);
        
            generics_bounds.insert(ident.clone(), generic.bounds);
        }

        let predicates = match attr.where_clause {
            Some(where_clause) => where_clause.predicates.into_iter().collect(),
            None => Vec::new()
        };

        let mut predicates_generics = Vec::with_capacity(predicates.len());
        for predicate in &predicates {
            predicates_generics.push(utils::get_predicate_mod_generics(&generics, predicate));
        }
        
        Ok(Self {
            generics,
            generics_dependencies,
            generics_bounds,
            predicates,
            predicates_generics,
        })
    }

    /// Takes the a list of generics and returns the other generics that they depend on
    pub fn get_dependencies(&self, generics: &HashSet<syn::Ident>) -> Vec<syn::Ident> {
        let mut dependencies = Vec::new();
        // Fist we get the dependencies of the generics
        for generic in generics {
            if let Some(deps) = self.generics_dependencies.get(generic) {
                for dep in deps {
                    if !generics.contains(dep) {
                        dependencies.push(dep.clone());
                    }
                }
            }
        }
        // Then we get the dependencies of the dependencies
        let mut dependencies = dependencies;
        let mut i = 0;
        while i < dependencies.len() {
            let dep = &dependencies[i];

            let nalready = |dep: &&syn::Ident| !dependencies.iter().any(|x| x == *dep) && !generics.contains(*dep);
            let new_deps = self.generics_dependencies.get(dep)
                .expect("Every dependency should be in the map")
                .iter()
                .filter(nalready)
                .cloned()
                .collect();

            dependencies = [dependencies, new_deps].concat();
            
            i += 1;
        }

        dependencies
    }
}