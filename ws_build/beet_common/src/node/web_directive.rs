/// Template directives related to web rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WebDirective {
	StyleId { id: u64 },
}


#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for WebDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			WebDirective::StyleId { id } => {
				quote::quote! {WebDirective::StyleId{ id: #id }}
			}
		}
	}
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ParseDirectiveError {
	#[error("Failed To Parse Directive: {key}\n{message}")]
	InvalidValue { key: String, message: String },
}

pub type ParseDirectiveResult<T> = std::result::Result<T, ParseDirectiveError>;

impl WebDirective {
	pub fn try_from_attr(
		key: &str,
		value: Option<&str>,
	) -> ParseDirectiveResult<Option<Self>> {
		match (key, value) {
			("style:id", Some(val)) => {
				let id = val.parse::<u64>().map_err(|_| {
					ParseDirectiveError::InvalidValue {
						key: key.to_string(),
						message: format!(
							"Failed to parse style:id value: {}",
							val
						),
					}
				})?;
				Ok(Some(WebDirective::StyleId { id }))
			}
			_ => Ok(None),
		}
	}
}


pub trait WebDirectiveExt {
	fn find_map_web_directive<T>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T>,
	) -> Option<&T>;
	fn style_id(&self) -> Option<u64> {
		self.find_map_web_directive(|d| match d {
			WebDirective::StyleId { id } => Some(id),
		})
		.copied()
	}
}
impl WebDirectiveExt for Vec<WebDirective> {
	fn find_map_web_directive<T>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T>,
	) -> Option<&T> {
		self.iter().find_map(|d| func(d))
	}
}
impl WebDirectiveExt for WebDirective {
	fn find_map_web_directive<T>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T>,
	) -> Option<&T> {
		func(self)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		WebDirective::try_from_attr("style:id", Some("123"))
			.xpect()
			.to_be(Ok(Some(WebDirective::StyleId { id: 123 })));
		WebDirective::try_from_attr("style:id", Some("foobar"))
			.xpect()
			.to_be_err();
	}
}
