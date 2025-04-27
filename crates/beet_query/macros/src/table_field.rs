use beet_build::prelude::*;
use heck::ToTitleCase;
use proc_macro2::TokenStream;
use quote::ToTokens;
use sweet::prelude::*;
use syn;
use syn::Expr;
use syn::Ident;
use syn::Result;
use syn::Type;
use syn::parse_quote;
use syn::spanned::Spanned;


pub struct TableField<'a> {
	/// The original field
	pub inner: NamedField<'a>,
	/// The TitleCase `Foo` for a field `foo`
	pub variant_ident: Ident,
	/// The `PRIMARY KEY` attribute
	pub primary_key: bool,
	/// The `AUTOINCREMENT` attribute
	pub auto_increment: bool,
	/// The `UNIQUE` attribute
	pub unique: bool,
}

impl<'a> std::ops::Deref for TableField<'a> {
	type Target = NamedField<'a>;
	fn deref(&self) -> &Self::Target { &self.inner }
}


impl<'a> TableField<'a> {
	pub fn new(inner: NamedField<'a>) -> Self {
		let variant_ident = Ident::new(
			&inner.ident.to_string().to_title_case(),
			inner.ident.span(),
		);
		// if the field is called 'id' it is assumed to be the primary key
		let auto_primary_key = inner.ident == "id"
			&& !inner.attributes.contains("not_primary_key");

		let primary_key =
			auto_primary_key || inner.attributes.contains("primary_key");
		let auto_increment =
			auto_primary_key || inner.attributes.contains("auto_increment");
		let unique = inner.attributes.contains("unique");

		Self {
			inner,
			variant_ident,
			primary_key,
			auto_increment,
			unique,
		}
	}

	/// When constructing the default Insert type, this field
	/// will be marked non-optional.
	pub fn insert_required(&self) -> bool {
		self.is_optional() == false
			&& self.primary_key == false
			&& self.attributes.contains("default") == false
	}


	/// This is the ident of the type builder portion of a ColumnDef:
	pub fn column_type(&self) -> Result<TokenStream> {
		self.attributes
			.get_value("type")
			.map(|v| v.to_token_stream().xok())
			.unwrap_or_else(|| {
				parse_column_type(self).map(|v| v.to_token_stream())
			})
	}
}


/// This is the [sea_query::ColumnType] passed into [`sea_query::ColumnDef::new_with_type`]
fn parse_column_type(field: &NamedField) -> Result<Expr> {
	let Type::Path(type_path) = &field.inner_ty else {
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

	// untested but a good start
	#[rustfmt::skip]
	let expr:Expr = match ident.as_str() {
    "String" | "str" => parse_quote!(sea_query::ColumnType::Text),
    "i8" => parse_quote!(sea_query::ColumnType::TinyInteger),
    "i16" => parse_quote!(sea_query::ColumnType::SmallInteger),
    "i32" | "isize" => parse_quote!(sea_query::ColumnType::Integer),
    "i64" => parse_quote!(sea_query::ColumnType::BigInteger),
    "u8" => parse_quote!(sea_query::ColumnType::TinyUnsigned),
    "u16" => parse_quote!(sea_query::ColumnType::SmallUnsigned),
    "u32" | "usize" => parse_quote!(sea_query::ColumnType::Unsigned),
    "u64" => parse_quote!(sea_query::ColumnType::BigUnsigned),
    "f32" => parse_quote!(sea_query::ColumnType::Float),
    "f64" => parse_quote!(sea_query::ColumnType::Double),
    "bool" => parse_quote!(sea_query::ColumnType::Boolean),
    "char" => parse_quote!(sea_query::ColumnType::Char(Some(1))),
    "Vec<u8>" | "[u8]" => parse_quote!(sea_query::ColumnType::Blob),
    "Uuid" => parse_quote!(sea_query::ColumnType::Uuid),
    "NaiveDateTime" => parse_quote!(sea_query::ColumnType::DateTime),
    "DateTime<Utc>" | "DateTime<FixedOffset>" => parse_quote!(sea_query::ColumnType::TimestampWithTimeZone),
    "NaiveDate" => parse_quote!(sea_query::ColumnType::Date),
    "NaiveTime" => parse_quote!(sea_query::ColumnType::Time),
    "Json" | "Value" => parse_quote!(sea_query::ColumnType::Json),
    "Decimal" => parse_quote!(sea_query::ColumnType::Decimal(None)),
    "IpAddr" | "Ipv4Addr" | "Ipv6Addr" => parse_quote!(sea_query::ColumnType::Inet),
    "MacAddr" => parse_quote!(sea_query::ColumnType::MacAddr),
		_ => {
			return Err(syn::Error::new(
				field.inner.ty.span(),
				format!("Unsupported type: {}", ident),
			));
		}
	};
	expr.xok()
}
