/// Serializable version of a html node where rust blocks are
/// converted to hashes based on their location
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseRsxNode {
	Component {
		hash: RustLocationHash,
		tag: String,
		self_closing: bool,
		children: Vec<ReverseRsxNode>,
		attributes: Vec<ReverseRsxAttribute>,
	},
	RustBlock(RustLocationHash),
	Element {
		tag: String,
		self_closing: bool,
		attributes: Vec<ReverseRsxAttribute>,
		children: Vec<ReverseRsxNode>,
	},
	Doctype,
	Text(String),
	Comment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseRsxAttribute {
	Key {
		key: String,
	},
	KeyValue {
		key: String,
		value: String,
	},
	Block(RustLocationHash),
	BlockValue {
		key: String,
		value: RustLocationHash,
	},
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RustLocationHash(u64);


impl RustLocationHash {
	pub fn new(hash: u64) -> Self { Self(hash) }
	pub fn inner(&self) -> u64 { self.0 }
}



/// TODO this may be used for resumability
#[allow(dead_code)]
struct ReverseRsxNodeToHtml {
	/// The attribute to identify the block,
	/// ie `<div>{rust_code}</div>`
	/// will become `<div><rsx-block hash="1234"/></div>`
	rust_block_tag: String,
	/// An attribute to identify a rust block attribute,
	/// ie `<div {rust_code}/>`
	/// will become `<div rsx-attr-block="1234"/>`
	attribute_block_key: String,
	/// An attribute to identify a rust block attribute value,
	/// ie `<div key={rust_code}/>`
	/// will become `<div key="rsx-attr-value-1234"/>`
	attribute_value_prefix: String,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn simple() {
		let node = reverse_rsx! {<div>{value}</div>};

		expect(&node[0]).to_be(&ReverseRsxNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			children: vec![ReverseRsxNode::RustBlock(RustLocationHash::new(
				5702809214192366611,
			))],
		});
	}
	#[test]
	fn complex() {
		let reverse_node = reverse_rsx! {
			<div
				key
				str="value"
				num=32
				ident=some_val
				>
				<p>hello
					<MyComponent>
						<div>some child</div>
					</MyComponent>
				</p>
			</div>
		};

		expect(&reverse_node[0]).to_be(&ReverseRsxNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![
				ReverseRsxAttribute::Key {
					key: "key".to_string(),
				},
				ReverseRsxAttribute::KeyValue {
					key: "str".to_string(),
					value: "value".to_string(),
				},
				ReverseRsxAttribute::KeyValue {
					key: "num".to_string(),
					value: "32".to_string(),
				},
				ReverseRsxAttribute::BlockValue {
					key: "ident".to_string(),
					value: RustLocationHash::new(9485767732281972896),
				},
			],
			children: vec![ReverseRsxNode::Element {
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				children: vec![
					ReverseRsxNode::Text("hello\n\t\t\t\t\t".to_string()),
					ReverseRsxNode::Component {
						hash: RustLocationHash::new(1967000568540887016),
						tag: "MyComponent".to_string(),
						self_closing: false,
						children: vec![ReverseRsxNode::Element {
							tag: "div".to_string(),
							self_closing: false,
							attributes: vec![],
							children: vec![ReverseRsxNode::Text(
								"some child".to_string(),
							)],
						}],
						attributes: vec![],
					},
				],
			}],
		});
	}
}
