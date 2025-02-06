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
	/// defaults to "data-styleid"
	attr: String,
	/// an index used to track the current component being styled
	idx: usize,
}

impl Default for ScopedStyle {
	fn default() -> Self {
		ScopedStyle {
			attr: "data-styleid".to_string(),
			idx: 0,
		}
	}
}

impl ScopedStyle {
	pub fn apply(&mut self, node: &mut RsxNode) -> ParseResult<()> {
		let mut parse_err = Ok(());
		VisitRsxComponentMut::new(|component| {
			let opts = VisitRsxOptions::ignore_component_node();
			let mut contains_style = false;

			VisitRsxElementMut::new_with_options(opts.clone(), |el| {
				if el.tag == "style" {
					contains_style = true;
					// currently only recurse top level, we could create another
					// visitor to go deeper
					for child in &mut el.children {
						if let RsxNode::Text(text) = child {
							if let Err(err) = self.apply_styles(text) {
								parse_err = Err(err);
							}
						}
					}
				}
			})
			.walk_node(&mut component.node);
			if contains_style {
				println!("contains style");
				VisitRsxElementMut::new_with_options(opts.clone(), |el| {
					el.attributes.push(RsxAttribute::KeyValue {
						key: self.attr.to_string(),
						value: self.idx.to_string(),
					});
				})
				.walk_node(&mut component.node);
				self.idx += 1;
			}
		})
		.walk_node(node);
		parse_err
	}
	fn apply_styles(&self, css: &mut String) -> ParseResult<()> {
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
							value: self.idx.to_string().into(),
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

impl RsxVisitorMut for ScopedStyle {
	fn visit_component(&mut self, component: &mut RsxComponent) {}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	struct Child;

	impl Component for Child {
		fn render(self) -> RsxRoot {
			rsx! {
				<div>
					<slot />
				</div>
			}
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
			.to_be("<div data-styleid=\"0\"><br data-styleid=\"0\"/><style data-styleid=\"0\">span[data-styleid=\"0\"] {\n  color: red;\n}\n</style><div><br/></div></div>");
	}
}
