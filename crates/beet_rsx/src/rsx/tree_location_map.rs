use crate::prelude::*;

/// This map is updated every hot reload, the position
/// of a rust block in the tree can change
#[derive(Debug, Default, Clone, PartialEq)]
pub struct TreeLocationMap {
	// we could technically use a vec where the indices are 'block_idx',
	// and track block_idx in the [TreeLocation]
	// but at this stage of the project thats harder to reason about
	// and this provides symmetry with [Self::collapsed_elements]
	pub rusty_locations: HashMap<RsxIdx, TreeLocation>,
	pub collapsed_elements: HashMap<RsxIdx, TextBlockEncoder>,
}

///	Delimiter Reference:
/// - `,` `-` `.` are used by [TreeLocation::to_csv] and [TextBlockEncoder::to_csv]
/// - `*` seperates key value pairs
/// - `;` seperates items in hash maps
/// - `_` seperates [Self::rusty_locations] and [Self::collapsed_elements]
impl TreeLocationMap {
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

				Ok((key, TreeLocation::from_csv(value)?))
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

		TreeLocationVisitor::visit(node, |loc, node| match node {
			RsxNode::Block(_) => {
				map.rusty_locations.insert(loc.rsx_idx, loc);
			}
			RsxNode::Element(el) => {
				if el.children.directly_contains_rust_node() {
					let encoded = TextBlockEncoder::encode(loc.rsx_idx, el);
					map.collapsed_elements.insert(loc.rsx_idx, encoded);
				}
			}
			_ => {}
		});
		map
	}
}


#[cfg(test)]
mod test {
	use crate::as_beet::*;
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

		let map = TreeLocationMap::from_node(&root);

		// test csv
		let csv = map.to_csv();
		let map2 = TreeLocationMap::from_csv(&csv).unwrap();
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
		expect(&map.rusty_locations[&3]).to_be(&TreeLocation {
			rsx_idx: 3,
			parent_idx: 0,
			child_idx: 1,
		});
		// {color}
		expect(&map.rusty_locations[&6]).to_be(&TreeLocation {
			rsx_idx: 6,
			parent_idx: 0,
			child_idx: 3,
		});
		// {action}
		expect(&map.rusty_locations[&10]).to_be(&TreeLocation {
			rsx_idx: 10,
			parent_idx: 0,
			child_idx: 5,
		});
	}
}
