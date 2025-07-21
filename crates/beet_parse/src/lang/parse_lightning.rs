use beet_core::prelude::*;
use bevy::prelude::*;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

/// Parse css using lightningcss, applying styleid to selectors as required.
pub fn parse_lightning(
	constants: Res<HtmlConstants>,
	mut query: Populated<
		(
			&mut InnerText,
			Option<&LangSnippetHash>,
			Option<&StyleScope>,
			Option<&FileSpanOf<ElementNode>>,
		),
		Added<StyleElement>,
	>,
) -> Result {
	query
		.iter_mut()
		.collect::<Vec<_>>()
		.into_par_iter()
		.map(|(mut text, hash, scope, span)| {
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
			if scope == StyleScope::Local
				&& let Some(hash) = hash
			{
				let class_name = constants.style_id_attribute(**hash);
				stylesheet.rules.0.iter_mut().for_each(|rule| {
					match rule {
						// currently only style rules are supported
						lightningcss::rules::CssRule::Style(style_rule) => {
							style_rule.selectors.0.iter_mut().for_each(
								|selector| {
									selector.append(
										lightningcss::selector::Component::AttributeInNoNamespaceExists {
											local_name: class_name.clone().into(),
											local_name_lower: class_name.clone().into(),
										}
									);
								},
							);
						}
						_ => {}
					}
				});
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




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.init_resource::<HtmlConstants>()
			.add_systems(Update, parse_lightning);

		let global = app
			.world_mut()
			.spawn((
				InnerText("div { color: red; }".to_string()),
				StyleElement,
				StyleScope::Global,
			))
			.id();
		let local = app
			.world_mut()
			.spawn((
				InnerText("div { color: red; }".to_string()),
				StyleElement,
				LangSnippetHash::new(7),
			))
			.id();

		app.update();

		app.world()
			.entity(global)
			.get::<InnerText>()
			.unwrap()
			.xpect()
			.to_be(&InnerText("div {\n  color: red;\n}\n".to_string()));
		app.world()
			.entity(local)
			.get::<InnerText>()
			.unwrap()
			.xpect()
			.to_be(&InnerText(
				"div[data-beet-style-id-7] {\n  color: red;\n}\n".to_string(),
			));
	}
}
