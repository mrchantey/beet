use crate::table_field::TableField;
use beet_core::prelude::*;
use heck::ToSnakeCase;
use proc_macro2;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use beet_utils::prelude::*;
use syn;
use syn::DeriveInput;
use syn::Ident;
use syn::LitBool;
use syn::Result;



pub(crate) struct DeriveTable<'a> {
	input: &'a DeriveInput,
	/// The ident used by `TableView`, for a #[derive(Table)]
	/// this is Self, but for a #[derive(TableView)] this is
	/// #[table_view(table=User)]
	table_ident: Ident,
	/// the name of the table, ie "users"
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
	cols_ident: Ident,
	/// This will be `MyTablePartial`
	partial_ident: Ident,
}

impl<'a> DeriveTable<'a> {
	fn new(input: &'a DeriveInput) -> Result<Self> {
		let attributes = AttributeGroup::parse(&input.attrs, "table")?;
		attributes.validate_allowed_keys(&["name", "if_not_exists"])?;
		let fields = NamedField::parse_derive_input(&input)?
			.into_iter()
			.map(|f| f.xmap(|f| TableField::new(f)))
			.collect::<Vec<_>>();

		let table_view_attributes =
			AttributeGroup::parse(&input.attrs, "table_view")?;
		table_view_attributes.validate_allowed_keys(&["table"])?;
		let table_ident = table_view_attributes
			.get_value_parsed::<Ident>("table")?
			.unwrap_or_else(|| input.ident.clone());

		Self {
			input,
			table_name: attributes
				.get_value("name")
				.map(ToTokens::to_token_stream)
				.unwrap_or_else(|| {
					table_ident.to_string().to_snake_case().to_token_stream()
				}),
			cols_ident: Ident::new(
				&format!("{}Cols", &table_ident),
				table_ident.span(),
			),
			partial_ident: Ident::new(
				&format!("{}Partial", &table_ident),
				table_ident.span(),
			),
			table_attributes: attributes,
			table_ident,
			fields,
		}
		.xok()
	}
	pub fn parse_derive_table(input: DeriveInput) -> TokenStream {
		DeriveTable::new(&input)
			.and_then(|table| table.parse_derive_table_inner())
			.unwrap_or_else(|err| err.into_compile_error())
	}
	pub fn parse_derive_table_view(input: DeriveInput) -> TokenStream {
		DeriveTable::new(&input)
			.and_then(|table| table.parse_derive_table_view_inner())
			.unwrap_or_else(|err| err.into_compile_error())
	}

	fn parse_derive_table_inner(&self) -> Result<TokenStream> {
		let impl_table = self.impl_table()?;
		let columns_enum = self.columns_enum()?;
		let impl_columns = self.impl_columns()?;
		let partial_table = self.partial_table()?;
		let impl_table_view = self.impl_table_view()?;


		quote! {
			use beet::prelude::*;
			#impl_table
			#impl_table_view

			#columns_enum
			#impl_columns

			#partial_table
		}
		.xok()
	}
	fn parse_derive_table_view_inner(&self) -> Result<TokenStream> {
		let impl_table_view = self.impl_table_view()?;
		quote! {
			use beet::prelude::*;
			#impl_table_view
		}
		.xok()
	}

