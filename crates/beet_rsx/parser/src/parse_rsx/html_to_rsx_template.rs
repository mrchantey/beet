use anyhow::Result;
use ego_tree::NodeRef;
use scraper::Html;
use scraper::Node;
use std::io;
use std::io::Write;


pub struct HtmlToRsxTemplate;

impl HtmlToRsxTemplate {
	pub fn parse_document(document: &str) -> Result<String> {
		let mut out = String::with_capacity(document.len());
		let html = Html::parse_document(document);
		let root = html.tree.root();
		Self::parse_node(root, &mut out)?;

		Ok(out)
	}


	fn parse_node(node: NodeRef<'_, Node>, buffer: &mut str) -> Result<()> {
		match node.value() {
			// the root node of Html::parse_document
			Node::Document => {
				for child in node.children() {
					Self::parse_node(child, buffer)?;
				}
			}
			// the root of Html::parse_fragment, not an rstml fragment
			Node::Fragment => {
				for child in node.children() {
					Self::parse_node(child, buffer)?;
				}
			}
			Node::Doctype(_) => {
				
				
			},
			Node::Comment(comment) => todo!(),
			Node::Text(text) => todo!(),
			Node::Element(element) => todo!(),
			Node::ProcessingInstruction(processing_instruction) => {
				todo!()
			}
		}


		Ok(())
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() { expect(true).to_be_false(); }
}
