use crate::prelude::*;
use bevy::prelude::*;


/// System set in which all template directives are extracted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExtractDirectivesSet;


/// Generic plugin for extracting and propagating directives to tokens.
/// ## Example
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_common::prelude::*;
/// App::new().add_plugins(directive_plugin::<ClientIslandDirective>);
/// ```
pub fn extract_directive_plugin<T: TemplateDirective>(app: &mut App) {
	app.add_systems(
		Update,
		try_extract_directive::<T>.in_set(ExtractDirectivesSet),
	);
}

/// Generic system for extracting a [TemplateDirective] from attributes.
fn try_extract_directive<T: TemplateDirective>(
	mut commands: Commands,
	query: Populated<(Entity, &AttributeOf, &AttributeLit)>,
) {
	for (entity, parent, lit) in query.iter() {
		let (key, value) = lit.into_parts();
		if let Some(directive) = T::try_from_attribute(key, value) {
			commands.entity(**parent).insert(directive);
			commands.entity(entity).despawn();
		}
	}
}


/// DEPRECATED BELOW THIS LINE

/// Template directives contain instructions for various stages of a beet
/// pipeline. Some the syntax of a colon, ie `<div client:load />`, and
/// some are more nuanced, for example a script with a src attribute that
/// starts with a `.` is a file source directive.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateDirectiveEnum {
	/// Indicate this node should be extracted from wherever it is and inserted
	/// in the head of the document.
	Head,
	/// A node which should have a template stored, keyed by this nodes [`FileSpan`].
	/// This is set by default on all root nodes.
	/// ## Example
	/// ```rust ignore
	/// <div is:template />
	/// ```
	NodeTemplate,
	/// This directive is applied to script and style tags that have had their content removed.
	/// The `content_hash` is used to retrieve the styleid when resolving scoped styles.
	/// ## Example
	/// ```rust ignore
	/// // before
	///
	/// <style>
	/// 	div { color: blue; }
	/// </style>
	/// // after, this wont be rendered but conceptually this is what happens
	/// <style placeholder="1234567890" />
	/// ```
	LangTemplate {
		/// A rapidhash of a [`StyleTemplate`], including:
		/// - the inner text of the style tag.
		/// - its [`StyleScope`] directive.
		/// The hash is used to resolve the style id when rendering.
		content_hash: LangContentHash,
	},
	/// Indicates that a component should be rendered in html, and also
	/// hydrated on the client. This is the `client islands architecture` used
	/// by frameworks like astro.
	/// ## Example
	/// ```rust ignore
	/// <div client:load />
	/// ```
	ClientLoad,
	Web(WebDirective),
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


/// Trait for template directives
pub trait TemplateDirective: 'static + Sized + Component {
	/// Try to parse from an attribute key-value pair
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self>;
}



impl TemplateDirectiveEnum {
	pub fn try_from_attr(
		key: &str,
		value: Option<&str>,
	) -> ParseDirectiveResult<Option<Self>> {
		if let Some(directive) = WebDirective::try_from_attr(key, value)? {
			return Ok(Some(directive.into()));
		}

		match (key, value) {
			("is:template", _) => Some(Self::NodeTemplate),
			("client:load", _) => Some(Self::ClientLoad),
			("scope:local", _) => Some(Self::StyleScope(StyleScope::Local)),
			("scope:global", _) => Some(Self::StyleScope(StyleScope::Global)),
			("is:inline", _) => Some(Self::Inline),
			("style:cascade", _) => Some(Self::StyleCascade),
			(runtime_key, _) if runtime_key.starts_with("runtime:") => {
				if let Some(suffix) = runtime_key.split(':').nth(1) {
					Some(Self::Runtime(suffix.to_string()))
				} else {
					None
				}
			}
			("slot", Some(value)) => Some(Self::Slot(value.to_string())),
			("src", Some(value)) if value.starts_with('.') => {
				// alternatively we could use an ignore approach
				// if ["/", "http://", "https://"]
				// .iter()
				// .all(|p| val.starts_with(p) == false)
				Some(Self::FsSrc(value.to_string()))
			}
			(custom_key, custom_value) if custom_key.contains(':') => {
				let mut parts = custom_key.split(':');
				let prefix = parts.next().unwrap_or_default().to_string();
				let suffix = parts.next().unwrap_or_default().to_string();
				Some(Self::Custom {
					prefix,
					suffix,
					value: custom_value.map(|v| v.to_string()),
				})
			}
			_ => None,
		}
		.xok()
	}
}

impl TemplateDirectiveExt for TemplateDirectiveEnum {
	fn find_directive(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> bool,
	) -> Option<&TemplateDirectiveEnum> {
		if func(self) { Some(self) } else { None }
	}

	fn find_map_directive<T>(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> Option<&T>,
	) -> Option<&T> {
		func(self)
	}
}

impl TemplateDirectiveExt for Vec<TemplateDirectiveEnum> {
	fn find_directive(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> bool,
	) -> Option<&TemplateDirectiveEnum> {
		self.iter().find(|d| func(d))
	}