	fn impl_table(&self) -> Result<TokenStream> {
		let if_not_exists = self
			.table_attributes
			.get_value_parsed::<LitBool>("if_not_exists")?
			.map(|v| v.value())
			.unwrap_or(true)
			.xmap(|v| {
				if v {
					Some(quote! {.if_not_exists()})
				} else {
					None
				}
			});
		let cols_ident = &self.cols_ident;

		let cols = self.fields.iter().map(|field| {
			let variant_ident = &field.variant_ident;
			// TODO foreign key
			quote! {
				.col(#cols_ident::#variant_ident)
			}
		});

		let columns_ident = &self.cols_ident;
		let ident = &self.input.ident;
		let table_name = &self.table_name;
		quote! {

			impl Table for #ident {
				type Columns = #columns_ident;
				fn name() -> std::borrow::Cow<'static, str> {
					#table_name.into()
				}
				fn stmt_create_table() -> beet::exports::sea_query::TableCreateStatement{
					beet::exports::sea_query::Table::create()
					.table(CowIden::new(#table_name))
					#if_not_exists
					#(#cols)*
					.to_owned()
				}
			}
		}
		.xok()
	}
	fn columns_enum(&self) -> Result<TokenStream> {
		let variants = self.fields.iter().map(|field| {
			let iden_attrs = field
				.named_field
				.attrs
				.iter()
				.filter(|attr| attr.path().is_ident("iden"));

			let variant_ident = &field.variant_ident;
			quote! {
				#(#iden_attrs)*
				#variant_ident
			}
		});

		let vis = &self.input.vis;

		let columns_ident = &self.cols_ident;
		quote! {
			#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash , beet::exports::sea_query::Iden)]
			#vis enum #columns_ident {
				#(#variants),*
			}
		}
		.xok()
	}
	fn impl_columns(&self) -> Result<TokenStream> {
		let col_defs = self
			.fields
			.iter()
			.map(parse_col_def)
			.collect::<Result<Vec<_>>>()?;

		let variants = self.fields.iter().map(|field| &field.variant_ident);

		let primary_key = self
			.fields
			.iter()
			.find(|field| field.primary_key)
			.map(|field| {
				let variant = &field.variant_ident;
				quote! {
					fn primary_key() -> Option<Self> {
						Some(Self::#variant)
					}
				}
			});

		let value_types = self.fields.iter().map(|field| {
			let value_type = field
				.field_attributes
				.get_value("type")
				.map(|v| v.to_token_stream())
				.unwrap_or_else(|| field.inner_ty.to_token_stream());
			let variant_ident = &field.variant_ident;
			quote! {
				Self::#variant_ident => #value_type::value_type()
			}
		});

		let table_ident = &self.input.ident;
		let columns_ident = &self.cols_ident;
		quote! {
			impl Columns for #columns_ident {
				type Table = #table_ident;

				#primary_key

				fn variants() -> Vec<Self> {
					vec![#(Self::#variants),*]
				}
			}
			impl beet::exports::sea_query::IntoColumnDef for #columns_ident {
				fn into_column_def(self) -> beet::exports::sea_query::ColumnDef {
					match self {
						#(#col_defs),*
					}
				}
			}
			impl ValueIntoValueType for #columns_ident {
				fn value_type(&self) -> ValueType {
					match self {
						#(#value_types),*
					}
				}
			}
		}
		.xok()
	}


	/// Build a partial version of the table, ie `UserPartial` without
	/// primary keys and fields marked with `partial_exclude`
	fn partial_table(&self) -> Result<TokenStream> {
		let fields = self.fields.iter().filter_map(|field| {
			if field.partial_exclude() {
				None
			} else {
				let ident = &field.ident;
				let ty = &field.named_field.ty;
				Some(quote! {
					#ident: #ty
				})
			}
		});
		let vis = &self.input.vis;
		let partial_ident = &self.partial_ident;

		let table_ident = &self.input.ident;
		quote! {
			#[derive(TableView)]
			#[table_view(table = #table_ident)]
			#vis struct #partial_ident {
				#(#fields),*
			}
		}
		.xok()
	}

	/// Implements `TableView` for the table
	fn impl_table_view(&self) -> Result<TokenStream> {
		let col_variants = self.fields.iter().map(|field| &field.variant_ident);

		let push_values = self.fields.iter().map(|field| {
			let ident = &field.ident;
			if field.is_optional() {
				quote! {
					if let Some(v) = self.#ident {
						values.push(v.into_value()?);
					} else {
						values.push(Value::Null);
					}
				}
			} else {
				quote! {
					values.push(self.#ident.into_value()?);
				}
			}
		});
		let field_idents = self.fields.iter().map(|field| &field.ident);

		let primary_value = self
			.fields
			.iter()
			.find(|field| field.primary_key)
			.map(|field| {
				let ident = &field.ident;
				quote! {
					fn primary_value(&self) -> ConvertValueResult<Option<Value>> {
						Ok(Some(self.#ident.into_value()?))
					}
				}
			});

		let primary_key_type = self
			.fields
			.iter()
			.find(|field| field.primary_key)
			.map(|field| field.named_field.ty.to_token_stream())
			.unwrap_or_else(|| quote! { () });

		let ident = &self.input.ident;
		let table_ident = &self.table_ident;
		let cols_ident = &self.cols_ident;
		let num_fields = self.fields.len();

		quote! {
			impl TableView for #ident{
				type Table = #table_ident;
				type PrimaryKey = #primary_key_type;

				#primary_value

				fn columns() -> Vec<#cols_ident> {
					vec![#(#cols_ident::#col_variants),*]
				}
				fn into_row(self) -> ConvertValueResult<Row> {
					let mut values = vec![];
					#(#push_values)*
					Ok(Row::new(values))
				}

				fn from_row(row: Row) -> Result<Self,DeserializeError> {
					let values = row.inner();
					if values.len() != #num_fields {
						return Err(DeserializeError::RowLengthMismatch {
							expected: #num_fields,
							received: values.len(),
						})
					}
					let mut values = values.into_iter();
					Ok(Self {
						#(#field_idents: values.next().unwrap().into_other()?,)*
					})
				}
			}
		}
		.xok()
	}
}

