use super::error::Error;
use beet_common::prelude::*;
use bevy::prelude::*;
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

/// Parse css using lightningcss, applying styleid to selectors as required.
pub fn parse_lightning(
	constants: Res<HtmlConstants>,
	mut commands: Commands,
	query: Populated<
		(
			Entity,
			&LangSnippet,
			&NodeTag,
			Option<&StyleId>,
			Option<&FileSpanOf<ElementNode>>,
		),
		Added<LangSnippet>,
	>,
) -> Result {
	let output = query
		.iter()
		.collect::<Vec<_>>()
		.into_par_iter()
		.filter(|(_, _, tag, _, _)| tag.as_str() == "style")
		.map(|(entity, partial, _tag, styleid, span)| {
			// Parse the stylesheet
			let mut stylesheet =
				StyleSheet::parse(&partial, ParserOptions::default()).map_err(
					|e| Error::LightningCss {
						span: span.map(|s| s.value.clone()),
						err: e.to_string(),
					},
				)?;
			if let Some(styleid) = styleid {
				let class_name = constants.style_id_attribute(*styleid);
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
			#[cfg(not(debug_assertions))]
			let options = PrinterOptions {
				minify: true,
				..Default::default()
			};

			let new_css = stylesheet
				.to_css(options)
				.map_err(|e| Error::LightningCss {
					span: span.map(|s| s.value.clone()),
					err: e.to_string(),
				})?
				.code;
			Ok((entity, new_css))
		})
		.collect::<Result<Vec<_>>>()?;
	// only local style tags

	for (entity, css) in output {
		commands.entity(entity).insert(LangSnippet::new(css));
	}
	Ok(())
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::prelude::*;
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
				LangSnippet("div { color: red; }".to_string()),
				NodeTag("style".into()),
				// no styleid indicates global
			))
			.id();
		let local = app
			.world_mut()
			.spawn((
				LangSnippet("div { color: red; }".to_string()),
				NodeTag("style".into()),
				StyleId::new(7),
			))
			.id();

		app.update();

		app.world()
			.entity(global)
			.get::<LangSnippet>()
			.unwrap()
			.xpect()
			.to_be(&LangSnippet("div {\n  color: red;\n}\n".to_string()));
		app.world()
			.entity(local)
			.get::<LangSnippet>()
			.unwrap()
			.xpect()
			.to_be(&LangSnippet(
				"div[data-beet-style-id-7] {\n  color: red;\n}\n".to_string(),
			));
	}
}
