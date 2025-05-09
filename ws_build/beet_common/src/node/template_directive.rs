use super::StyleScope;

/// Template directives contain instructions for various stages of a beet
/// pipeline. Some the syntax of a colon, ie `<div client:load />`, and
/// some are more nuanced, for example a script with a src attribute that
/// starts with a `.` is a file source directive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateDirective {
	/// The entry point for an rsx template. This is set by default on all
	/// root nodes.
	/// ## Example
	/// ```rust ignore
	/// <div is:template />
	/// ```
	RsxTemplate,
	/// Indicates that a component should be rendered in html, and also
	/// hydrated on the client. This is the `client islands architecture` used
	/// by frameworks like astro.
	/// ## Example
	/// ```rust ignore
	/// <div client:load />
	/// ```
	ClientLoad,
	/// The scope of a style tag, see [`StyleScope`] for more details.
	/// ## Example
	/// ```rust ignore
	/// <style scope:global>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	StyleScope(StyleScope),
	/// Mark a *component* as allowing styles to cascasde into it. This means that
	/// it will have the `data-styleid` attribute applied for each style tag in
	/// its parent.
	/// This behavior is *recursive*, meaning that its children will also
	/// have the attribute applied.
	/// ## Example
	/// ```rust ignore
	/// <MyComponent style:cascade>
	/// <style>
	/// 	/* this css will also be applied to children of MyComponent */
	/// </style>
	/// ```
	StyleCascade,
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
		/// A rapidhash of a [`StyleTemplate`], including:
		/// - the inner text of the style tag.
		/// - its [`StyleScope`] directive.
		/// The hash is used to resolve the style id when rendering.
		content_hash: u64,
	},
	/// This script or style tag should be rendered inline, and not
	/// deduplicated or be used for component scoped styles.
	/// ## Example
	/// ```rust ignore
	/// <style is:inline>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	Inline,
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
	fn find_directive(
		&self,
		func: impl Fn(&TemplateDirective) -> bool,
	) -> Option<&TemplateDirective>;
	fn find_map_directive<T>(
		&self,
		func: impl Fn(&TemplateDirective) -> Option<&T>,
	) -> Option<&T>;

	/// Check if the template directive is a client directive
	/// which means the RsxComponent should be serialized, ie `ClientLoad`
	/// This must match TemplateDirective::is_client_reactive
	fn is_client_reactive(&self) -> bool {
		// Check if the template directive is a client directive
		self.any_directive(|d| matches!(d, TemplateDirective::ClientLoad))
	}
	/// Check if the template directive is a local scope directive
	fn style_scope(&self) -> Option<StyleScope> {
		self.find_map_directive(|d| match d {
			TemplateDirective::StyleScope(scope) => Some(scope),
			_ => None,
		})
		.copied()
	}
	fn is_template(&self) -> bool {
		self.any_directive(|d| matches!(d, TemplateDirective::RsxTemplate))
	}

	/// Check if the template directive is a cascade style directive
	fn is_cascade_style(&self) -> bool {
		self.any_directive(|d| matches!(d, TemplateDirective::StyleCascade))
	}
	fn is_inline(&self) -> bool {
		self.any_directive(|d| matches!(d, TemplateDirective::Inline))
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
	fn style_placeholder(&self) -> Option<u64> {
		self.find_map_directive(|d| match d {
			TemplateDirective::StylePlaceholder { content_hash } => {
				Some(content_hash)
			}
			_ => None,
		})
		.copied()
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
}

#[cfg(feature = "tokens")]
use quote::quote;


#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for TemplateDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			TemplateDirective::RsxTemplate => {
				quote! {TemplateDirective::RsxTemplate}
			}
			TemplateDirective::ClientLoad => {
				quote! {TemplateDirective::ClientLoad}
			}
			TemplateDirective::StyleScope(scope) => {
				let scope = scope.into_rust_tokens();
				quote! {TemplateDirective::StyleScope(#scope)}
			}
			TemplateDirective::StyleCascade => {
				quote! {TemplateDirective::StyleCascade}
			}
			TemplateDirective::StylePlaceholder { content_hash } => {
				quote! {TemplateDirective::StylePlaceholder{content_hash: #content_hash}}
			}
			TemplateDirective::Inline => {
				quote! {TemplateDirective::Inline}
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
				let value = match value {
					Some(value) => quote! {Some(#value.into())},
					None => quote! {None},
				};
				quote! {TemplateDirective::Custom{
					prefix: #prefix.into(),
					suffix: #suffix.into(),
					value: #value
				}}
			}
		}
	}
}

