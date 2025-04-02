use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use rstml::node::NodeAttribute;
use syn::Expr;


pub(crate) enum ParsedTemplateDirective {
	ClientLoad,
	ScopeLocal,
	ScopeGlobal,
	Runtime(String),
	Custom {
		/// The part before the colon
		prefix: String,
		/// The part after the colon
		suffix: String,
		/// The part after the equals sign, if any
		value: Option<Expr>,
	},
}


impl ParsedTemplateDirective {
	pub fn from_attr(attr: &NodeAttribute) -> Option<ParsedTemplateDirective> {
		let NodeAttribute::Attribute(keyed_attr) = attr else {
			return None;
		};
		let attr_key_str = keyed_attr.key.to_string();
		match attr_key_str.as_str() {
			"client:load" => Some(ParsedTemplateDirective::ClientLoad),
			"scope:local" => Some(ParsedTemplateDirective::ScopeLocal),
			"scope:global" => Some(ParsedTemplateDirective::ScopeGlobal),
			other => {
				match other.contains(":") {
					// its a client directive
					true => {
						let prefix =
							other.split(':').next().unwrap().to_string();

						let suffix =
							other.split(':').nth(1).unwrap().to_string();

						if prefix == "runtime" {
							return Some(ParsedTemplateDirective::Runtime(
								suffix,
							));
						}


						let value = keyed_attr.value();
						Some(ParsedTemplateDirective::Custom {
							prefix,
							suffix,
							value: value.map(|v| v.clone()),
						})
					}
					// its a prop assignemnt
					false => None,
				}
			}
		}
	}

	/// Check if this is a client directive that means the
	/// RsxComponent should be serialized, ie `ClientLoad`
	/// This must match TemplateDirective::is_client_reactive
	pub fn is_client_reactive(&self) -> bool {
		match self {
			ParsedTemplateDirective::ClientLoad => true,
			ParsedTemplateDirective::Custom { prefix, .. } => {
				prefix == "client"
			}
			_ => false,
		}
	}
}



/// Builds a [`NodeMeta`] struct in rust or ron syntax
pub struct MetaBuilder;




impl MetaBuilder {
	pub fn parse_attributes(
		attributes: &[NodeAttribute],
	) -> (Vec<ParsedTemplateDirective>, Vec<&NodeAttribute>) {
		attributes.iter().partition_map(|attr| {
			match ParsedTemplateDirective::from_attr(attr) {
				Some(directive) => itertools::Either::Left(directive),
				None => itertools::Either::Right(attr),
			}
		})
	}


	pub fn build_with_directives(
		template_directives: &[ParsedTemplateDirective],
	) -> TokenStream {
		let template_directives = template_directives
			.iter()
			.map(|directive| match directive {
				ParsedTemplateDirective::ClientLoad => {
					quote! {TemplateDirective::ClientLoad}
				}
				ParsedTemplateDirective::ScopeLocal => {
					quote! {TemplateDirective::ScopeLocal}
				}
				ParsedTemplateDirective::ScopeGlobal => {
					quote! {TemplateDirective::ScopeGlobal}
				}
				ParsedTemplateDirective::Runtime(runtime) => {
					quote! {TemplateDirective::Runtime(#runtime.into())}
				}
				ParsedTemplateDirective::Custom {
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
					}
					}
				}
			})
			.collect::<Vec<_>>();
		quote! {
			RsxNodeMeta {
				template_directives: vec![#(#template_directives),*],
				location: None
			}
		}
	}

	pub fn build_ron(location: TokenStream) -> TokenStream {
		quote! {RsxNodeMeta(
			template_directives: [],
			location: #location
		)}
	}

	pub fn build_ron_with_directives(
		location: TokenStream,
		directives: &[ParsedTemplateDirective],
	) -> TokenStream {
		let template_directives = directives
			.iter()
			.map(|directive| match directive {
				ParsedTemplateDirective::ClientLoad => {
					quote! {ClientLoad}
				}
				ParsedTemplateDirective::ScopeLocal => {
					quote! {ScopeLocal}
				}
				ParsedTemplateDirective::ScopeGlobal => {
					quote! {ScopeGlobal}
				}
				ParsedTemplateDirective::Runtime(runtime) => {
					quote! {Runtime(#runtime)}
				}
				ParsedTemplateDirective::Custom {
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
			})
			.collect::<Vec<_>>();
		quote! {RsxNodeMeta(
			template_directives: [#(#template_directives),*],
			location: #location
		)}
	}
}
