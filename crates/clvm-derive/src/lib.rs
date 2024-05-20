extern crate proc_macro;

mod from_clvm;
mod helpers;
mod hide_values;
mod parser;
mod to_clvm;

use from_clvm::from_clvm;
use hide_values::impl_hide_values;
use proc_macro::TokenStream;

use proc_macro2::Span;
use syn::{parse_macro_input, DeriveInput, Ident};
use to_clvm::to_clvm;

const CRATE_NAME: &str = "clvm_traits";

fn crate_name(name: Option<Ident>) -> Ident {
    name.unwrap_or_else(|| Ident::new(CRATE_NAME, Span::call_site()))
}

#[proc_macro_derive(ToClvm, attributes(clvm))]
pub fn to_clvm_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    to_clvm(ast).into()
}

#[proc_macro_derive(FromClvm, attributes(clvm))]
pub fn from_clvm_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    from_clvm(ast).into()
}

#[proc_macro_attribute]
pub fn hide_values(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_hide_values(ast).into()
}