fn parse_col_def(field: &TableField) -> Result<TokenStream> {
	// we now use derive Iden with #[iden="foo"] instead
	// let name = field
	// 	.attributes
	// 	.get_value("name")
	// 	.map(ToTokens::to_token_stream)
	// 	.unwrap_or_else(|| field.ident.to_string().to_token_stream());

	let not_null = if field.is_optional() {
		None
	} else {
		Some(quote! {.not_null()})
	};

	let primary_key = if field.primary_key {
		Some(quote! {.primary_key()})
	} else {
		None
	};

	let auto_increment = if field.auto_increment {
		Some(quote! {.auto_increment()})
	} else {
		None
	};

	let default_value = match field.field_attributes.get_value("default") {
		Some(value) => Some(quote! {.default(#value)}),
		None => None,
	};
	let unique = if field.unique {
		Some(quote! {.unique()})
	} else {
		None
	};

	let ident = &field.variant_ident;
	quote! {
		Self::#ident =>{
			let column_type = self.value_type().into_column_type();
			beet::exports::sea_query::ColumnDef::new_with_type(self,column_type)
			#primary_key
			#auto_increment
			#unique
			#default_value
			#not_null
			.to_owned()
		}
	}
	.xok()
}


#[cfg(test)]
mod test {
	use quote::quote;
	use sweet::prelude::*;
	use syn::parse_quote;

	use super::DeriveTable;

	#[test]
	fn default() {
		expect(
			DeriveTable::parse_derive_table(parse_quote! {
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
					type Columns = MyTableCols;
					fn name() -> std::borrow::Cow<'static, str> {
						"my_table".into()
					}
					fn stmt_create_table() -> beet::exports::sea_query::TableCreateStatement {
						beet::exports::sea_query::Table::create()
							.table(CowIden::new("my_table"))
							.if_not_exists()
							.col(MyTableCols::Test)
							.to_owned()
					}
				}
				impl TableView for MyTable {
					type Table = MyTable;
					type PrimaryKey = ();
					fn columns() -> Vec<MyTableCols> {
						vec![MyTableCols::Test]
					}
					fn into_row(self) -> ConvertValueResult<Row> {
						let mut values = vec![];
						values.push(self.test.into_value()?);
						Ok(Row::new(values))
					}
					fn from_row(row: Row) -> Result<Self,DeserializeError> {
						let values = row.inner();
						if values.len() != 1usize {
							return Err(DeserializeError::RowLengthMismatch {
								expected: 1usize,
								received: values.len(),
							})
						}
						let mut values = values.into_iter();
						Ok(Self {
							test: values.next().unwrap().into_other()?,
						})
					}
				}
				#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, beet::exports::sea_query::Iden)]
				enum MyTableCols {
					Test
				}
				impl Columns for MyTableCols {
					type Table = MyTable;
					fn variants() -> Vec<Self> {
						vec![Self::Test]
					}
				}
				impl beet::exports::sea_query::IntoColumnDef for MyTableCols {
					fn into_column_def(self) -> beet::exports::sea_query::ColumnDef {
						match self {
							Self::Test => {
								let column_type = self.value_type().into_column_type();
								beet::exports::sea_query::ColumnDef::new_with_type(self,column_type)
									.not_null()
									.to_owned()
							}
						}
					}
				}
				impl ValueIntoValueType for MyTableCols {
					fn value_type(&self) -> ValueType {
						match self {
							Self::Test => u32::value_type()
						}
					}
				}
				#[derive(TableView)]
				#[table_view(table = MyTable)]
				struct MyTablePartial {
					test: u32
				}
			}
			.to_string(),
		);
	}

	#[test]
	fn with_attributes() {
		expect(
			DeriveTable::parse_derive_table(parse_quote! {
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
					type Columns = MyTableCols;
					fn name() -> std::borrow::Cow<'static, str> {
						"foobar".into()
					}
					fn stmt_create_table() -> beet::exports::sea_query::TableCreateStatement {
						beet::exports::sea_query::Table::create()
							.table(CowIden::new("foobar"))
							.col(MyTableCols::Id)
							.col(MyTableCols::Test)
							.to_owned()
					}
				}
				impl TableView for MyTable {
					type Table = MyTable;
					type PrimaryKey = u32;
					fn primary_value(&self) -> ConvertValueResult<Option<Value>> {
						Ok(Some(self.test.into_value()?))
					}
					fn columns() -> Vec<MyTableCols> {
						vec![MyTableCols::Id, MyTableCols::Test]
					}
					fn into_row(self) -> ConvertValueResult<Row> {
						let mut values = vec![];
						if let Some(v) = self.id {
							values.push(v.into_value()?);
						} else {
							values.push(Value::Null);
						}
						values.push(self.test.into_value()?);
						Ok(Row::new(values))
					}
					fn from_row(row: Row) -> Result<Self,DeserializeError> {
						let values = row.inner();
						if values.len() != 2usize {
							return Err(DeserializeError::RowLengthMismatch {
								expected: 2usize,
								received: values.len(),
							})
						}
						let mut values = values.into_iter();
						Ok(Self {
							id: values.next().unwrap().into_other()?,
							test: values.next().unwrap().into_other()?,
						})
					}
				}
				#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, beet::exports::sea_query::Iden)]
				pub enum MyTableCols {
					Id,
					Test
				}
				impl Columns for MyTableCols {
					type Table = MyTable;
					fn primary_key() -> Option<Self> {
						Some(Self::Test)
					}
					fn variants() -> Vec<Self> {
						vec![Self::Id, Self::Test]
					}
				}
				impl beet::exports::sea_query::IntoColumnDef for MyTableCols {
					fn into_column_def(self) -> beet::exports::sea_query::ColumnDef {
						match self {
							Self::Id => {
								let column_type = self.value_type().into_column_type();
								beet::exports::sea_query::ColumnDef::new_with_type(self,column_type)
									.default(9)
									.to_owned()
							},
							Self::Test => {
								let column_type = self.value_type().into_column_type();
								beet::exports::sea_query::ColumnDef::new_with_type(self,column_type)
									.primary_key()
									.auto_increment()
									.unique()
									.not_null()
									.to_owned()
							}
						}
					}
				}
				impl ValueIntoValueType for MyTableCols {
					fn value_type(&self) -> ValueType {
						match self {
							Self::Id => u32::value_type(),
							Self::Test => u32::value_type()
						}
					}
				}
				#[derive(TableView)]
				#[table_view(table = MyTable)]
				pub struct MyTablePartial {
					id: Option<u32>
				}
			}
			.to_string(),
		);
	}
}
