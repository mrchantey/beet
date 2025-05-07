use crate::prelude::*;
use anyhow::Result;
use beet_common::prelude::*;
use proc_macro2::TokenStream;
use quote::quote;
use std::convert::Infallible;
use sweet::prelude::Pipeline;


/// For each [`ElementTokens`], read its [`attributes`](ElementTokens::attributes) and extract them
/// into the [`directives`](ElementTokens::directives) field.
#[derive(Default)]
pub struct ExtractTemplateDirectives;

impl<T: ElementTokensVisitor<Infallible>> Pipeline<T, Result<T>>
	for ExtractTemplateDirectives
{
	fn apply(self, mut node: T) -> Result<T> {
		node.walk_rsx_tokens(parse_node)?;
		Ok(node)
	}
}

/// remove template directives from attributes,
/// and add them to the directives field
fn parse_node(
	ElementTokens {
		attributes,
		directives,
		..
	}: &mut ElementTokens,
) -> Result<(), Infallible> {
	attributes.retain(|attr| {
		if let Some(directive) = attr_to_template_directive(attr) {
			directives.push(directive);
			return false;
		}
		true
	});
	Ok(())
}

fn attr_to_template_directive(
	attr: &RsxAttributeTokens,
) -> Option<TemplateDirective> {
	match attr {
		RsxAttributeTokens::Key { key } => match key.to_string().as_str() {
			"client:load" => Some(TemplateDirective::ClientLoad),
			"scope:local" => Some(TemplateDirective::ScopeLocal),
			"scope:global" => Some(TemplateDirective::ScopeGlobal),
			"scope:cascade" => Some(TemplateDirective::ScopeCascade),
			runtime_key if runtime_key.starts_with("runtime:") => {
				let Some(suffix) = runtime_key.split(':').nth(1) else {
					return None;
				};
				return Some(TemplateDirective::Runtime(suffix.to_string()));
			}
			custom_key if custom_key.contains(':') => {
				let mut parts = custom_key.split(':');
				let prefix = parts.next().unwrap_or_default().to_string();
				let suffix = parts.next().unwrap_or_default().to_string();
				Some(TemplateDirective::Custom {
					prefix,
					suffix,
					value: None,
				})
			}
			_attr => None,
		},
		// only key value pairs where the value is a string are valid
		// templates
		RsxAttributeTokens::KeyValue { key, value }
			if let Some(value) = value.try_lit_str() =>
		{
			match key.to_string().as_str() {
				"slot" => Some(TemplateDirective::Slot(value)),
				"src" if value.starts_with('.') => {
					// alternatively we could use an ignore approach
					// if ["/", "http://", "https://"]
					// .iter()
					// .all(|p| val.starts_with(p) == false)
					Some(TemplateDirective::FsSrc(value))
				}
				custom_key if custom_key.contains(':') => {
					let mut parts = custom_key.split(':');
					let prefix = parts.next().unwrap_or_default().to_string();
					let suffix = parts.next().unwrap_or_default().to_string();
					Some(TemplateDirective::Custom {
						prefix,
						suffix,
						value: Some(value),
					})
				}
				_attr => None,
			}
		}
		_ => None,
	}
}
/// Builds a [`NodeMeta`] struct in rust or ron syntax
pub struct MetaBuilder;


impl MetaBuilder {
	pub fn build(location: TokenStream) -> TokenStream {
		quote! {NodeMeta {
			template_directives: Vec::new(),
			location: #location
		}}
	}

	pub fn build_with_directives(
		location: TokenStream,
		template_directives: &[TemplateDirective],
	) -> TokenStream {
		let template_directives = template_directives
			.iter()
			.map(|directive| directive.into_rust_tokens());
		quote! {
			NodeMeta {
				template_directives: vec![#(#template_directives),*],
				location: #location
			}
		}
	}

	pub fn build_ron(location: TokenStream) -> TokenStream {
		quote! {NodeMeta(
			template_directives: [],
			location: #location
		)}
	}

	pub fn build_ron_with_directives(
		location: TokenStream,
		directives: &[TemplateDirective],
	) -> TokenStream {
		let template_directives = directives
			.iter()
			.map(|directive| directive.into_ron_tokens());
		quote! {NodeMeta(
			template_directives: [#(#template_directives),*],
			location: #location
		)}
	}
}
