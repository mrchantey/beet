//! inspired by [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui-derive/src/attributes.rs)
use quote::ToTokens;
use syn::Expr;
use syn::ExprLit;
use syn::Ident;
use syn::Lit;
use syn::Member;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;

/// [`AttributeItem`] collection parsed from Syn attributes.
/// These cannot be ref types because they are parsed.
#[derive(Debug)]
pub struct AttributeGroup {
	pub attributes: Vec<AttributeItem>,
}



impl AttributeGroup {
	pub fn parse(attrs: &[syn::Attribute], name: &str) -> Result<Self> {
		let attributes = attrs
			.iter()
			.filter(|attr| attr.path().get_ident().is_some_and(|p| p == name))
			.map(|attr| attr.parse_args_with(parse_inspectable_attributes))
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect();
		Ok(Self { attributes })
	}
	/// ## Errors
	/// if any of the attributes does not match a provided key
	pub fn validate_allowed_keys(&self, keys: &[&str]) -> Result<&Self> {
		for attr in &self.attributes {
			if let Some(name) = attr.name() {
				if !keys.contains(&name.to_string().as_str()) {
					return Err(syn::Error::new(
						name.span(),
						format!(
							"Invalid Attribute key `{}`. Allowed attributes are: {}",
							name,
							keys.join(", ")
						),
					));
				}
			}
		}
		Ok(&self)
	}

	/// Returns the attribute if it is present.
	pub fn get(&self, name: &str) -> Option<&AttributeItem> {
		self.attributes.iter().find(|attr| {
			attr.name().map(|n| n.to_string() == name).unwrap_or(false)
		})
	}

	pub fn get_many(&self, name: &str) -> Vec<&AttributeItem> {
		self.attributes
			.iter()
			.filter(|attr| {
				attr.name().map(|n| n.to_string() == name).unwrap_or(false)
			})
			.collect()
	}

	pub fn contains(&self, name: &str) -> bool { self.get(name).is_some() }

	/// Returns the value if the attribute is present and has a value.
	pub fn get_value(&self, name: &str) -> Option<&Expr> {
		self.get(name).map(|attr| attr.value.as_ref()).flatten()
	}

	pub fn get_value_parsed<T: Parse>(&self, name: &str) -> Result<Option<T>> {
		if let Some(attr) = self.get(name) {
			attr.value_parsed()
		} else {
			Ok(None)
		}
	}
}

/// An attribute item.
/// `#[foo(bar=7)]` would be parsed as `AttributeItem { key: "bar", value: Some(7) }`
///
#[derive(Debug)]
pub struct AttributeItem {
	pub key: Member,
	pub value: Option<Expr>,
}

impl AttributeItem {
	pub fn parse(input: ParseStream) -> syn::Result<Self> {
		let key: syn::Member = input.parse()?;
		let value = if input.peek(syn::Token![=]) {
			let _eq_token: syn::Token![=] = input.parse()?;
			Some(input.parse()?)
		} else {
			None
		};
		Ok(Self { key, value })
	}

	pub fn name(&self) -> Option<&Ident> {
		match &self.key {
			Member::Named(ident) => Some(ident),
			_ => None,
		}
	}


	/// Handle `#[foo(bar="baz")]` and `#[foo(bar=bazz)]`, useful
	/// for when generics are required that arent valid tokens
	pub fn value_parsed<T: Parse>(&self) -> Result<Option<T>> {
		if let Some(expr) = &self.value {
			let val = if let Expr::Lit(ExprLit { lit, .. }) = expr
				&& let Lit::Str(lit) = &lit
			{
				syn::parse_str(&lit.value())?
			} else {
				syn::parse2(expr.to_token_stream())?
			};
			Ok(Some(val))
		} else {
			Ok(None)
		}
	}
}

fn parse_inspectable_attributes(
	input: syn::parse::ParseStream,
) -> Result<Vec<AttributeItem>> {
	Ok(input
		.parse_terminated(AttributeItem::parse, Token![,])?
		.into_iter()
		.collect())
}
