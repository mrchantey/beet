use beet_core::prelude::*;
use beet_dom::prelude::*;
use lightningcss::printer::PrinterOptions;
use lightningcss::rules::CssRule;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;

/// Parse css using lightningcss, applying styleid to selectors as required.
pub fn parse_lightning(
	constants: Res<HtmlConstants>,
	mut query: Populated<
		(
			&mut InnerText,
			Option<&StyleScope>,
			Option<&FileSpanOf<ElementNode>>,
		),
		Added<StyleElement>,
	>,
) -> Result {
	query
		.iter_mut()
		.collect::<Vec<_>>()
		.into_iter()
		.map(|(mut text, scope, span)| {
			// Parse the stylesheet
			let mut stylesheet =
				StyleSheet::parse(&text.0, ParserOptions::default()).map_err(
					|e| {
						bevyhow!(
							"Failed to parse css: {}\nSpan: {:?}\nInput: \"{}\"",
							e.to_string(),
							span,
							text.0,
						)
					},
				)?;

			let scope = scope.map(|s| *s).unwrap_or(StyleScope::Local);

			// apply hash as a style id for local styles
			let selector_placeholder =
				constants.style_id_attribute_placeholder();
			if scope == StyleScope::Local {
				apply_recursive(&mut stylesheet.rules.0, selector_placeholder);
			}

			#[cfg(debug_assertions)]
			let options = PrinterOptions::default();
			// minify in release builds
			#[cfg(not(debug_assertions))]
			let options = PrinterOptions {
				minify: true,
				..Default::default()
			};

			let new_css = stylesheet
				.to_css(options)
				.map_err(|e| {
					bevyhow!(
						"Failed to serialize stylesheet: {}\nSpan: {:?}",
						e.to_string(),
						span,
					)
				})?
				.code;
			drop(stylesheet);
			text.0 = new_css;

			Ok(())
		})
		.collect::<Result<Vec<_>>>()?;
	Ok(())
}

/// Recursively apply style ID to all selectors in all rules, including nested ones like media queries
fn apply_recursive<'a>(
	rules: &mut Vec<CssRule<'a>>,
	selector_placeholder: String,
) {
	rules.iter_mut().for_each(|rule| match rule {
		CssRule::Style(style_rule) => {
			style_rule.selectors.0.iter_mut().for_each(|selector| {
				selector.append(
						lightningcss::selector::Component::AttributeInNoNamespaceExists {
							local_name: selector_placeholder.clone().into(),
							local_name_lower: selector_placeholder.clone().into(),
						}
					);
			});
		}
		CssRule::Media(media_rule) => {
			apply_recursive(
				&mut media_rule.rules.0,
				selector_placeholder.clone(),
			);
		}
		CssRule::Supports(supports_rule) => {
			apply_recursive(
				&mut supports_rule.rules.0,
				selector_placeholder.clone(),
			);
		}
		CssRule::Container(container_rule) => {
			apply_recursive(
				&mut container_rule.rules.0,
				selector_placeholder.clone(),
			);
		}
		CssRule::LayerBlock(layer_block_rule) => {
			apply_recursive(
				&mut layer_block_rule.rules.0,
				selector_placeholder.clone(),
			);
		}
		CssRule::Scope(scope_rule) => {
			apply_recursive(
				&mut scope_rule.rules.0,
				selector_placeholder.clone(),
			);
		}
		CssRule::StartingStyle(starting_style_rule) => {
			apply_recursive(
				&mut starting_style_rule.rules.0,
				selector_placeholder.clone(),
			);
		}
		_ => {
			// i think we got em all, open an issue otherwise
		}
	});
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::*;


	fn setup(bundle: impl Bundle) -> InnerText {
		let mut app = App::new();
		app.init_resource::<HtmlConstants>()
			.add_systems(Update, parse_lightning);

		let entity = app.world_mut().spawn(bundle).id();
		app.update();
		app.world()
			.entity(entity)
			.get::<InnerText>()
			.unwrap()
			.clone()
	}

	#[test]
	fn global_styles() {
		setup((
			InnerText("div { color: red; }".to_string()),
			StyleElement,
			StyleScope::Global,
		))
		.xpect_eq(InnerText("div {\n  color: red;\n}\n".to_string()));
	}

	#[test]
	fn local_styles() {
		let placeholder =
			HtmlConstants::default().style_id_attribute_placeholder();
		setup((InnerText("div { color: red; }".to_string()), StyleElement))
			.xpect_eq(InnerText(format!(
				"div[{placeholder}] {{\n  color: red;\n}}\n"
			)));
	}

	#[test]
	fn local_styles_with_media_query() {
		let placeholder =
			HtmlConstants::default().style_id_attribute_placeholder();
		setup((
			InnerText("@media (width <= 768px) { div { color: #00f; } }".to_string()),
			StyleElement,
		))
		.xpect_eq(InnerText(format!(
			"@media (width <= 768px) {{\n  div[{placeholder}] {{\n    color: #00f;\n  }}\n}}\n"
		)));
	}
}
