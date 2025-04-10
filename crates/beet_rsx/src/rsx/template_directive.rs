/// Attributes with a colon `:` are considered special template directives,
/// for example `client:load`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateDirective {
	/// A node with a client directive: <div client:load />
	ClientLoad,
	/// A node with a local scope directive: <div scope:local />
	ScopeLocal,
	/// A node with a global scope directive: <div scope:global />
	ScopeGlobal,
	/// A node with a cascade scope directive: <div scope:cascade />
	ScopeCascade,
	/// A node with a slot directive: <div slot="foo" />
	Slot(String),
	/// A node with a runtime directive: <div runtime:bevy />
	Runtime(String),
	// A node with an fs source directive: <div src="foo" />
	// By default this is any src attribute starting wth a `.`
	FsSrc(String),
	/// A node with a custom directive: <div custom:foo="bar" />
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
	fn is_cascade_scope(&self) -> bool {
		matches!(self, TemplateDirective::ScopeCascade)
	}
	fn find_directive(
		&self,
		func: impl Fn(&TemplateDirective) -> bool,
	) -> Option<&TemplateDirective> {
		if func(self) { Some(self) } else { None }
	}

	fn find_map_directive<T>(
		&self,
		func: impl Fn(&TemplateDirective) -> Option<&T>,
	) -> Option<&T> {
		func(self)
	}
}
/// Trait that also allows calling the methods on a vector of template directives
/// like in [`NodeMeta`]
pub trait TemplateDirectiveExt {
	/// Check if the template directive is a client directive
	fn is_client_reactive(&self) -> bool {
		// Check if the template directive is a client directive
		self.any_directive(|d| d.is_client_reactive())
	}
	/// Check if the template directive is a local scope directive
	fn is_local_scope(&self) -> bool {
		self.any_directive(|d| d.is_local_scope())
	}
	/// Check if the template directive is a global scope directive
	fn is_global_scope(&self) -> bool {
		self.any_directive(|d| d.is_global_scope())
	}
	/// Check if the template directive is a cascade scope directive
	fn is_cascade_scope(&self) -> bool {
		self.any_directive(|d| d.is_cascade_scope())
	}

	fn slot_directive(&self) -> Option<&String> {
		self.find_map_directive(|d| match d {
			TemplateDirective::Slot(slot) => Some(slot),
			_ => None,
		})
	}

	fn src_directive(&self) -> Option<&String> {
		self.find_map_directive(|d| match d {
			TemplateDirective::FsSrc(src) => Some(src),
			_ => None,
		})
	}

	fn any_directive(&self, func: impl Fn(&TemplateDirective) -> bool) -> bool {
		self.find_directive(func).is_some()
	}

	fn find_directive(
		&self,
		func: impl Fn(&TemplateDirective) -> bool,
	) -> Option<&TemplateDirective>;
	fn find_map_directive<T>(
		&self,
		func: impl Fn(&TemplateDirective) -> Option<&T>,
	) -> Option<&T>;
}
