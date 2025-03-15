//! copied from [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui-derive/src/attributes.rs)
use syn::Expr;
use syn::Field;
use syn::Ident;
use syn::Member;
use syn::Result;
use syn::Token;
use syn::parse::ParseStream;


pub struct NodeField<'a> {
	pub inner: &'a Field,
	pub attributes: Vec<FieldAttribute>,
}


impl<'a> NodeField<'a> {
	pub fn parse(inner: &'a Field) -> syn::Result<Self> {
		let attributes = inner
			.attrs
			.iter()
			.filter(|attr| {
				attr.path().get_ident().is_some_and(|p| p == "field")
			})
			.map(|attr| attr.parse_args_with(parse_inspectable_attributes))
			.collect::<syn::Result<Vec<_>>>()?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		Ok(Self { inner, attributes })
	}

	pub fn is_optional(&self) -> bool {
		matches!(self.inner.ty, syn::Type::Path(ref p) if p.path.segments.last()
				.map(|s| s.ident == "Option")
				.unwrap_or(false))
	}

	/// Returns true if the field is required.
	pub fn is_required(&self) -> bool {
		!self.is_optional() && self.attribute("default").is_none()
	}


	/// Returns the attribute if it is present.
	pub fn attribute(&self, name: &str) -> Option<&FieldAttribute> {
		self.attributes.iter().find(|attr| {
			attr.name().map(|n| n.to_string() == name).unwrap_or(false)
		})
	}

	/// Returns the value if the attribute is present and has a value.
	#[allow(unused)]
	pub fn attribute_value(&self, name: &str) -> Option<&Expr> {
		self.attribute(name)
			.map(|attr| attr.value.as_ref())
			.flatten()
	}
}


pub struct FieldAttribute {
	pub key: Member,
	pub value: Option<Expr>,
}

impl FieldAttribute {
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
}

fn parse_inspectable_attributes(
	input: syn::parse::ParseStream,
) -> Result<Vec<FieldAttribute>> {
	Ok(input
		.parse_terminated(FieldAttribute::parse, Token![,])?
		.into_iter()
		.collect())
}
