use std::hash::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// An rsx template is conceptually similar to a html template
/// but instead of {{PLACEHOLDER}} there is a hash for a known
/// location of the associated rust code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateNode {
	Component {
		hash: LineColumn,
		tag: String,
		self_closing: bool,
		children: Vec<RsxTemplateNode>,
		attributes: Vec<RsxTemplateAttribute>,
	},
	RustBlock(LineColumn),
	Element {
		tag: String,
		self_closing: bool,
		attributes: Vec<RsxTemplateAttribute>,
		children: Vec<RsxTemplateNode>,
	},
	Doctype,
	Text(String),
	Comment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RsxTemplateAttribute {
	Key { key: String },
	KeyValue { key: String, value: String },
	Block(LineColumn),
	BlockValue { key: String, value: LineColumn },
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineColumn {
	pub line: u32,
	pub column: u32,
}


impl LineColumn {
	pub fn new(line: u32, column: u32) -> Self { Self { line, column } }
	pub fn to_hash(&self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.hash(&mut hasher);
		hasher.finish()
	}
}



/// TODO this may be used for resumability
#[allow(dead_code)]
struct RsxTemplateNodeToHtml {
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
		let loc = LineColumn::new(line!() + 1, 33);
		let node = rsx_template! {<div>{value}</div>};

		expect(&node[0]).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![],
			children: vec![RsxTemplateNode::RustBlock(loc)],
		});
	}
	#[test]
	fn complex() {
		let ident_linecol = LineColumn::new(line!() + 7, 10);
		let component_linecol = LineColumn::new(line!() + 9, 5);
		let template = rsx_template! {
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

		expect(&template[0]).to_be(&RsxTemplateNode::Element {
			tag: "div".to_string(),
			self_closing: false,
			attributes: vec![
				RsxTemplateAttribute::Key {
					key: "key".to_string(),
				},
				RsxTemplateAttribute::KeyValue {
					key: "str".to_string(),
					value: "value".to_string(),
				},
				RsxTemplateAttribute::KeyValue {
					key: "num".to_string(),
					value: "32".to_string(),
				},
				RsxTemplateAttribute::BlockValue {
					key: "ident".to_string(),
					value: ident_linecol,
				},
			],
			children: vec![RsxTemplateNode::Element {
				tag: "p".to_string(),
				self_closing: false,
				attributes: vec![],
				children: vec![
					RsxTemplateNode::Text("hello\n\t\t\t\t\t".to_string()),
					RsxTemplateNode::Component {
						hash: component_linecol,
						tag: "MyComponent".to_string(),
						self_closing: false,
						children: vec![RsxTemplateNode::Element {
							tag: "div".to_string(),
							self_closing: false,
							attributes: vec![],
							children: vec![RsxTemplateNode::Text(
								"some child".to_string(),
							)],
						}],
						attributes: vec![],
					},
				],
			}],
		});
	}

	#[test]
	fn ron() {
		let mut template = rsx_template! {
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
		let template_ron = rsx_template_ron! {
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

		let mut parsed =
			ron::de::from_str::<Vec<RsxTemplateNode>>(&template_ron).unwrap();
		zero_out_linecol(&mut template);
		zero_out_linecol(&mut parsed);

		expect(template).to_be(parsed);
	}

	fn zero_out_linecol(nodes: &mut Vec<RsxTemplateNode>) {
		for item in nodes {
			match item {
				RsxTemplateNode::Component { hash, .. } => {
					hash.line = 0;
					hash.column = 0;
				}
				RsxTemplateNode::RustBlock(loc) => {
					loc.line = 0;
					loc.column = 0;
				}
				RsxTemplateNode::Element {
					attributes,
					children,
					..
				} => {
					for attr in attributes {
						if let RsxTemplateAttribute::BlockValue {
							value, ..
						} = attr
						{
							value.line = 0;
							value.column = 0;
						}
						if let RsxTemplateAttribute::Block(loc) = attr {
							loc.line = 0;
							loc.column = 0;
						}
					}
					// Recursively process children
					zero_out_linecol(children);
				}
				_ => {}
			}
		}
	}
}
