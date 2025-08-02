use beet_core::prelude::*;
use bevy::prelude::*;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;



pub fn parse_syntect(
	mut commands: Commands,
	mut query: Populated<(Entity, &CodeNode, &mut InnerText), Added<CodeNode>>,
) -> Result {
	let syntax_set = SyntaxSet::load_defaults_newlines();
	let theme_set = ThemeSet::load_defaults();
	let theme = &theme_set.themes["base16-ocean.dark"];
	for (entity, code, mut text) in query.iter_mut() {
		let str = &text.0;
		let syntax =
			syntax_set.find_syntax_by_token(&code.lang).ok_or_else(|| {
				let available_sets = syntax_set
					.syntaxes()
					.iter()
					.map(|s| &s.name)
					.collect::<Vec<_>>();
				bevyhow!(
					"Failed to find syntax for language: {}\nAvailable syntaxes: {:#?}",
					code.lang,
					available_sets
				)
			})?;
		let html =
			highlighted_html_for_string(str, &syntax_set, &syntax, theme)?;
		// text.0.0 = html_escape::encode_text(&text.0.0).to_string();
		text.0 = html;

		// hack until template parsing, entity is now a div
		// before: <code lang="rust">RAW CODE</code>
		// after: <div><pre><code>SYNTECT CODE</code></pre></div>
		// commands.entity(entity).insert(NodeTag::new("div"));
		commands.entity(entity).with_related::<AttributeOf>((
			AttributeKey::new("class"),
			TextNode::new("syntect-code"),
		));
		// .remove::<NodeTag>()
		// .remove::<ElementNode>()
		// .insert(FragmentNode);
	}
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
		let mut world = World::new();
		let entity = world
			.spawn((
				CodeNode::new("rust"),
				InnerText("let foo = rsx!{<div>{\"bar\"}</div>}".to_string()),
			))
			.id();
		world.run_system_cached(parse_syntect).unwrap().unwrap();

		let text = world.entity(entity).get::<InnerText>().unwrap();
		expect(&text.0).to_be_snapshot();
	}
}
