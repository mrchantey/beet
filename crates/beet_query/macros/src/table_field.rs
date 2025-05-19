use beet_common::prelude::*;
use heck::ToTitleCase;
use syn;
use syn::Ident;


pub struct TableField<'a> {
	/// The original field
	pub named_field: NamedField<'a>,
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
	fn deref(&self) -> &Self::Target { &self.named_field }
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
			named_field: inner,
			variant_ident,
			primary_key,
			auto_increment,
			unique,
		}
	}

	/// returns false if the field is either:
	/// - a primary key without partial_include
	/// - a non-primary key with partial_exclude
	#[rustfmt::skip]
	pub fn partial_exclude(&self) -> bool {
		 (self.primary_key && !self.attributes.contains("partial_include"))
		 || self.attributes.contains("partial_exclude") 
	}
}
