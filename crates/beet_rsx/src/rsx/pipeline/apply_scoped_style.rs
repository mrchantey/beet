use crate::prelude::*;
use anyhow::Result;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use parcel_selectors::attr::AttrSelectorOperator;
use parcel_selectors::attr::ParsedCaseSensitivity;

/// ScopedStyle is a utility for applying scoped styles to components.
/// The approach is inspired by astro https://docs.astro.build/en/guides/styling/
///
/// # Scoped Style Rules:
///
/// - Style tags are applied to all elements in a root or component
///   but not [RsxComponent::node] or [RsxComponent::slot_children]
/// - In release mode the css will be minified
/// - Scope rules:
/// 	- `<style scope:global/>` will not be scoped at all
///
pub struct ApplyScopedStyle {
	/// the attribute to use as a selector for the component,
	/// defaults to "data-styleid"
	attr: String,
	/// an index used to track the current component being styled
	/// TODO use treeidx
	idx: usize,
}

impl Default for ApplyScopedStyle {
	fn default() -> Self {
		ApplyScopedStyle {
			attr: "data-styleid".to_string(),
			idx: 0,
		}
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Scope {
	#[default]
	Component,
	Global,
	// Cascade (eargerly apply slots?)
}

impl Pipeline<RsxNode, Result<RsxNode>> for ApplyScopedStyle {
	/// Applies scoped style to:
	/// 1. root node
	/// 2. all component nodes
	/// 3. all component slot children
	fn apply(mut self, mut node: RsxNode) -> Result<RsxNode> {
		// 1. apply to the root node, if its a component nothing happens
		//    in this step, it will be handled by the component visitor
		self.apply_node(&mut node)?;
		let mut parse_err = Ok(());

		// visit all components
		VisitRsxComponentMut::walk(&mut node, |component| {
			// 2. apply to component node
			if let Err(err) = self.apply_node(&mut component.node) {
				parse_err = Err(err);
			};
		});
		parse_err.map(|_| node).map_err(|e| anyhow::anyhow!(e))
	}
}

impl ApplyScopedStyle {
	/// 1. apply the idx to all style bodies
	/// 2. if contains style, apply tag to all elements in the component
	fn apply_node(&mut self, node: &mut RsxNode) -> ParseResult<()> {
		let mut parse_err = Ok(());

		// the boundary for scoped style is to apply to every descendent
		// with the exception of component nodes
		let opts = VisitRsxOptions::ignore_component_node();
		let mut component_scope_found = false;

		// 1. apply to style bodies
		VisitRsxElementMut::walk_with_opts(node, opts.clone(), |el| {
			if el.tag == "style" {
				let scope = match el.is_global_scope() {
					true => Scope::Global,
					false => Scope::Component,
				};
				if scope == Scope::Component {
					component_scope_found = true;
				}
				// currently only recurse top level style children, we could create another
				// visitor to go deeper if we start supporting style body components
				match &mut *el.children {
					RsxNode::Text(text) => {
						// this is a hack to allow for the css unit "em" to be used in the style tag
						// we should put it somewhere else
						text.value = text.value.replace(".em", "em");
						if let Err(err) =
							self.apply_styles(&mut text.value, scope)
						{
							parse_err = Err(err);
						}
					}
					RsxNode::Fragment(fragment) => {
						if !fragment.nodes.is_empty() {
							parse_err = Err(ParseError::Serde(format!(
								"ScopedStyle: Expected Text Node, received Fragment with {} nodes",
								fragment.nodes.len()
							)));
						}
					}
					other => {
						parse_err = Err(ParseError::Serde(format!(
							"ScopedStyle: Expected Text Node, received {:#?}",
							other
						)));
					}
				}
			}
		});
		// 2. tag elements if *any* component scoped styles were found
		if component_scope_found {
			VisitRsxElementMut::walk_with_opts(node, opts.clone(), |el| {
				el.attributes.push(RsxAttribute::KeyValue {
					key: self.attr.to_string(),
					value: self.idx.to_string(),
				});
			});
			self.idx += 1;
		}
		parse_err
	}
	fn apply_styles(&self, css: &mut String, scope: Scope) -> ParseResult<()> {
		// Parse the stylesheet
		let mut stylesheet = StyleSheet::parse(css, ParserOptions::default())
			.map_err(|e| ParseError::Serde(e.to_string()))?;

		if scope == Scope::Component {
			stylesheet.rules.0.iter_mut().for_each(|rule| {
				// we only care about style rules
				if let lightningcss::rules::CssRule::Style(style_rule) = rule {
					style_rule.selectors.0.iter_mut().for_each(|selector| {
						selector.append(
						lightningcss::selector::Component::AttributeInNoNamespace {
							local_name: self.attr.clone().into(),
							operator: AttrSelectorOperator::Equal,
							value: self.idx.to_string().into(),
							case_sensitivity:
								ParsedCaseSensitivity::CaseSensitive,
							never_matches: false,
						},
					);
					});
				}
			});
		}

		#[cfg(debug_assertions)]
		let options = PrinterOptions::default();
		#[cfg(not(debug_assertions))]
		let options = PrinterOptions {
			minify: true,
			..Default::default()
		};

		let new_css = stylesheet
			.to_css(options)
			.map_err(|e| ParseError::Serde(e.to_string()))?
			.code;
		drop(stylesheet);
		*css = new_css;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Node)]
	struct JustSlot;

	fn just_slot(_: JustSlot) -> RsxNode {
		rsx! { <slot /> }
	}
	#[derive(Node)]
	struct Child;

	fn child(_: Child) -> RsxNode {
		rsx! {
			<div>
				<style>span { color: blue; }</style>
				<slot />
			</div>
		}
	}

	#[test]
	fn applies_to_root() {
		expect(
			rsx! {
				<div>
					<style>span { color: red; }</style>
					// <Child/>
				</div>
			}
			.xpipe(RsxToHtmlString::default()).unwrap(),
		)
		.to_be("<div data-styleid=\"0\"><style data-styleid=\"0\">span[data-styleid=\"0\"] {\n  color: red;\n}\n</style></div>");
	}

	#[test]
	fn global_scope() {
		expect(
			rsx! {
				<div>
					<style scope:global>span { color: red; }</style>
					// <Child/>
				</div>
			}
			.xpipe(RsxToHtmlString::default())
			.unwrap(),
		)
		.to_be("<div><style>span {\n  color: red;\n}\n</style></div>");
	}
	#[test]
	fn local_and_global_scope() {
		expect(
			rsx! {
				<div>
					<style>div { color: blue; }</style>
					<style scope:global>span { color: red; }</style>
				</div>
			}
			.xpipe(RsxToHtmlString::default()).unwrap(),
		)
		.to_be("<div data-styleid=\"0\"><style data-styleid=\"0\">div[data-styleid=\"0\"] {\n  color: #00f;\n}\n</style><style data-styleid=\"0\">span {\n  color: red;\n}\n</style></div>");
	}


	#[test]
	fn applies_to_component_node() {
		expect(rsx! { <Child /> }.xpipe(RsxToHtmlString::default()).unwrap())
		.to_be("<div data-styleid=\"0\"><style data-styleid=\"0\">span[data-styleid=\"0\"] {\n  color: #00f;\n}\n</style></div>");
	}
	#[test]
	fn applies_to_nested_component() {
		expect(rsx! {
			<Child>
				<Child />
			</Child>
		}.xpipe(RsxToHtmlString::default()).unwrap())
			.to_be("<div data-styleid=\"0\"><style data-styleid=\"0\">span[data-styleid=\"0\"] {\n  color: #00f;\n}\n</style><div data-styleid=\"1\"><style data-styleid=\"1\">span[data-styleid=\"1\"] {\n  color: #00f;\n}\n</style></div></div>");
	}
	#[test]
	fn applies_to_slot_children() {
		expect(rsx! {
			<JustSlot>
				<br/>
			</JustSlot>
			<style>br { color: red; }</style>
		}.xpipe(RsxToHtmlString::default()).unwrap())
			.to_be("<br data-styleid=\"0\"/><style data-styleid=\"0\">br[data-styleid=\"0\"] {\n  color: red;\n}\n</style>");
	}


	// this is invalid css, wrapping in "1em", we need to unwrap somehow
	#[test]
	fn inner_text() {
		expect(rsx! {
				<style>span { padding: 1.em; }</style>
		}.xpipe(RsxToHtmlString::default()).unwrap())
			.to_be("<style data-styleid=\"0\">span[data-styleid=\"0\"] {\n  padding: 1em;\n}\n</style>");
	}
}
