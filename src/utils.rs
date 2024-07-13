use std::collections::HashSet;

use quote::ToTokens;
use syn::visit::Visit;


macro_rules! bail {
    ($span:expr, $msg:expr) => {
        return Err(syn::Error::new_spanned($span, $msg))
    };
}
/// A macro that exit a compact function with an a syn error 
pub(crate) use bail;

/// A visitor that finds module generics in a syn AST
struct BoundsDependentiesVisit<'ast> {
    /// The generics to find
    generics: &'ast[syn::Ident],
    /// Whether the generics have been found
    found: Vec<bool>
}

impl<'ast> BoundsDependentiesVisit<'ast> {
    // Create a new instance of the visitor
    fn new(generics: &'ast [syn::Ident]) -> Self {
        let is_dependency = vec![false; generics.len()];
        Self {
            generics,
            found: is_dependency
        }
    }

    // Mark a generic as found
    fn mark(&mut self, index: usize) {
        self.found[index] = true;
    }

    // Reset the found generics
    fn reset(&mut self) {
        self.found = vec![false; self.generics.len()];
    }

    // Accumulate the found generics in a vector
    fn get_dependencies(&self) -> Vec<syn::Ident> {
        self.generics.iter().zip(self.found.iter())
            .filter(|(_, &is_dependency)| is_dependency)
            .map(|(ident, _)| ident.clone())
            .collect()
    }
}

impl<'ast> syn::visit::Visit<'ast> for BoundsDependentiesVisit<'ast> {
    fn visit_type_path(&mut self, i: &'ast syn::TypePath) {
        if let Some(ident) = i.path.get_ident() {
            if let Some(index) = self.generics.iter().position(|x| x == ident) {
                self.mark(index);
            }
        }
        syn::visit::visit_type_path(self, i);
    }

    fn visit_generic_param(&mut self, i: &'ast syn::GenericParam) {
        let ident = match i {
            syn::GenericParam::Type(type_param) => &type_param.ident,
            syn::GenericParam::Lifetime(_lifetime) => return,
            syn::GenericParam::Const(_const_param) => return,
        };

        if let Some(index) = self.generics.iter().position(|x| x == ident) {
            self.mark(index);
        }

        syn::visit::visit_generic_param(self, i);
    }
}

pub fn get_bounds_mod_generics<'ast>(generics: &[syn::Ident], bound: impl IntoIterator<Item = &'ast syn::TypeParamBound>) -> HashSet<syn::Ident> {
    let mut dependencies = HashSet::new();
    let mut visitor = BoundsDependentiesVisit::new(generics);

    for bound in bound.into_iter() {
        visitor.visit_type_param_bound(bound);
        dependencies.extend(visitor.get_dependencies());
        visitor.reset();
    }

    dependencies
}

pub fn get_generics_mod_generics(generics: &[syn::Ident], ast: & syn::Generics) -> HashSet<syn::Ident> {
    let mut dependencies = HashSet::new();
    let mut visitor = BoundsDependentiesVisit::new(generics);

    visitor.visit_generics(ast);
    dependencies.extend(visitor.get_dependencies());

    dependencies
}

pub fn get_type_mod_generics(generics: &[syn::Ident], ty: & syn::Type) -> HashSet<syn::Ident> {
    let mut dependencies = HashSet::new();
    let mut visitor = BoundsDependentiesVisit::new(generics);

    visitor.visit_type(ty);
    dependencies.extend(visitor.get_dependencies());

    dependencies
}

pub fn get_path_arguments_mod_generics(generics: &[syn::Ident], path_arg: & syn::PathArguments) -> HashSet<syn::Ident> {
    let mut dependencies = HashSet::new();
    let mut visitor = BoundsDependentiesVisit::new(generics);

    visitor.visit_path_arguments(path_arg);
    dependencies.extend(visitor.get_dependencies());

    dependencies
}

pub fn get_predicate_mod_generics(generics: &[syn::Ident], predicate: & syn::WherePredicate) -> HashSet<syn::Ident> {
    let mut dependencies = HashSet::new();
    let mut visitor = BoundsDependentiesVisit::new(generics);

    visitor.visit_where_predicate(predicate);
    dependencies.extend(visitor.get_dependencies());

    dependencies
}

pub fn compare_bounds(a: &syn::TypeParamBound, b: &syn::TypeParamBound) -> bool {
    let a = a.to_token_stream().to_string();
    let b = b.to_token_stream().to_string();
    a == b
}
