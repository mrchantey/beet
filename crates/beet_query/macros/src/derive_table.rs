use crate::table_field::TableField;
use beet_build::prelude::*;
use heck::ToSnakeCase;
use proc_macro2;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use sweet::prelude::*;
use syn;
use syn::DeriveInput;
use syn::Ident;
use syn::Result;

pub fn parse_derive_table(input: DeriveInput) -> TokenStream {
	DeriveTable::new(&input)
		.and_then(|table| table.parse())
		.unwrap_or_else(|err| err.into_compile_error())
}

struct DeriveTable<'a> {
	input: &'a DeriveInput,
	/// the name of the table
	table_name: TokenStream,
	/// the attributes of the table
	/// ie `#[table(name = "foo", if_not_exists)]`
	table_attributes: AttributeGroup,
	/// the fields of the table
	/// ```no_run
	/// #[field(name = "foo", value_type = ValueType::Text)]`
	/// foo: Bar
	/// ```
	fields: Vec<TableField<'a>>,
	/// This will be `MyTableColumns`
	columns_ident: Ident,
	/// This will be `InsertMyTable`
	insert_ident: Ident,
}

impl<'a> DeriveTable<'a> {
	fn new(input: &'a DeriveInput) -> Result<Self> {
		let attributes = AttributeGroup::parse(&input.attrs, "table")?;
		attributes.validate_allowed_keys(&["name", "if_not_exists"])?;
		let fields = NamedField::parse_all(&input)?
			.into_iter()
			.map(|f| f.xmap(|f| TableField::new(f)))
			.collect::<Vec<_>>();

		Self {
			input,
			table_name: attributes
				.get_value("name")
				.map(ToTokens::to_token_stream)
				.unwrap_or_else(|| {
					input.ident.to_string().to_snake_case().to_token_stream()
				}),
			columns_ident: Ident::new(
				&format!("{}Columns", &input.ident),
				input.ident.span(),
			),
			insert_ident: Ident::new(
				&format!("Insert{}", &input.ident),
				input.ident.span(),
			),
			table_attributes: attributes,
			fields,
		}
		.xok()
	}
	fn parse(&self) -> Result<TokenStream> {
		let impl_table = self.impl_table()?;
		let columns_struct = self.columns_struct()?;
		let impl_columns = self.impl_into_column()?;
		let insert_struct = self.insert_struct()?;
		let impl_insert_table_view = self.impl_insert_table_view()?;

		quote! {
			use beet::prelude::*;
			#impl_table

			#columns_struct
			#impl_columns

			#insert_struct
			#impl_insert_table_view
		}
		.xok()
	}

	fn impl_table(&self) -> Result<TokenStream> {
		let if_not_exists =
			self.table_attributes.get_value("if_not_exists").map(|v| {
				quote! {
					fn if_not_exists() -> bool {
						#v
					}
				}
			});
		let columns_ident = &self.columns_ident;
		let ident = &self.input.ident;
		let table_name = &self.table_name;
		quote! {

			impl Table for #ident {
				type Columns = #columns_ident;
				fn name() -> String {
					#table_name.into()
				}
				#if_not_exists
			}

		}
		.xok()
	}
	fn columns_struct(&self) -> Result<TokenStream> {
		let variants = self.fields.iter().map(|field| &field.variant_ident);

		let vis = &self.input.vis;

		let columns_ident = &self.columns_ident;
		quote! {
			#vis enum #columns_ident {
				#(#variants),*
			}
		}
		.xok()
	}
	fn impl_into_column(&self) -> Result<TokenStream> {
		let match_column = self
			.fields
			.iter()
			.map(parse_column)
			.collect::<Result<Vec<_>>>()?;

		let variants = self
			.fields
			.iter()
			.map(|field| {
				let ident = &field.variant_ident;
				quote! {
					Self::#ident
				}
				.xok()
			})
			.collect::<Result<Vec<_>>>()?;

		let table_ident = &self.input.ident;
		let columns_ident = &self.columns_ident;
		quote! {
			impl Columns for #columns_ident {
				type Table = #table_ident;

				fn all() -> Vec<Column> {
					vec![#(#variants.into_column()),*]
				}

				fn into_column(&self) -> Column {
					match self {
						#(#match_column),*
					}
				}
			}
		}
		.xok()
	}

	fn insert_struct(&self) -> Result<TokenStream> {
		let fields = self.fields.iter().map(|field| {
			let ident = &field.ident;
			let inner_ty = &field.inner_ty;
			if field.insert_required() {
				quote! {
					#ident: #inner_ty
				}
			} else {
				quote! {
					#ident: Option<#inner_ty>
				}
			}
		});
		let vis = &self.input.vis;
		let insert_ident = &self.insert_ident;
		quote! {
			#vis struct #insert_ident {
				#(#fields),*
			}
		}
		.xok()
	}

	fn impl_insert_table_view(&self) -> Result<TokenStream> {
		let insert_ident = &self.insert_ident;
		let columns_ident = &self.columns_ident;

		let columns = self.fields.iter().filter_map(|field| {
			let variant_ident = &field.variant_ident;
			// if field.insert_required() {
			Some(quote! {#columns_ident::#variant_ident})
			// } else {
			// 	None
			// }
		});

		let push_values = self.fields.iter().map(|field| {
			let ident = &field.ident;
			if field.insert_required() {
				quote! {
					values.push(self.#ident.try_into_value()?);
				}
			} else {
				quote! {
					if let Some(v) = self.#ident {
						values.push(v.try_into_value()?);
					} else {
						values.push(Value::Null);
					}
				}
			}
		});

		let table_ident = &self.input.ident;

		quote! {
			impl TableView for #insert_ident{
				type Table = #table_ident;
				fn columns() -> Vec<#columns_ident> {
					vec![#(#columns),*]
				}
				fn into_values(self) -> ParseValueResult<Vec<Value>> {
					let mut values = vec![];
					#(#push_values)*
					Ok(values)
				}

			}
		}
		.xok()
	}
}







