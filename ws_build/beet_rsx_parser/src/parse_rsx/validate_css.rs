use super::WebTokens;
use anyhow::Result;
use beet_common::prelude::*;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use sweet::prelude::Pipeline;
pub struct ValidateStyleNode;

impl Pipeline<WebTokens, Result<WebTokens>> for ValidateStyleNode {
	fn apply(self, node: WebTokens) -> Result<WebTokens> {
		// doesnt need to be mut but no ref visitor
		node.walk_web_tokens(|html| match html {
			WebTokens::Element {
				component,
				children,
				..
			} if component.tag.value() == "style"
				&& let WebTokens::Text { value, .. } = &**children =>
			{
				let span = component.span();
				validate_css(&value.value(), span)
			}
			_ => Ok(()),
		})?;
		Ok(node)
	}
}

pub fn validate_css(val: &str, span: &FileSpan) -> Result<()> {
	let val = val.replace(".em", "em");
	StyleSheet::parse(&val, ParserOptions::default())
		.map(|_| ())
		.map_err(|err| {
			let mut line = span.start_line();
			let mut column = span.start_col();
			// attempt to find error location probs wrong but best we can do i guess
			// remember proc_macro lines are 1 based, lightningcss is 0 based
			if let Some(err_loc) = err.loc {
				line += err_loc.line;
				if err_loc.line == 0 {
					column += err_loc.column;
				}
			}
			anyhow::anyhow!(
				"CSS Error at approx ({}:{}): {}",
				line,
				column,
				err.kind.to_string()
			)
		})
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::node::FileSpan;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(validate_css("body { color: red; }", &FileSpan::default()))
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
