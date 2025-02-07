use crate::prelude::*;
use std::collections::HashMap;


/// This map is updated every hot reload, the position
/// of a rust block in the tree can change
#[derive(Debug, Default, Clone, PartialEq)]
pub struct DomLocationMap {
	// we could technically use a vec where the indices are 'block_idx',
	// and track block_idx in the [DomLocation]
	// but at this stage of the project thats harder to reason about
	// and this provides symmetry with [Self::collapsed_elements]
	pub rusty_locations: HashMap<RsxIdx, DomLocation>,
	pub collapsed_elements: HashMap<DomIdx, TextBlockEncoder>,
}

///	Delimiter Reference:
/// - `,` `-` `.` are used by [DomLocation::to_csv] and [TextBlockEncoder::to_csv]
/// - `*` seperates key value pairs
/// - `;` seperates items in hash maps
/// - `_` seperates [Self::rusty_locations] and [Self::collapsed_elements]
impl DomLocationMap {
	pub fn to_csv(&self) -> String {
		let mut csv = String::new();
		csv.push_str(
			&self
				.rusty_locations
				.iter()
				.map(|(k, v)| format!("{}*{}", k, v.to_csv()))
				.collect::<Vec<_>>()
				.join(";"),
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
			.ok_or_else(|| ParseError::Serde("missing rusty locations".into()))?
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

				Ok((key, DomLocation::from_csv(value)?))
			})
			.collect::<ParseResult<HashMap<_, _>>>()?;
		let collapsed_elements = parts
			.next()
			.ok_or_else(|| ParseError::Serde("missing text encoders".into()))?
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
				map.rusty_locations.insert(loc.rsx_idx, loc);
			}
			RsxNode::Element(el) => {
				if el.contains_blocks() {
					let encoded = TextBlockEncoder::encode(loc.dom_idx, el);
					map.collapsed_elements.insert(loc.dom_idx, encoded);
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

		// test csv
		let csv = map.to_csv();
		let map2 = DomLocationMap::from_csv(&csv).unwrap();
		expect(&map2).to_be(&map);
		// println!("{:#?}", map);

		expect(map.collapsed_elements).to_be(
			vec![(0, TextBlockEncoder {
				parent_id: 0,
				split_positions: vec![vec![4, 5, 5], vec![10]],
			})]
			.into_iter()
			.collect::<HashMap<_, _>>(),
		);
		// {desc}
		expect(&map.rusty_locations[&2]).to_be(&DomLocation {
			parent_idx: 0,
			dom_idx: 2,
			child_idx: 1,
			rsx_idx: 2,
		});
		// {color}
		expect(&map.rusty_locations[&5]).to_be(&DomLocation {
			parent_idx: 0,
			dom_idx: 4,
			child_idx: 3,
			rsx_idx: 5,
		});
		// {action}
		expect(&map.rusty_locations[&9]).to_be(&DomLocation {
			parent_idx: 0,
			dom_idx: 7,
			child_idx: 5,
			rsx_idx: 9,
		});
	}
}
