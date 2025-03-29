use crate::prelude::*;

/// This module is for handling rsx text blocks in html text node.
///
/// The tricky part of resumability encoding the *minimal* amount of information
/// in html, the first version of quik relied heavily on using `<-- COMMENTS -->` to
/// break up text nodes but this bloats html size very quickly.
/// Instead this encoder uses the bare minimum information more closely resembling
/// the quik 2.0 proposal https://www.builder.io/blog/qwik-2-coming-soon
///
/// An element may have multiple collapsed text blocks,
/// for instance:
/// ```html
/// <div> the quick brown {animal} <b> jumps </b> over the {color} dog </div>
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextBlockEncoder {
	pub parent_id: TreeIdx,
	/// the index of the child text node that collapsed
	/// a vec of 'next index to split at'
	pub split_positions: Vec<Vec<usize>>,
}

impl TextBlockEncoder {
	pub fn new(parent_id: TreeIdx) -> Self {
		Self {
			parent_id,
			split_positions: Vec::new(),
		}
	}


	/// Store the indices
	pub fn encode(idx: TreeIdx, el: &RsxElement) -> Self {
		let mut encoder = Self::new(idx);
		// the index is the child index and the value is a vec of 'next index to split at'
		// let indices: Vec<Vec<usize>> = Vec::new();

		let mut child_index = 0;

		let mut push = |pos: usize, child_index: usize| match encoder
			.split_positions
			.get_mut(child_index)
		{
			Some(vec) => vec.push(pos),
			None => {
				encoder.split_positions.resize(child_index + 1, Vec::new());
				encoder.split_positions.last_mut().unwrap().push(pos);
			}
		};

		for node in CollapsedNode::from_element(el) {
			match node {
				CollapsedNode::StaticText(t) => {
					push(t.len(), child_index);
				}
				CollapsedNode::RustText(t) => {
					push(t.len(), child_index);
				}
				CollapsedNode::Break => {
					child_index += 1;
				}
			}
		}

		// no need to split at the last index
		for pos in encoder.split_positions.iter_mut() {
			pos.pop();
		}
		encoder.split_positions.retain(|pos| !pos.is_empty());

		encoder
	}
}



#[derive(Debug, Clone, PartialEq)]
enum CollapsedNode {
	/// static text, ie `rsx!{"foo"}`
	StaticText(String),
	/// text that can change, ie `rsx!{{val}}`
	RustText(String),
	/// doctype, comment, and element all break text node
	/// ie `rsx!{<div/>}`
	Break,
}
impl CollapsedNode {
	#[allow(unused)]
	pub(crate) fn as_str(&self) -> &str {
		match self {
			CollapsedNode::StaticText(val) => val,
			CollapsedNode::RustText(val) => val,
			CollapsedNode::Break => "|",
		}
	}
}

impl CollapsedNode {
	fn from_element(el: &RsxElement) -> Vec<CollapsedNode> {
		Self::from_node(&el.children)
	}
	fn from_node(node: &RsxNode) -> Vec<CollapsedNode> {
		let mut out = Vec::new();
		match node {
			RsxNode::Fragment { nodes, .. } => {
				out.extend(nodes.into_iter().flat_map(Self::from_node));
			}
			RsxNode::Component(RsxComponent { root, .. }) => {
				out.extend(Self::from_node(root));
			}
			RsxNode::Block(RsxBlock { initial, .. }) => {
				// let initial: &RsxNode = initial.as_ref();
				let html = initial
					.as_ref()
					.pipe(RsxToHtml::default())
					.pipe(RenderHtml::default())
					.unwrap();
				out.push(CollapsedNode::RustText(html));
			}
			RsxNode::Text { value, .. } => {
				out.push(CollapsedNode::StaticText(value.clone()))
			}
			RsxNode::Doctype { .. } => out.push(CollapsedNode::Break),
			RsxNode::Comment { .. } => out.push(CollapsedNode::Break),
			RsxNode::Element(_) => out.push(CollapsedNode::Break),
		}
		return out;
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlockPosition {
	/// the actual node index of the html parent element
	pub child_index: usize,
	/// the starting index of the text block
	pub text_index: usize,
	/// the length of the text block
	pub len: usize,
}

impl TextBlockPosition {
	/// returns a vec where the indices are the child indexes,
	/// and the values are a text index and length of each block
	/// Block positions at 0 are ignored
	pub fn into_split_positions(
		positions: Vec<TextBlockPosition>,
	) -> Vec<Vec<usize>> {
		let mut out = Vec::new();
		for pos in positions {
			let child = {
				if let Some(child) = out.get_mut(pos.child_index) {
					child
				} else {
					out.resize(pos.child_index + 1, Vec::new());
					out.last_mut().unwrap()
				}
			};
			if pos.text_index > 0 {
				child.push(pos.text_index);
			}
			child.push(pos.text_index + pos.len);
		}
		out
	}
}


#[cfg(test)]
mod test {
	use super::CollapsedNode;
	use crate::as_beet::*;
	use sweet::prelude::*;

	#[derive(Node)]
	struct Adjective;

	fn adjective(_: Adjective) -> RsxRoot {
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

		let tree = rsx! {
			<div>
				"The "{desc}" and "{color}<b>fox</b> {action}" the "
				<Adjective>and fat</Adjective>dog
			</div>
		};
		let RsxNode::Element(el) = &tree.node else {
			panic!("expected element");
		};


		let collapsed = CollapsedNode::from_element(&el);

		expect(&collapsed).to_be(&vec![
			CollapsedNode::StaticText("The ".into()),
			CollapsedNode::RustText("quick".into()),
			CollapsedNode::StaticText(" and ".into()),
			CollapsedNode::RustText("brown".into()),
			CollapsedNode::Break,
			CollapsedNode::RustText("jumps over".into()),
			CollapsedNode::StaticText(" the ".into()),
			CollapsedNode::StaticText("lazy".into()),
			CollapsedNode::Break,
			CollapsedNode::StaticText("dog\n\t\t\t".into()),
		]);
	}
}