	fn find_map_directive<T>(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> Option<&T>,
	) -> Option<&T> {
		self.iter().find_map(|d| func(d))
	}
}


/// Trait that also allows calling the methods on a vector of template directives
/// like in [`NodeMeta`]
pub trait TemplateDirectiveExt {
	fn find_directive(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> bool,
	) -> Option<&TemplateDirectiveEnum>;
	fn find_map_directive<T>(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> Option<&T>,
	) -> Option<&T>;

	/// Check if the template directive is a client directive
	/// which means the RsxComponent should be serialized, ie `ClientLoad`
	/// This must match TemplateDirective::is_client_reactive
	fn is_client_reactive(&self) -> bool {
		// Check if the template directive is a client directive
		self.any_directive(|d| matches!(d, TemplateDirectiveEnum::ClientLoad))
	}
	/// Check if the template directive is a local scope directive
	fn style_scope(&self) -> Option<StyleScope> {
		self.find_map_directive(|d| match d {
			TemplateDirectiveEnum::StyleScope(scope) => Some(scope),
			_ => None,
		})
		.copied()
	}
	fn is_template(&self) -> bool {
		self.any_directive(|d| matches!(d, TemplateDirectiveEnum::NodeTemplate))
	}

	/// Check if the template directive is a cascade style directive
	fn is_cascade_style(&self) -> bool {
		self.any_directive(|d| matches!(d, TemplateDirectiveEnum::StyleCascade))
	}
	fn is_inline(&self) -> bool {
		self.any_directive(|d| matches!(d, TemplateDirectiveEnum::Inline))
	}

	fn slot_directive(&self) -> Option<&String> {
		self.find_map_directive(|d| match d {
			TemplateDirectiveEnum::Slot(slot) => Some(slot),
			_ => None,
		})
	}

	fn src_directive(&self) -> Option<&String> {
		self.find_map_directive(|d| match d {
			TemplateDirectiveEnum::FsSrc(src) => Some(src),
			_ => None,
		})
	}
	fn lang_template(&self) -> Option<LangContentHash> {
		self.find_map_directive(|d| match d {
			TemplateDirectiveEnum::LangTemplate { content_hash } => {
				Some(content_hash)
			}
			_ => None,
		})
		.copied()
	}


	fn runtime(&self) -> Option<&String> {
		self.find_map_directive(|d| match d {
			TemplateDirectiveEnum::Runtime(runtime) => Some(runtime),
			_ => None,
		})
	}

	fn any_directive(
		&self,
		func: impl Fn(&TemplateDirectiveEnum) -> bool,
	) -> bool {
		self.find_directive(func).is_some()
	}
}

impl<T> WebDirectiveExt for T
where
	T: TemplateDirectiveExt,
{
	fn find_map_web_directive<T2>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T2>,
	) -> Option<&T2> {
		self.find_map_directive(|d| match d {
			TemplateDirectiveEnum::Web(web) => func(web),
			_ => None,
		})
	}
}

impl Into<TemplateDirectiveEnum> for WebDirective {
	fn into(self) -> TemplateDirectiveEnum { TemplateDirectiveEnum::Web(self) }
}

#[cfg(feature = "tokens")]
use quote::quote;
use sweet::prelude::PipelineTarget;


#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for TemplateDirectiveEnum {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			TemplateDirectiveEnum::Head => {
				quote! {TemplateDirective::Head}
			}
			TemplateDirectiveEnum::NodeTemplate => {
				quote! {TemplateDirective::NodeTemplate}
			}
			TemplateDirectiveEnum::ClientLoad => {
				quote! {TemplateDirective::ClientLoad}
			}
			TemplateDirectiveEnum::StyleScope(scope) => {
				let scope = scope.into_rust_tokens();
				quote! {TemplateDirective::StyleScope(#scope)}
			}
			TemplateDirectiveEnum::StyleCascade => {
				quote! {TemplateDirective::StyleCascade}
			}
			TemplateDirectiveEnum::Web(web) => {
				let web = web.into_rust_tokens();
				quote! {TemplateDirective::Web(#web)}
			}
			TemplateDirectiveEnum::LangTemplate { content_hash } => {
				let content_hash = content_hash.into_rust_tokens();
				quote! {TemplateDirective::ContentPlaceholder{
						content_hash: #content_hash
					}
				}
			}
			TemplateDirectiveEnum::Inline => {
				quote! {TemplateDirective::Inline}
			}
			TemplateDirectiveEnum::FsSrc(src) => {
				quote! {TemplateDirective::FsSrc(#src.into())}
			}
			TemplateDirectiveEnum::Slot(slot) => {
				quote! {TemplateDirective::Slot(#slot.into())}
			}
			TemplateDirectiveEnum::Runtime(runtime) => {
				quote! {TemplateDirective::Runtime(#runtime.into())}
			}
			TemplateDirectiveEnum::Custom {
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

// /// Marker directive indicating this node is the root of a template.
// pub struct NodeTemplate;
