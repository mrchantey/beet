use beet_core::prelude::*;
use bevy::prelude::*;


/// Handle any LangNodes that were not overwritten by the
/// static lang nodes in apply_lang_snippets.
/// This is essentially the inverse of extract_lang_nodes
pub fn apply_unparsed_lang_nodes(
	mut commands: Commands,
	with_node: Query<
		Entity,
		Or<(With<ScriptElement>, With<StyleElement>, With<CodeElement>)>,
	>,
	unparsed_text: Query<(Entity, &InnerText)>,
) {
	for entity in with_node.iter() {
		commands.entity(entity).insert(ElementNode::open());
	}
	for (entity, inner_text) in unparsed_text.iter() {
		commands
			.entity(entity)
			.with_child(TextNode::new(inner_text.0.clone()));
	}
}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use sweet::prelude::*;


	#[test]
	fn style_inline() {
		HtmlFragment::parse_bundle(rsx! {<style>body { color: red; }</style>})
			.xpect()
			.to_be_str("<style>body { color: red; }</style>");
	}

	#[test]
	#[cfg(not(feature = "client"))]
	fn style_src() {
		HtmlFragment::parse_bundle(
			rsx! {<style src="../../tests/test_file.css"/>},
		)
		.xpect()
		.to_be_snapshot();
	}

	#[test]
	fn script() {
		HtmlFragment::parse_bundle(
			rsx! {<script type="pizza">let foo = "bar"</script>},
		)
		.xpect()
		.to_be_str("<script type=\"pizza\">let foo = \"bar\"</script>");
	}
	#[test]
	fn code() {
		HtmlFragment::parse_bundle(
			rsx! {<code lang="pizza">let foo = "bar"</code>},
		)
		.xpect()
		.to_be_str("<code lang=\"pizza\">let foo = \"bar\"</code>");
	}
}
