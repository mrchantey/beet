use crate::prelude::*;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use rstml::node::KeyedAttribute;
use rstml::node::NodeAttribute;
use syn::Expr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateDirectiveTokens {
	ClientLoad,
	ScopeLocal,
	ScopeGlobal,
	FsSrc(String),
	Slot(String),
	Runtime(String),
	CustomKey(NameExpr),
	CustomKeyValue { key: NameExpr, value: Expr },
}


fn str_lit_val(attr: &KeyedAttribute) -> Option<String> {
	if let Some(Expr::Lit(exp)) = attr.value() {
		match exp.lit {
			syn::Lit::Str(ref lit) => {
				return Some(lit.value());
			}
			_ => {}
		}
	}
	None
}

impl TemplateDirectiveTokens {
	pub fn from_attr(attr: &NodeAttribute) -> Option<TemplateDirectiveTokens> {
		let NodeAttribute::Attribute(keyed_attr) = attr else {
			return None;
		};
		let attr_key_str = keyed_attr.key.to_string();
		match attr_key_str.as_str() {
			"client:load" => Some(TemplateDirectiveTokens::ClientLoad),
			"scope:local" => Some(TemplateDirectiveTokens::ScopeLocal),
			"scope:global" => Some(TemplateDirectiveTokens::ScopeGlobal),
			"slot" => {
				if let Some(val) = str_lit_val(keyed_attr) {
					return Some(TemplateDirectiveTokens::Slot(val));
				}
				None
			}
			"src" => {
				if let Some(val) = str_lit_val(keyed_attr) {
					// alternatively we could use an ignore approach
					// if ["/", "http://", "https://"]
					// .iter()
					// .all(|p| val.starts_with(p) == false)
					if val.starts_with('.') {
						return Some(TemplateDirectiveTokens::FsSrc(val));
					}
				}
				None
			}
			other => {
				match other.contains(":") {
					// its a client directive
					true => {
						let prefix =
							other.split(':').next().unwrap().to_string();

						let suffix =
							other.split(':').nth(1).unwrap().to_string();

						if prefix == "runtime" {
							return Some(TemplateDirectiveTokens::Runtime(
								suffix,
							));
						}
						None
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
			TemplateDirectiveTokens::ClientLoad => true,
			// TemplateDirectiveTokens::Custom { prefix, .. } => {
			// 	prefix == "client"
			// }
			_ => false,
		}
	}
}



/// Builds a [`NodeMeta`] struct in rust or ron syntax
pub struct MetaBuilder;




impl MetaBuilder {
	pub fn parse_attributes(
		attributes: Vec<NodeAttribute>,
	) -> (Vec<TemplateDirectiveTokens>, Vec<NodeAttribute>) {
		attributes.into_iter().partition_map(|attr| {
			match TemplateDirectiveTokens::from_attr(&attr) {
				Some(directive) => itertools::Either::Left(directive),
				None => itertools::Either::Right(attr),
			}
		})
	}


	pub fn build_with_directives(
		template_directives: &[TemplateDirectiveTokens],
	) -> TokenStream {
		let template_directives = template_directives
			.iter()
			.map(|directive| match directive {
				TemplateDirectiveTokens::ClientLoad => {
					quote! {TemplateDirective::ClientLoad}
				}
				TemplateDirectiveTokens::ScopeLocal => {
					quote! {TemplateDirective::ScopeLocal}
				}
				TemplateDirectiveTokens::ScopeGlobal => {
					quote! {TemplateDirective::ScopeGlobal}
				}
				TemplateDirectiveTokens::FsSrc(src) => {
					quote! {TemplateDirective::FsSrc(#src.into())}
				}
				TemplateDirectiveTokens::Slot(slot) => {
					quote! {TemplateDirective::Slot(#slot.into())}
				}
				TemplateDirectiveTokens::Runtime(runtime) => {
					quote! {TemplateDirective::Runtime(#runtime.into())}
				}
				TemplateDirectiveTokens::CustomKey(key) => {
					quote! {TemplateDirective::CustomKey(#key.into())}
				}
				TemplateDirectiveTokens::CustomKeyValue { key, value } => {
					quote! {TemplateDirective::CustomKeyValue{
						key: #key.into(),
						value: #value.into()
					}}
				} // TemplateDirectiveTokens::Custom {
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
		directives: &[TemplateDirectiveTokens],
	) -> TokenStream {
		let template_directives = directives
			.iter()
			.map(|directive| match directive {
				TemplateDirectiveTokens::ClientLoad => {
					quote! {ClientLoad}
				}
				TemplateDirectiveTokens::ScopeLocal => {
					quote! {ScopeLocal}
				}
				TemplateDirectiveTokens::ScopeGlobal => {
					quote! {ScopeGlobal}
				}
				TemplateDirectiveTokens::FsSrc(src) => {
					quote! {FsSrc(#src)}
				}
				TemplateDirectiveTokens::Slot(slot) => {
					quote! {Slot(#slot)}
				}
				TemplateDirectiveTokens::Runtime(runtime) => {
					quote! {Runtime(#runtime)}
				}
				TemplateDirectiveTokens::CustomKey(key) => {
					quote! {CustomKey(#key)}
				}
				TemplateDirectiveTokens::CustomKeyValue { key, value } => {
					quote! {CustomKeyValue(
						key: #key,
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
