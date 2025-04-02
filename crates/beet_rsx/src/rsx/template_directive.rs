/// Attributes with a colon `:` are considered special template directives,
/// for example `client:load`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateDirective {
	ClientLoad,
	ScopeLocal,
	ScopeGlobal,
	Runtime(String),
	FsSrc,
	Custom {
		/// The part before the colon
		prefix: String,
		/// The part after the colon
		suffix: String,
		/// The part after the equals sign, if any
		value: Option<String>,
	},
}

impl TemplateDirective {
	/// Create a new template directive
	/// ## Panics
	/// If the key does not contain two parts split by a colon
	pub fn parse_custom(key: &str, value: Option<&str>) -> Self {
		let mut parts = key.split(':');
		let prefix = parts
			.next()
			.expect("expected colon prefix in template directive");
		let suffix = parts
			.next()
			.expect("expected colon suffix in template directive");
		Self::Custom {
			prefix: prefix.into(),
			suffix: suffix.into(),
			value: value.map(|v| v.into()),
		}
	}
}

impl TemplateDirectiveExt for TemplateDirective {
	fn is_client_reactive(&self) -> bool {
		matches!(self, TemplateDirective::ClientLoad)
	}

	fn is_local_scope(&self) -> bool {
		matches!(self, TemplateDirective::ScopeLocal)
	}

	fn is_global_scope(&self) -> bool {
		matches!(self, TemplateDirective::ScopeGlobal)
	}
}

pub trait TemplateDirectiveExt {
	/// Check if the template directive is a client directive
	fn is_client_reactive(&self) -> bool;
	/// Check if the template directive is a local scope directive
	fn is_local_scope(&self) -> bool;
	/// Check if the template directive is a global scope directive
	fn is_global_scope(&self) -> bool;
}