fn parse_column(field: &TableField) -> Result<TokenStream> {
	let name = field
		.attributes
		.get_value("name")
		.map(ToTokens::to_token_stream)
		.unwrap_or_else(|| field.ident.to_string().to_token_stream());
	let value_type = field.value_type()?;

	let optional = field.is_optional();


	let default_value = match field.attributes.get_value("default") {
		Some(value) => quote! {
			Some(#value.into())
		},
		None => quote! {None},
	};

	let ident = &field.variant_ident;
	let primary_key = &field.primary_key;
	let auto_increment = &field.auto_increment;
	let unique = &field.unique;
	quote! {
		Self::#ident => Column{
			name: #name.into(),
			value_type: #value_type,
			optional: #optional,
			default_value: #default_value,
			primary_key: #primary_key,
			auto_increment: #auto_increment,
			unique: #unique,
		}
	}
	.xok()
}


#[cfg(test)]
mod test {
	use super::parse_derive_table;
	use quote::quote;
	use sweet::prelude::*;
	use syn::parse_quote;

	#[test]
	fn default() {
		expect(
			parse_derive_table(parse_quote! {
				#[derive(Table)]
				struct MyTable {
					test: u32,
				}
			})
			.to_string(),
		)
		.to_be(
			quote! {
				use beet::prelude::*;
				impl Table for MyTable {
					type Columns = MyTableColumns;
					fn name() -> String {
						"my_table".into()
					}
				}
				enum MyTableColumns{
					Test
				}
				impl Columns for MyTableColumns {
					type Table = MyTable;
					fn all() -> Vec<Column> {
						vec![Self::Test.into_column()]
					}
					fn into_column(&self) -> Column {
						match self {
							Self::Test => Column {
								name: "test".into(),
								value_type: ValueType::Integer,
								optional: false,
								default_value: None,
								primary_key: false,
								auto_increment: false,
								unique: false,
							}
						}
					}
				 }
				struct InsertMyTable {
					test: u32
				}
				impl TableView for InsertMyTable {
					type Table = MyTable;
					fn columns() -> Vec<MyTableColumns> {
						vec![MyTableColumns::Test]
					}
					fn into_values(self) -> ParseValueResult<Vec<Value>> {
						let mut values = vec![];
						values.push(self.test.try_into_value()?);
						Ok(values)
					}
				}
			}
			.to_string(),
		);
	}
	#[test]
	fn with_attributes() {
		expect(
			parse_derive_table(parse_quote! {
				#[derive(Table)]
				#[table(name = "foobar",if_not_exists = false)]
				pub struct MyTable {
					#[field(not_primary_key,default=9)]
					id: Option<u32>,
					#[field(
						name = "FooBazz",
						value_type = ValueType::Text,
						primary_key,
						auto_increment,
						unique
					)]
					test: u32,
				}
			})
			.to_string(),
		)
		.to_be(
			quote! {
				use beet::prelude::*;

				impl Table for MyTable {
					type Columns = MyTableColumns;
					fn name() -> String {
						"foobar".into()
					}
					fn if_not_exists() -> bool {
						false
					}
				}
				pub enum MyTableColumns{
					Id,
					Test
				}
				impl Columns for MyTableColumns {
					type Table = MyTable;
					fn all() -> Vec<Column> {
						vec![Self::Id.into_column(), Self::Test.into_column()]
					}
					fn into_column(&self) -> Column {
						match self {
							Self::Id => Column{
								name: "id".into(),
								value_type: ValueType::Integer,
								optional: true,
								default_value: Some(9.into()),
								primary_key: false,
								auto_increment: false,
								unique: false,
							},
							Self::Test=> Column{
								name: "FooBazz".into(),
								value_type: ValueType::Text,
								optional: false,
								default_value: None,
								primary_key: true,
								auto_increment: true,
								unique: true,
							}
						}
					}
				 }
				pub struct InsertMyTable {
					id: Option<u32>,
					test: Option<u32>
				}
				impl TableView for InsertMyTable {
					type Table = MyTable;
					fn columns() -> Vec<MyTableColumns> {
						vec![MyTableColumns::Id, MyTableColumns::Test]
					}
					fn into_values(self) -> ParseValueResult<Vec<Value>> {
						let mut values = vec![];
						if let Some(v) = self.id {
							values.push(v.try_into_value()?);
						} else {
							values.push(Value::Null);
						}
						if let Some(v) = self.test {
							values.push(v.try_into_value()?);
						} else {
							values.push(Value::Null);
						}
						Ok(values)
					}
				}
			}
			.to_string(),
		);
	}
}
