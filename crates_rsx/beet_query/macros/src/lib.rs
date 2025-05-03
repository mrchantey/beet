#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![feature(stmt_expr_attributes)]
mod derive_table;
mod table_field;
use derive_table::DeriveTable;
use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::parse_macro_input;
/// Define a sql table:
///
///
/// ```no_run
/// #[derive(Table)]
/// struct User{
///
///
/// }
/// ```
#[proc_macro_derive(Table, attributes(table, field, iden))]
pub fn table(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	DeriveTable::parse_derive_table(input).into()
}

/// ```no_run
/// #[derive(TableView)]
/// #[table_view(table=User)]
/// struct UserLastName {
///   last_name: String,
/// }
/// ```
#[proc_macro_derive(TableView, attributes(table_view, field))]
pub fn table_view(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	DeriveTable::parse_derive_table_view(input).into()
}
