/// Template directives contain instructions for various stages of a beet
/// pipeline. Some the syntax of a colon, ie `<div client:load />`, and
/// some are more nuanced, for example a script with a src attribute that
/// starts with a `.` is a file source directive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateDirective {
	/// Indicates that a component should be rendered in html, and also
	/// hydrated on the client. This is the `client islands architecture` used
	/// by frameworks like astro.
	/// ## Example
	/// ```rust ignore
	/// <div client:load />
	/// ```
	ClientLoad,
	/// The default scope for a style tag, its styles will only be applied to
	/// elements within the component, each selector will be preprended with
	/// an attribute selector for the component, eg `[data-styleid-1]`.
	/// ## Example
	/// ```rust ignore
	/// <style scope:local>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	ScopeLocal,
	/// Global scope for a style tag, its styles will not have an attribute
	/// selector prepended to them, so will apply to all elements in the document.
	/// ## Example
	/// ```rust ignore
	/// <style scope:global>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	ScopeGlobal,
	/// Mark a *component* as allowing styles to cascasde into it. This means that
	/// it will have the `data-styleid` attribute applied for each style tag in
	/// its parent.
	/// This behavior is *recursive*, meaning that its children will also
	/// have the attribute applied.
	/// ## Example
	/// ```rust ignore
	/// <MyComponent scope:cascade>
	/// <style>
	/// 	/* this css will also be applied to children of MyComponent */
	/// </style>
	/// ```
	ScopeCascade,
	/// This directive is applied to style tags that have had their content removed.
	/// The `content_hash` is used to retrieve the styleid when resolving scoped styles.
	/// ## Example
	/// ```rust ignore
	/// // before
	/// <style>
	/// 	div { color: blue; }
	/// </style>
	/// // after, this wont be rendered but conceptually this is what happens
	/// <style style:content-hash="1234567890" />
	/// ```
	StylePlaceholder {
		/// A rapidhash of the inner text of the style tag.
		content_hash: u64,
	},
	/// Indicates this node should be rendered in a named slot instead of
	/// the default slot.
	/// ## Example
	/// ```rust ignore
	/// <div slot="header" />
	/// ````
	Slot(String),
	/// Sets the runtime for the parser.
	/// ## Example
	/// ```rust ignore
	/// <div runtime:bevy />
	/// ````
	Runtime(String),
	// A node with an fs source directive: <div src="foo" />
	// By default this is any src attribute starting wth a `.`
	/// ## Example
	/// ```rust ignore
	/// <style src="./style.css" />
	/// <script src="./my-script.js" />
	/// ```
	FsSrc(String),
	/// A custom directive used by a pipeline defined by the user.
	/// ## Example
	/// ```rust ignore
	/// <div custom:foo="bar" />
	/// ```
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
	/// which means the RsxComponent should be serialized, ie `ClientLoad`
	/// This must match TemplateDirective::is_client_reactive
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


	fn runtime(&self) -> Option<&String> {
		self.find_map_directive(|d| match d {
			TemplateDirective::Runtime(runtime) => Some(runtime),
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

#[cfg(feature = "tokens")]
use quote::quote;

#[cfg(feature = "tokens")]
impl crate::prelude::SerdeTokens for TemplateDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			TemplateDirective::ClientLoad => {
				quote! {TemplateDirective::ClientLoad}
			}
			TemplateDirective::ScopeLocal => {
				quote! {TemplateDirective::ScopeLocal}
			}
			TemplateDirective::ScopeGlobal => {
				quote! {TemplateDirective::ScopeGlobal}
			}
			TemplateDirective::ScopeCascade => {
				quote! {TemplateDirective::ScopeCascade}
			}
			TemplateDirective::StylePlaceholder { content_hash } => {
				quote! {TemplateDirective::StylePlaceholder{content_hash: #content_hash}}
			}
			TemplateDirective::FsSrc(src) => {
				quote! {TemplateDirective::FsSrc(#src.into())}
			}
			TemplateDirective::Slot(slot) => {
				quote! {TemplateDirective::Slot(#slot.into())}
			}
			TemplateDirective::Runtime(runtime) => {
				quote! {TemplateDirective::Runtime(#runtime.into())}
			}
			TemplateDirective::Custom {
				prefix,
				suffix,
				value,
			} => {
				quote! {TemplateDirective::Custom{
					prefix: #prefix.into(),
					suffix: #suffix.into(),
					value: #value.into()
				}}
			} // TemplateDirective::Custom {
			  // 	prefix,
			  // 	suffix,
			  // 	value,
			  // } => {
			  // 	let value = match value {
			  // 		Some(value) => quote! {Some(#value.into())},
			  // 		None => quote! {None},
			  // 	};
			  // 	quote! {TemplateDirective::Custom{
			  // 		prefix: #prefix.into(),
			  // 		suffix: #suffix.into(),
			  // 		value: #value
			  // 	}
			  // 	}
			  // }
		}
	}

	fn into_ron_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			TemplateDirective::ClientLoad => {
				quote! {ClientLoad}
			}
			TemplateDirective::ScopeLocal => {
				quote! {ScopeLocal}
			}
			TemplateDirective::ScopeGlobal => {
				quote! {ScopeGlobal}
			}
			TemplateDirective::ScopeCascade => {
				quote! {ScopeCascade}
			}
			TemplateDirective::StylePlaceholder { content_hash } => {
				let content_hash =
					proc_macro2::Literal::u64_unsuffixed(*content_hash);
				quote! {StylePlaceholder(
					content_hash: #content_hash
				)}
			}
			TemplateDirective::FsSrc(src) => {
				quote! {FsSrc(#src)}
			}
			TemplateDirective::Slot(slot) => {
				quote! {Slot(#slot)}
			}
			TemplateDirective::Runtime(runtime) => {
				quote! {Runtime(#runtime)}
			}
			TemplateDirective::Custom {
				prefix,
				suffix,
				value,
			} => {
				let value = match value {
					Some(value) => quote! {Some(#value)},
					None => quote! {None},
				};
				quote! {Custom(
					prefix: #prefix,
					suffix: #suffix,
					value: #value
				)
				}
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	#[cfg(feature = "tokens")]
	fn ron_serde() {
		let directives = vec![
			TemplateDirective::ClientLoad,
			TemplateDirective::ScopeLocal,
			TemplateDirective::ScopeGlobal,
			TemplateDirective::ScopeCascade,
			TemplateDirective::StylePlaceholder { content_hash: 1 },
			TemplateDirective::FsSrc("foo".into()),
			TemplateDirective::Slot("bar".into()),
			TemplateDirective::Runtime("baz".into()),
			TemplateDirective::Custom {
				prefix: "foo".into(),
				suffix: "".into(),
				value: None,
			},
			TemplateDirective::Custom {
				prefix: "foo".into(),
				suffix: "bar".into(),
				value: Some("baz".into()),
			},
		];
		let tokens = directives
			.iter()
			.map(|d| d.into_ron_tokens().to_string())
			.collect::<Vec<_>>()
			.join(", ");
		let directives2 =
			ron::de::from_str::<Vec<TemplateDirective>>(&format!("[{tokens}]"))
				.unwrap();
		expect(directives2).to_be(directives);
	}
}
