use super::RsxContext;
use super::RsxNode;
use crate::prelude::*;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use parcel_selectors::attr::AttrSelectorOperator;
use parcel_selectors::attr::ParsedCaseSensitivity;

/// ScopedStyle is a utility for applying scoped styles to components.
/// The approach is inspired by astro https://docs.astro.build/en/guides/styling/
/// In --release the css will be minified
pub struct ScopedStyle {
	/// the attribute to use as a selector for the component,
	/// defaults to "data-bcid"
	attr: String,
}

impl Default for ScopedStyle {
	fn default() -> Self {
		ScopedStyle {
			attr: "data-bcid".to_string(),
		}
	}
}


impl ScopedStyle {
	/// apply scoped style to all child components,
	/// the root is only applied if it is a component,
	/// so style tags in a root fragment will not be scoped.
	pub fn apply(&self, node: &mut RsxNode) -> ParseResult<()> {
		let mut result = Ok(());
		RsxContext::visit_mut(node, |cx, node| match node {
			RsxNode::Component { node, .. } => {
				let mut contains_style = false;
				node.visit_ignore_components_mut(|node| {
					if let RsxNode::Element(e) = node {
						if e.tag == "style" {
							contains_style = true;
							node.visit_mut(|node| {
								if let RsxNode::Text(text) = node {
									if let Err(err) = self
										.apply_styles(cx.component_idx(), text)
									{
										result = Err(err);
									}
								}
							});
						}
					}
				});
				if contains_style {
					node.visit_ignore_components_mut(|node| {
						if let RsxNode::Element(e) = node {
							if e.tag != "style" {
								e.attributes.push(RsxAttribute::KeyValue {
									key: self.attr.to_string(),
									value: cx.component_idx().to_string(),
								});
							}
						}
					});
				}
			}
			_ => {}
		});
		result
	}

	/// apply [data-bcid=cid] attribute selector to all style rules in the CSS
	fn apply_styles(&self, cid: usize, css: &mut String) -> ParseResult<()> {
		// Parse the stylesheet
		let mut stylesheet = StyleSheet::parse(css, ParserOptions::default())
			.map_err(|e| ParseError::Serde(e.to_string()))?;

		stylesheet.rules.0.iter_mut().for_each(|rule| {
			// we only care about style rules
			if let lightningcss::rules::CssRule::Style(style_rule) = rule {
				style_rule.selectors.0.iter_mut().for_each(|selector| {
					selector.append(
						lightningcss::selector::Component::AttributeInNoNamespace {
							local_name: self.attr.clone().into(),
							operator: AttrSelectorOperator::Equal,
							value: cid.to_string().into(),
							case_sensitivity:
								ParsedCaseSensitivity::CaseSensitive,
							never_matches: false,
						},
					);
				});
			}
		});

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
	use crate::prelude::*;
	use sweet::prelude::*;

	struct Child;

	impl Component for Child {
		fn render(self) -> impl Rsx {
			rsx! {<div><slot/></div>}
		}
	}


	#[test]
	fn ignores_root() {
		let mut node = rsx! {
			<div>
				<style>span { color: red; }</style>
				<Child/>
			</div>
		};
		ScopedStyle::default().apply(&mut node).unwrap();
		let html = RsxToHtml::render_body(&node);
		expect(html)
			.to_be("<div><style>span { color: red; }</style><div></div></div>");
	}
	#[test]
	fn applies_to_component_not_children() {
		let mut node = rsx! {
			<Child>
				<br/>
				<style>span { color: red; }</style>
				<Child>
					<br/>
				</Child>
			</Child>
		};
		ScopedStyle::default().apply(&mut node).unwrap();
		let html = RsxToHtml::render_body(&node);
		expect(html)
			.to_be("<div data-bcid=\"0\"><br data-bcid=\"0\"/><style>span[data-bcid=\"0\"] {\n  color: red;\n}\n</style><div><br/></div></div>");
	}
}
