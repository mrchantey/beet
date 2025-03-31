use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use proc_macro2::Span;
use quote::ToTokens;
use rstml::node::CustomNode;
use rstml::node::Node;
use syn::spanned::Spanned;

type Result = std::result::Result<(), (Span, String)>;

pub fn validate_style_node<'a, C: CustomNode>(
	children: &'a Vec<Node<C>>,
) -> Result {
	for node in children {
		let result = match node {
			Node::Text(node_text) => {
				let str = node_text.value_string();
				if str.is_empty() {
					panic!("Empty text node");
				}
				validate_css(&str, node_text)
			}
			Node::RawText(raw_text) => {
				let str = raw_text.to_string_best();
				if str.is_empty() {
					panic!("Empty text node");
				}
				validate_css(&str, raw_text)
			}
			_ => Ok(()),
		};
		if result.is_err() {
			return result;
		}
	}
	Ok(())
}


pub fn validate_css<'a>(val: &'a str, source: impl ToTokens) -> Result {
	StyleSheet::parse(val, ParserOptions::default())
		.map(|_| ())
		.map_err(|err| {
			let span = source.to_token_stream().span();

			let mut linecol = source.to_token_stream().span().start();
			// attempt to find error location probs wrong but best we can do i guess
			// remember proc_macro lines are 1 based, lightningcss is 0 based
			if let Some(err_loc) = err.loc {
				linecol.line += err_loc.line as usize;
				if err_loc.line == 0 {
					linecol.column += err_loc.column as usize;
				}
			}
			let msg = format!(
				"CSS Error at approx ({}:{}): {}",
				linecol.line,
				linecol.column,
				err.kind.to_string()
			);
			(span, msg)
		})
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	// use proc_macro2::LineColumn;
	use proc_macro2::TokenStream;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(validate_css("body { color: red; }", TokenStream::default()))
			.to_be_ok();
		// expect(validate_css("//dsds", TokenStream::default()))
		// 	.to_be_ok();
		// expect(
		// 	validate_css("body { :red ", TokenStream::default())
		// 		.unwrap_err()
		// 		.0,
		// )
		// .to_be(LineColumn {
		// 	line: 1,
		// 	column: 13,
		// });
	}
}
