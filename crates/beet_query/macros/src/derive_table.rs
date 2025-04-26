use beet_build::prelude::*;
use heck::ToSnakeCase;
use heck::ToTitleCase;
use proc_macro2;
use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use sweet::prelude::*;
use syn;
use syn::DeriveInput;
use syn::Ident;
use syn::Result;
use syn::Type;
use syn::spanned::Spanned;

pub fn parse_derive_table(input: DeriveInput) -> TokenStream {
	parse(input).unwrap_or_else(|err| err.into_compile_error())
}

fn parse(input: DeriveInput) -> Result<TokenStream> {
	let attributes = AttributeGroup::parse(&input.attrs, "table")?;
	attributes.validate_allowed_keys(&["name"])?;
	let fields = NamedField::parse_all(&input)?;
	let impl_table = impl_table(&input, &attributes)?;
	let columns_struct = columns_struct(&input, &fields)?;
	let impl_columns = impl_columns(&input, &fields)?;


	quote! {
		use beet::prelude::*;
		#impl_table
		#columns_struct
		#impl_columns
	}
	.xok()
}


fn impl_table(
	input: &DeriveInput,
	attributes: &AttributeGroup,
) -> Result<TokenStream> {
	let name = attributes
		.get_value("name")
		.map(ToTokens::to_token_stream)
		.unwrap_or_else(|| {
			input.ident.to_string().to_snake_case().to_token_stream()
		});

	let ident = &input.ident;
	let columns_ident = columns_ident(input);
	quote! {

		impl Table for #ident {
			type Columns = #columns_ident;
			fn name() -> &'static str {
				#name
			}
		}

	}
	.xok()
}


fn columns_ident(input: &DeriveInput) -> Ident {
	Ident::new(&format!("{}Columns", &input.ident), input.ident.span())
}

fn columns_struct(
	input: &DeriveInput,
	fields: &[NamedField],
) -> Result<TokenStream> {
	let variants = fields
		.iter()
		.map(|field| {
			let ident = field.ident.to_string().to_title_case();
			let ident = Ident::new(&ident, field.ident.span());
			quote! {
				#ident
			}
			.xok()
		})
		.collect::<Result<Vec<_>>>()?;

	let columns_ident = columns_ident(input);
	quote! {
		pub enum #columns_ident {
			#(#variants),*
		}
	}
	.xok()
}

fn impl_columns(
	input: &DeriveInput,
	fields: &[NamedField],
) -> Result<TokenStream> {
	let match_names = fields
		.iter()
		.map(|field| {
			let name = field
				.attributes
				.get_value("name")
				.map(ToTokens::to_token_stream)
				.unwrap_or_else(|| field.ident.to_string().to_token_stream());
			let ident = field.ident.to_string().to_title_case();
			let ident = Ident::new(&ident, field.ident.span());
			quote! {
				Self::#ident: #name
			}
			.xok()
		})
		.collect::<Result<Vec<_>>>()?;
	let match_sql_types = fields
		.iter()
		.map(|field| {
			let type_val = field
				.attributes
				.get_value("sql_type")
				.map(|v| v.to_token_stream().xok())
				.unwrap_or_else(|| {
					parse_sql_ty(field).map(|v| v.to_token_stream())
				})?;
			let ident = field.ident.to_string().to_title_case();
			let ident = Ident::new(&ident, field.ident.span());
			quote! {
				Self::#ident: #type_val
			}
			.xok()
		})
		.collect::<Result<Vec<_>>>()?;

	let columns_ident = columns_ident(input);
	quote! {
		impl Columns for #columns_ident {

			fn name(&self) -> &'static str {
				match self {
					#(#match_names),*
				}
			}
			fn sql_type(&self) -> beet::sql::SqlType {
				match self {
					#(#match_sql_types),*
				}
			}
		}
	}
	.xok()
}


fn parse_sql_ty(field: &NamedField) -> Result<&'static str> {
	let Type::Path(type_path) = &field.inner.ty else {
		return Err(syn::Error::new(
			field.inner.ty.span(),
			"Only path types are supported",
		));
	};
	let ident = type_path
		.path
		.segments
		.last()
		.ok_or_else(|| {
			syn::Error::new(type_path.path.span(), "No segments found")
		})?
		.ident
		.to_string();

	#[rustfmt::skip]
	match ident.as_str() {
		"String" | "str" => "TEXT",
		"u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" => "INTEGER",
		"f32" | "f64" => "REAL",
		"bool" => "BOOLEAN",
		"chrono::NaiveDateTime" | "chrono::DateTime" => "DATETIME",
		// "Vec" => "BLOB" ,
		_ => {
			return Err(syn::Error::new(
				field.inner.ty.span(),
				format!("Unsupported type: {}", ident),
			));
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
					fn name() -> &'static str {
						"my_table"
					}
				}
				pub enum MyTableColumns{
					Test
				}
				impl Columns for MyTableColumns {
					fn name(&self) -> &'static str {
						match self {
							Self::Test: "test"
						}
					}
					fn sql_type(&self) -> beet::sql::SqlType {
						match self {
							Self::Test: "INTEGER"
						}
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
				#[table(name = "foobar")]
				struct MyTable {
					#[field(name = "FooBazz", sql_type = "TEXT")]
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
					fn name() -> &'static str {
						"foobar"
					}
				}
				pub enum MyTableColumns{
					Test
				}
				impl Columns for MyTableColumns {
					fn name(&self) -> &'static str {
						match self {
							Self::Test: "FooBazz"
						}
					}
					fn sql_type(&self) -> beet::sql::SqlType {
						match self {
							Self::Test: "TEXT"
						}
					}
				}
			}
			.to_string(),
		);
	}
}
