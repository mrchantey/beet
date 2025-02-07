use crate::prelude::*;
use std::collections::HashMap;


/// This map is updated every hot reload, the position
/// of a rust block in the tree can change
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DomLocationMap {
	pub rusty_locations: Vec<DomLocation>,
	pub collapsed_elements: HashMap<NodeIdx, TextBlockEncoder>,
}

// const RUST_BLOCK_DELIMITER

impl DomLocationMap {
	pub fn to_csv(&self) -> String {
		let mut csv = String::new();
		csv.push_str(
			&self
				.rusty_locations
				.iter()
				.map(DomLocation::to_csv)
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
		let rusty_locations = parts
			.next()
			.ok_or_else(|| ParseError::Serde("missing rust blocks".into()))?
			.split("-")
			.map(|s| DomLocation::from_csv(s))
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
			rusty_locations,
			collapsed_elements,
		})
	}


	pub fn from_node(node: &RsxNode) -> Self {
		let mut map = Self::default();

		DomLocationVisitor::visit(node, |loc, node| match node {
			RsxNode::Block(_) => {
				map.rusty_locations.push(loc);
			}
			RsxNode::Element(el) => {
				if el.contains_blocks() {
					let encoded = TextBlockEncoder::encode(loc.node_idx, el);
					map.collapsed_elements.insert(loc.node_idx, encoded);
				}
			}
			_ => {}
		});
		map
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use std::collections::HashMap;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let desc = "quick";
		let color = "brown";
		let action = "jumps over";

		let root = rsx! {
			<div>
				"The "{desc}" and "{color}<b>fox</b>{action}" the lazy "andfatdog
			</div>
		};

		let map = DomLocationMap::from_node(&root);

		let csv = map.to_csv();
		let map2 = DomLocationMap::from_csv(&csv).unwrap();
		expect(&map2).to_be(&map);


		expect(map.collapsed_elements).to_be(
			vec![(0, TextBlockEncoder {
				parent_id: 0,
				split_positions: vec![vec![4, 5, 5], vec![10]],
			})]
			.into_iter()
			.collect::<HashMap<_, _>>(),
		);
		// {desc}
		expect(&map.rusty_locations[0]).to_be(&DomLocation {
			parent_idx: 0,
			node_idx: 2,
			child_idx: 1,
		});
		// {color}
		expect(&map.rusty_locations[1]).to_be(&DomLocation {
			parent_idx: 0,
			node_idx: 4,
			child_idx: 3,
		});
		// {action}
		expect(&map.rusty_locations[2]).to_be(&DomLocation {
			parent_idx: 0,
			node_idx: 7,
			child_idx: 5,
		});
	}
}
