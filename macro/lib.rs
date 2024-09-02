use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::abort;
use syn::*;
use template_quote::quote;

fn get_tag(tokens: &TokenStream) -> usize {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    tokens.to_string().hash(&mut hasher);
    hasher.finish() as usize
}

#[proc_macro_derive(Commonized)]
pub fn derive_commonized(input: TokenStream1) -> TokenStream1 {
    let item =
        parse::<Item>(input.clone()).unwrap_or_else(|_| abort!(Span::call_site(), "Bad item"));
    let (generics, ident) = match &item {
        Item::Enum(ie) => (&ie.generics, &ie.ident),
        Item::Struct(is) => (&is.generics, &is.ident),
        _ => abort!(Span::call_site(), "Bad item"),
    };
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let tag = get_tag(&input.into());
    quote! {
        unsafe impl #impl_generics ::pyo3_commonize::Commonized for #ident #ty_generics #where_clause {
            const __COMMONIZED_INTERNAL_TAG: usize = #tag;
            const __COMMONIZED_MODPATH: &'static str = ::core::module_path!();
            const __COMMONIZED_MANIFEST_DIR: &'static str = env!("CARGO_MANIFEST_DIR");
        }
    }
    .into()
}
