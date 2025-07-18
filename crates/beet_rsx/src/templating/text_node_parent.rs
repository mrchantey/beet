use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

pub(super) fn apply_text_node_parents(
	mut commands: Commands,
	query: Populated<(Entity, &Children), With<ElementNode>>,
	parser: Parser,
) {
	for (entity, children) in query.iter() {
		let nodes = parser.parse(children);
		if let Some(parent) = TextNodeParent::try_from_collapsed(nodes) {
			commands.entity(entity).insert(parent);
		}
	}
}



#[rustfmt::skip]
#[derive(SystemParam)]
pub(super) struct Parser<'w, 's> {
	// walk fragments and templates
	non_element_children: Query<'w, 's,&'static Children,Without<ElementNode>>,
	signal_texts: Query<'w, 's, &'static TextNode,With<SignalReceiver<String>>>,
	static_texts: Query<'w, 's, &'static TextNode,Without<SignalReceiver<String>>>,
	breaks: Query<'w, 's, (),Or<(With<DoctypeNode>, With<CommentNode>, With<ElementNode>)>>,
}


impl Parser<'_, '_> {
	/// Parse the children of this element into a vector of [`CollapsedNode`],
	/// which can be used to create a [`TextNodeParent`].
	fn parse(&self, children: &Children) -> Vec<CollapsedNode> {
		let mut out = Vec::new();
		for child in children.iter() {
			self.append(&mut out, child);
		}
		out
	}

	// we must dfs because thats the order in which a collapse occurs
	fn append(&self, out: &mut Vec<CollapsedNode>, entity: Entity) {
		if let Ok(text) = self.signal_texts.get(entity) {
			out.push(CollapsedNode::SignalText {
				entity,
				value: text.0.clone(),
			});
		} else if let Ok(text) = self.static_texts.get(entity) {
			out.push(CollapsedNode::StaticText {
				value: text.0.clone(),
			});
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
		for child in self.non_element_children.iter_direct_descendants(entity) {
			self.append(out, child);
		}
	}
}


/// An [`ElementNode`] with one or more [`TextNode`] children with a [`SignalReceiver<String>`],
/// meaning it can be used for reactive text updates.
/// This component contains the split positions of the text nodes, which can be used to
/// 'uncollapse' adjacent html text nodes.
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TextNodeParent {
	pub text_nodes: Vec<TextNodeChild>,
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
// un
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TextNodeChild {
	/// The *post-collapse* index of this html TextNode.
	pub child_index: usize,
	/// A vec of next index to split text at,
	/// and optionally the signal entity
	/// that should be assigned a [`DomTextBinding`] of the **existing**
	/// text node on [`web_sys::Text::split_text`].
	pub split_positions: Vec<(Option<Entity>, usize)>,
}
impl TextNodeChild {
	pub fn new(child_index: usize) -> Self {
		Self {
			child_index,
			split_positions: Vec::new(),
		}
	}
}

impl TextNodeParent {
	fn try_from_collapsed(collapsed_nodes: Vec<CollapsedNode>) -> Option<Self> {
		// track which *post-collapse* index we are up to
		let mut child_index = 0;
		let mut text_nodes = Vec::new();
		let mut current_child = TextNodeChild::new(child_index);

		for node in collapsed_nodes.into_iter() {
			match node {
				CollapsedNode::SignalText { entity, value } => {
					current_child
						.split_positions
						.push((Some(entity), value.len()));
				}
				CollapsedNode::StaticText { value } => {
					current_child.split_positions.push((None, value.len()));
				}
				CollapsedNode::Break => {
					child_index += 1;
					text_nodes.push(std::mem::replace(
						&mut current_child,
						TextNodeChild::new(child_index),
					));
				}
			}
		}
		text_nodes.push(current_child);
		text_nodes.retain(|child| {
			child.split_positions.iter().any(|node| node.0.is_some())
		});
		if text_nodes.is_empty() {
			None
		} else {
			Some(Self { text_nodes })
		}
	}
}


#[derive(Debug, Clone, PartialEq)]
enum CollapsedNode {
	/// A [`TextNode`] with a [`SignalReceiver<String>`]
	SignalText { entity: Entity, value: String },
	/// A [`TextNode`] without a [`SignalReceiver<String>`]
	StaticText { value: String },
	/// A [`DoctypeNode`], [`CommentNode`], or [`ElementNode`] which would
	/// break an adjacent [`TextNode`] collapse
	Break,
}

#[cfg(test)]
mod test {
	use super::CollapsedNode;
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use crate::templating::apply_text_node_parents;
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
			.to_be(vec![CollapsedNode::Break]);
	}
	#[test]
	fn simple() {
		let mut world = World::new();
		let entity = world
			.spawn(rsx! {
				<div>foobar</div>
			})
			.get::<Children>()
			.unwrap()[0];

		world
			.run_system_once(
				move |query: Populated<&Children>, parser: Parser| {
					parser.parse(query.get(entity).unwrap())
				},
			)
			.unwrap()
			.xpect()
			.to_be(vec![CollapsedNode::StaticText {
				value: "foobar".to_string(),
			}]);
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
			.get::<Children>()
			.unwrap()[0];
		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();
		world.run_system_once(apply_slots).unwrap().unwrap();

		world
			.run_system_once(
				move |query: Populated<&Children>, parser: Parser| {
					parser.parse(query.get(entity).unwrap())
				},
			)
			.unwrap()
			.xpect()
			.to_be(vec![
				CollapsedNode::StaticText {
					value: "The ".into(),
				},
				CollapsedNode::StaticText {
					value: "quick".into(),
				},
				CollapsedNode::StaticText {
					value: " and ".into(),
				},
				CollapsedNode::StaticText {
					value: "brown".into(),
				},
				CollapsedNode::Break,
				CollapsedNode::SignalText {
					entity: Entity::from_raw(8),
					value: "jumps over".into(),
				},
				CollapsedNode::StaticText {
					value: " the ".into(),
				},
				CollapsedNode::StaticText {
					value: "lazy".into(),
				},
				CollapsedNode::StaticText {
					value: "and fat".into(),
				},
				CollapsedNode::StaticText {
					value: "dog\n\t\t\t\t".into(),
				},
			]);

		world.run_system_once(apply_text_node_parents).unwrap();

		world
			.entity(entity)
			.get::<TextNodeParent>()
			.unwrap()
			.xpect()
			.to_be(&TextNodeParent {
				text_nodes: vec![TextNodeChild {
					child_index: 1,
					split_positions: vec![
						(Some(Entity::from_raw(8)), 10),
						(None, 5),
						(None, 4),
						(None, 7),
						(None, 8),
					],
				}],
			});
	}
}
