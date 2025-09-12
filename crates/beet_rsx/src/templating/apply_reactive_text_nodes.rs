use beet_dom::prelude::*;
use beet_utils::prelude::*;
use bevy::prelude::*;


pub fn apply_reactive_text_nodes(
	mut commands: Commands,
	constants: Res<HtmlConstants>,
	query: Populated<
		(Entity, &ChildOf, &DomIdx),
		(With<TextNode>, With<SignalEffect>, Without<AttributeOf>),
	>,
	children: Query<&Children>,
) -> Result {
	for (entity, childof, dom_idx) in query.iter() {
		let parent = childof.parent();
		let children = children.get(parent)?;


		let before = commands
			.spawn(CommentNode::new(format!(
				"{}|{}",
				constants.text_node_marker, dom_idx.0
			)))
			.id();

		let after = commands
			.spawn(CommentNode::new(format!("/{}", constants.text_node_marker)))
			.id();

		let mut children = children.iter().collect::<Vec<_>>();

		let child_index = children
			.iter()
			.position(|child| *child == entity)
			.ok_or_else(|| bevyhow!("TextNode not found in Children"))?;

		children.insert(child_index, before);
		children.insert(child_index + 2, after);

		commands.entity(parent).replace_children(&children);
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	#[test]
	fn ignores_static() {
		HtmlDocument::parse_bundle(rsx! { <div>hello {"world"}</div> })
		.xpect_str("<!DOCTYPE html><html><head></head><body><div>hello world</div></body></html>");
	}
	#[test]
	fn simple() {
		let (get, _set) = signal("world".to_string());
		HtmlDocument::parse_bundle(rsx! { <div>hello {get}</div> })
	.xpect_str("<!DOCTYPE html><html><head></head><body><div data-beet-dom-idx=\"0\">hello <!--bt|1-->world<!--/bt--></div></body></html>");
	}
	#[template]
	fn Adjective() -> impl Bundle {
		let (get, _set) = signal("and".to_string());
		rsx! {
			"lazy "
			{get}
			<slot />
		}
	}
	#[test]
	fn complex() {
		let desc = "quick";
		let color = "brown";
		let (get, _set) = signal("jumps over".to_string());
		HtmlDocument::parse_bundle(rsx! {
			<div>
				"The "{desc}" and "{color}<b>fox</b> {get}" the "
				<Adjective>" fat "</Adjective>"dog"
			</div>
		})
		.xpect_str("<!DOCTYPE html><html><head></head><body><div data-beet-dom-idx=\"0\">The quick and brown<b>fox</b><!--bt|1-->jumps over<!--/bt--> the lazy <!--bt|2-->and<!--/bt--> fat dog</div></body></html>");
	}
}
