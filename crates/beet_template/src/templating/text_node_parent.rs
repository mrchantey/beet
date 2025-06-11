use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use sweet::prelude::HierarchyQueryExtExt;

pub(super) fn apply_text_node_parents(
	mut commands: Commands,
	query: Populated<(Entity, &Children), With<ElementNode>>,
	parser: Parser,
) {
	for (entity, children) in query.iter() {
		if let Some(nodes) = parser.parse(children) {
			commands.entity(entity).insert(TextNodeParent::new(nodes));
		}
	}
}



#[rustfmt::skip]
#[derive(SystemParam)]
pub(super) struct Parser<'w, 's> {
	fragment_children: Query<'w, 's,&'static Children,Without<ElementNode>>,
	signal_texts: Query<'w, 's, &'static TextNode,With<SignalReceiver<String>>>,
	static_texts: Query<'w, 's, &'static TextNode,Without<SignalReceiver<String>>>,
	breaks: Query<'w, 's, (),Or<(With<DoctypeNode>, With<CommentNode>, With<ElementNode>)>>,
}


impl Parser<'_, '_> {
	/// Parse the children of this element into a vector of [`CollapsedNode`],
	/// which can be used to create a [`TextNodeParent`].
	fn parse(&self, children: &Children) -> Option<Vec<CollapsedNode>> {
		let mut out = Vec::new();
		for child in children.iter() {
			self.append(&mut out, child);
		}
		if out
			.iter()
			.any(|node| matches!(node, CollapsedNode::SignalText(_)))
		{
			Some(out)
		} else {
			None
		}
	}

	// we must dfs because thats the order in which a collapse occurs
	fn append(&self, out: &mut Vec<CollapsedNode>, entity: Entity) {
		if let Ok(text) = self.signal_texts.get(entity) {
			out.push(CollapsedNode::SignalText(text.0.clone()));
		} else if let Ok(text) = self.static_texts.get(entity) {
			out.push(CollapsedNode::StaticText(text.0.clone()));
		} else if self.breaks.contains(entity) {
			out.push(CollapsedNode::Break);
		} else {
			// do nothing, its a fragment
			// TODO do we still need to render expr initial to html like this?
			// TextNodes should already be created from expressions
			// WebNode::Block(RsxBlock { initial, .. }) => {
			// let html = initial
			// 	.as_ref()
			// 	.xpipe(RsxToHtml::default())
			// 	.xpipe(RenderHtmlEscaped::default());
			// out.push(CollapsedNode::Expr(html));
		}
		for child in self.fragment_children.iter_direct_descendants(entity) {
			self.append(out, child);
		}
	}
}


/// An [`ElementNode`] with one or more [`TextNode`] children with a [`SignalReceiver<String>`],
/// meaning it can be used for reactive text updates.
/// This component contains the split positions of the text nodes, which can be used to
/// 'uncollapse' adjacent html text nodes.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct TextNodeParent {
	pub split_positions: Vec<Vec<usize>>,
}
impl TextNodeParent {
	fn new(collapsed_nodes: Vec<CollapsedNode>) -> Self {
		let mut child_index = 0;
		let mut split_positions: Vec<Vec<usize>> = Vec::new();

		let mut push = |pos: usize, child_index: usize| match split_positions
			.get_mut(child_index)
		{
			Some(vec) => vec.push(pos),
			None => {
				split_positions.resize(child_index + 1, Vec::new());
				split_positions.last_mut().unwrap().push(pos);
			}
		};

		for node in collapsed_nodes.into_iter() {
			match node {
				CollapsedNode::SignalText(t) => {
					push(t.len(), child_index);
				}
				CollapsedNode::StaticText(t) => {
					push(t.len(), child_index);
				}
				CollapsedNode::Break => {
					child_index += 1;
				}
			}
		}

		// no need to split at the last index
		for pos in split_positions.iter_mut() {
			pos.pop();
		}
		split_positions.retain(|pos| !pos.is_empty());
		Self { split_positions }
	}
}


#[derive(Debug, Clone, PartialEq)]
enum CollapsedNode {
	/// A [`TextNode`] with a [`SignalReceiver<String>`]
	SignalText(String),
	/// A [`TextNode`] without a [`SignalReceiver<String>`]
	StaticText(String),
	/// A [`DoctypeNode`], [`CommentNode`], or [`ElementNode`] which would
	/// break an adjacent [`TextNode`] collapse
	Break,
}

#[cfg(test)]
mod test {
	use super::CollapsedNode;
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use crate::templating::text_node_parent::Parser;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[template]
	fn Adjective() -> impl Bundle {
		rsx! {
			"lazy"
			<slot />
		}
	}

	#[test]
	fn ignores_empty() {
		let mut world = World::new();
		let entity = world
			.spawn(rsx! {
				<div><span><br/></span></div>
			})
			.id();

		world
			.run_system_once(
				move |query: Populated<&Children>, parser: Parser| {
					parser.parse(query.get(entity).unwrap())
				},
			)
			.unwrap()
			.xpect()
			.to_be_none();
	}
	#[test]
	fn roundtrip() {
		let desc = "quick";
		let color = "brown";
		let (get, _set) = signal("jumps over".to_string());

		let mut world = World::new();
		let entity = world
			.spawn(rsx! {
				<div>
					"The "{desc}" and "{color}<b>fox</b> {get}" the "
					<Adjective>and fat</Adjective>dog
				</div>
			})
			.id();
		world.run_system_once(apply_slots).unwrap().unwrap();

		world
			.run_system_once(
				move |query: Populated<&Children>, parser: Parser| {
					parser.parse(query.get(entity).unwrap())
				},
			)
			.unwrap()
			.unwrap()
			.xpect()
			.to_be(vec![
				CollapsedNode::StaticText("The ".into()),
				CollapsedNode::StaticText("quick".into()),
				CollapsedNode::StaticText(" and ".into()),
				CollapsedNode::StaticText("brown".into()),
				CollapsedNode::Break,
				CollapsedNode::SignalText("jumps over".into()),
				CollapsedNode::StaticText(" the ".into()),
				CollapsedNode::StaticText("lazy".into()),
				CollapsedNode::Break,
				CollapsedNode::StaticText("dog\n\t\t\t\t".into()),
			]);
	}
}
