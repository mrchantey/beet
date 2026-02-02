//! Extracts and collects language-specific nodes like scripts, styles, and code blocks.

use beet_core::prelude::*;
use beet_dom::prelude::*;


/// For elements with a `script`, `style` or `code` tag, and without an
/// `node:inline` attribute, parse as a lang node:
/// - insert a [`ScriptElement`]
/// - insert a [`StyleElement`]
pub fn extract_lang_nodes(
	mut commands: Commands,
	query: Populated<(Entity, &NodeTag), Added<NodeTag>>,
	attributes: FindAttribute,
) {
	for (entity, tag) in query.iter() {
		// entirely skip node:inline
		if let Some((attr_ent, _)) = attributes.find(entity, "node:inline") {
			// its done its job, remove it
			commands.entity(attr_ent).despawn();
			continue;
		}
		// Insert the element type
		match tag.as_str() {
			"script" => {
				commands.entity(entity).insert(ScriptElement);
			}
			"style" => {
				commands.entity(entity).insert(StyleElement);
			}
			_ => {
				continue;
			}
		}
	}
}


/// Collects markdown code blocks from pulldown-cmark output into [`CodeNode`] components.
///
/// The following markdown:
/// ```markdown
/// 	```rust
/// 	let foo = bar;
/// 	```
/// ```
/// Will produce the following html:
/// ```html
/// <pre><code class="language-rust">let foo = bar;</code></pre>
/// ```
///
/// This system processes that HTML structure into a [`CodeNode`] with appropriate
/// language and theme metadata.
pub fn collect_md_code_nodes(
	mut commands: Commands,
	query: Query<(Entity, &NodeTag, &Children), Added<ElementNode>>,
	mut inner_text: Query<&mut InnerText>,
	find_attr: FindAttribute,
) {
	for (entity, tag, children) in query.iter() {
		if **tag != "pre" {
			continue;
		}

		if children.len() != 1 {
			continue;
		}
		let Some(&child) = children.first() else {
			continue;
		};

		let lang = find_attr
			.classes(child)
			.into_iter()
			.find(|c| c.starts_with("language-"))
			.map(|c| c.trim_start_matches("language-").to_string());

		let theme = find_attr
			.find(entity, "theme")
			.and_then(|(_, value)| value.map(|v| v.as_str().to_string()));

		commands.entity(entity).insert(CodeNode { lang, theme });

		if let Ok(mut text) = inner_text.get_mut(child) {
			let text = std::mem::take(&mut text.0);
			// pulldown-cmark escapes code html, we undo this to avoid double escaping
			// by syntect
			let unescaped = EscapeHtml::unescape(&text);
			commands.entity(entity).insert(InnerText(unescaped));
			commands.entity(child).despawn();
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_dom::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();
		let is_script = world.spawn(NodeTag::new("script")).id();
		let is_style = world.spawn(NodeTag::new("style")).id();
		let is_inline = world.spawn(NodeTag::new("style")).id();
		world.spawn((AttributeOf(is_inline), AttributeKey::new("node:inline")));

		world.run_system_cached(extract_lang_nodes).unwrap();

		world
			.entity(is_script)
			.contains::<ScriptElement>()
			.xpect_true();
		world
			.entity(is_style)
			.contains::<StyleElement>()
			.xpect_true();
		world
			.entity(is_inline)
			.contains::<StyleElement>()
			.xpect_false();
	}
}
