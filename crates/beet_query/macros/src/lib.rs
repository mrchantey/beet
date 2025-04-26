#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(stmt_expr_attributes)]

mod derive_table;
use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::parse_macro_input;


/// Define a sql table
#[proc_macro_derive(Table, attributes())]
pub fn table(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	derive_table::parse_derive_table(input).into()
}
