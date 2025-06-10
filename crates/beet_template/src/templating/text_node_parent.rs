use crate::prelude::*;
use beet_common::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use sweet::prelude::HierarchyQueryExtExt;


pub fn apply_text_node_parents_plugin(app: &mut App) {
	app.add_systems(
		Update,
		apply_text_node_parents
			.after(super::apply_slots)
			.in_set(ApplyTransformsStep),
	);
}


fn apply_text_node_parents(
	mut commands: Commands,
	query: Populated<(Entity, &Children), With<ElementNode>>,
	parser: Parser,
) {
	for (entity, children) in query.iter() {
		let nodes = parser.parse(children);
		if !nodes.is_empty() {
			commands.entity(entity).insert(TextNodeParent::new(nodes));
		}
	}
}



#[rustfmt::skip]
#[derive(SystemParam)]
struct Parser<'w, 's> {
	fragment_children: Query<'w, 's,&'static Children,Without<ElementNode>>,
	texts: Query<'w, 's, &'static TextNode>,
	breaks: Query<'w, 's, (),Or<(With<DoctypeNode>, With<CommentNode>, With<ElementNode>)>>,
}


impl Parser<'_, '_> {
	/// Parse the children of this element into a vector of [`CollapsedNode`],
	/// which can be used to create a [`TextNodeParent`].
	pub fn parse(&self, children: &Children) -> Vec<CollapsedNode> {
		let mut out = Vec::new();
		for child in children.iter() {
			self.append(&mut out, child);
		}
		out
	}

	// we must dfs because thats the order in which a collapse occurs
	fn append(&self, out: &mut Vec<CollapsedNode>, entity: Entity) {
		if let Ok(text) = self.texts.get(entity) {
			// if we have a text node, we push it
			out.push(CollapsedNode::Text(text.0.clone()));
		} else if self.breaks.contains(entity) {
			// if we have a break, we push it
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


/// An [`ElementNode`] with one or more [`TextNode`] children. This component
/// contains the split positions of the text nodes, which can be used to
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
				CollapsedNode::Text(t) => {
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
	/// A [`TextNode`]
	Text(String),
	/// A [`DoctypeNode`], [`CommentNode`], or [`ElementNode`] which would
	/// break an adjacent [`TextNode`] collapse
	Break,
}
impl CollapsedNode {
	#[allow(unused)]
	pub(crate) fn as_str(&self) -> &str {
		match self {
			CollapsedNode::Text(val) => val,
			CollapsedNode::Break => "|",
		}
	}
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
	fn roundtrip() {
		let desc = "quick";
		let color = "brown";
		let action = "jumps over";

		let mut world = World::new();
		let entity = world
			.spawn(rsx! {
				<div>
					"The "{desc}" and "{color}<b>fox</b> {action}" the "
					<Adjective>and fat</Adjective>dog
				</div>
			})
			.id();
		world.run_system_once(apply_slots).unwrap().unwrap();

		let collapsed = world
			.run_system_once(
				move |query: Populated<&Children>, parser: Parser| {
					parser.parse(query.get(entity).unwrap())
				},
			)
			.unwrap();


		expect(&collapsed).to_be(&vec![
			CollapsedNode::Text("The ".into()),
			CollapsedNode::Text("quick".into()),
			CollapsedNode::Text(" and ".into()),
			CollapsedNode::Text("brown".into()),
			CollapsedNode::Break,
			CollapsedNode::Text("jumps over".into()),
			CollapsedNode::Text(" the ".into()),
			CollapsedNode::Text("lazy".into()),
			CollapsedNode::Break,
			CollapsedNode::Text("dog\n\t\t\t\t".into()),
		]);
	}
}
