use crate::prelude::*;
use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;
use std::convert::Infallible;
use sweet::prelude::Pipeline;
use syn::Expr;

#[derive(Default)]
pub struct ApplyDefaultTemplateDirectives;


impl<T: RsxNodeTokensVisitor<Infallible>> Pipeline<T, T>
	for ApplyDefaultTemplateDirectives
{
	fn apply(self, mut node: T) -> T {
		node.walk_rsx_tokens(parse_node).ok();
		node
	}
}

fn parse_node(
	RsxNodeTokens {
		attributes,
		directives,
		..
	}: &mut RsxNodeTokens,
) -> Result<(), Infallible> {
	attributes.retain(|attr| {
		if let Some(directive) = TemplateDirectiveTokens::from_attr(attr) {
			directives.push(directive);
			return false;
		}
		true
	});
	Ok(())
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateDirectiveTokens {
	ClientLoad,
	ScopeLocal,
	ScopeGlobal,
	ScopeCascade,
	FsSrc(String),
	Slot(String),
	Runtime(String),
	CustomKey(NameExpr),
	CustomKeyValue { key: NameExpr, value: Expr },
}


impl TemplateDirectiveTokens {
	pub fn from_attr(
		attr: &RsxAttributeTokens,
	) -> Option<TemplateDirectiveTokens> {
		match attr {
			RsxAttributeTokens::Key { key } => match key.to_string().as_str() {
				"client:load" => Some(TemplateDirectiveTokens::ClientLoad),
				"scope:local" => Some(TemplateDirectiveTokens::ScopeLocal),
				"scope:global" => Some(TemplateDirectiveTokens::ScopeGlobal),
				"scope:cascade" => Some(TemplateDirectiveTokens::ScopeCascade),
				runtime if runtime.starts_with("runtime:") => {
					let Some(suffix) = runtime.split(':').nth(1) else {
						return None;
					};
					return Some(TemplateDirectiveTokens::Runtime(
						suffix.to_string(),
					));
				}
				_other => None,
			},
			RsxAttributeTokens::KeyValue { key, value }
				if let Some(value) = value.try_lit_str() =>
			{
				match key.to_string().as_str() {
					"slot" => Some(TemplateDirectiveTokens::Slot(value)),
					"src" if value.starts_with('.') => {
						Some(TemplateDirectiveTokens::FsSrc(value))
						// alternatively we could use an ignore approach
						// if ["/", "http://", "https://"]
						// .iter()
						// .all(|p| val.starts_with(p) == false)
					}
					_ => None,
				}
			}
			_ => None,
		}
		// TODO custom directives
		// RsxAttributeTokens::Key { key }
		// 	if let NameExpr::LitStr(key) = key =>
		// {
		// 	None
		// }
		// RsxAttributeTokens::KeyValue { key, value }
		// 	if let NameExpr::LitStr(key) = key =>
		// {
		// 	None
		// }
		// match attr_key_str.as_str() {
		// 	other => {
		// 		match other.contains(":") {
		// 			// its a client directive
		// 			true => {
		// 				let prefix =
		// 					other.split(':').next().unwrap().to_string();

		// 				let suffix =
		// 					other.split(':').nth(1).unwrap().to_string();

		// 				if prefix == "runtime" {
		// 					return Some(TemplateDirectiveTokens::Runtime(
		// 						suffix,
		// 					));
		// 				}
		// 				None
		// 			}
		// 			// its a prop assignemnt
		// 			false => None,
		// 		}
		// 	}
		// }
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
	pub fn build(location: TokenStream) -> TokenStream {
		quote! {RsxNodeMeta {
			template_directives: Vec::new(),
			location: #location
		}}
	}


	pub fn build_with_directives(
		location: TokenStream,
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
				TemplateDirectiveTokens::ScopeCascade => {
					quote! {TemplateDirective::ScopeCascade}
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
				location: #location
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
				TemplateDirectiveTokens::ScopeCascade => {
					quote! {ScopeCascade}
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
