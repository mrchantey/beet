//! copied from [bevy-inspector-egui](https://github.com/jakobhellermann/bevy-inspector-egui/blob/main/crates/bevy-inspector-egui-derive/src/attributes.rs)
use crate::prelude::*;
use syn::Field;
use syn::Result;

#[derive(Debug)]
pub struct PropsField<'a> {
	pub inner: &'a Field,
	pub attributes: AttributeGroup,
}


impl<'a> PropsField<'a> {
	pub fn parse(inner: &'a Field) -> Result<Self> {
		let attributes = AttributeGroup::parse(&inner.attrs, "field")?
			.validate_allowed_keys(&["default", "required", "into"])?;
		Ok(Self { inner, attributes })
	}

	pub fn is_optional(&self) -> bool {
		matches!(self.inner.ty, syn::Type::Path(ref p) if p.path.segments.last()
				.map(|s| s.ident == "Option")
				.unwrap_or(false))
	}

	/// Returns true if the field is required.
	pub fn is_required(&self) -> bool {
		self.is_optional() == false && self.default_attr().is_none()
	}

	pub fn default_attr(&self) -> Option<&AttributeItem> {
		self.attributes.get("default")
	}
}
