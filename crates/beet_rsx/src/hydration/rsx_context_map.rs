use crate::prelude::*;
use std::collections::HashMap;


/// This map is updated every hot reload, the position
/// of a rust block in the tree can change
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RsxContextMap {
	pub rust_blocks: Vec<RsxContext>,
	pub collapsed_elements: HashMap<usize, TextBlockEncoder>,
}

// const RUST_BLOCK_DELIMITER

impl RsxContextMap {
	pub fn to_csv(&self) -> String {
		let mut csv = String::new();
		csv.push_str(
			&self
				.rust_blocks
				.iter()
				.map(RsxContext::to_csv)
				.collect::<Vec<_>>()
				.join("-"),
		);
		csv.push_str("_");
		csv.push_str(
			&self
				.collapsed_elements
				.iter()
				.map(|(k, v)| format!("{}*{}", k, v.to_csv()))
				.collect::<Vec<_>>()
				.join(";"),
		);
		csv
	}

	pub fn from_csv(csv: &str) -> ParseResult<Self> {
		let mut parts = csv.split('_');
		let rust_blocks = parts
			.next()
			.ok_or_else(|| ParseError::Serde("missing rust blocks".into()))?
			.split("-")
			.map(|s| RsxContext::from_csv(s))
			.collect::<ParseResult<Vec<_>>>()?;

		let collapsed_elements = parts
			.next()
			.ok_or_else(|| ParseError::Serde("missing rust blocks".into()))?
			.split(";")
			.map(|s| {
				let mut parts = s.split('*');
				let key = parts
					.next()
					.ok_or_else(|| ParseError::Serde("missing key".into()))?
					.parse()?;
				let value = parts
					.next()
					.ok_or_else(|| ParseError::Serde("missing value".into()))?;

				Ok((key, TextBlockEncoder::from_csv(value)?))
			})
			.collect::<ParseResult<HashMap<_, _>>>()?;

		Ok(Self {
			rust_blocks,
			collapsed_elements,
		})
	}


	pub fn from_node(node: &RsxNode) -> Self {
		let mut map = Self::default();

		let visitor = RsxContext::visit(node, |cx, node| match node {
			RsxNode::Block { .. } => {
				assert_eq!(cx.block_idx(), map.rust_blocks.len());
				map.rust_blocks.push(cx.clone());
			}
			RsxNode::Element(el) => {
				if el.contains_blocks() {
					let encoded =
						TextBlockEncoder::encode(cx.element_idx(), el);
					map.collapsed_elements.insert(cx.element_idx(), encoded);
				}
			}
			_ => {}
		});
		assert_eq!(visitor.block_idx(), map.rust_blocks.len());

		map
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let desc = "quick";
		let color = "brown";
		let action = "jumps over";

		let tree = rsx! {<div>"The "{desc}" and "{color}<b> fox </b> {action}" the lazy " and fat dog</div>};

		let map = RsxContextMap::from_node(&tree);

		let csv = map.to_csv();
		let map2 = RsxContextMap::from_csv(&csv).unwrap();
		expect(&map2).to_be(&map);


		expect(map.collapsed_elements).to_be(
			vec![(0, TextBlockEncoder {
				parent_id: 0,
				split_positions: vec![vec![4, 5, 5], vec![10, 10]],
			})]
			.into_iter()
			.collect(),
		);

		expect(&map.rust_blocks[0]).to_be(&RsxContext::new(3, 0, 1, 1));
		expect(&map.rust_blocks[1]).to_be(&RsxContext::new(5, 1, 1, 2));
		expect(&map.rust_blocks[2]).to_be(&RsxContext::new(7, 2, 2, 3));
	}
}
