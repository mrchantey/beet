use super::Spanner;
use super::WebTokens;
use anyhow::Result;
use lightningcss::stylesheet::ParserOptions;
use lightningcss::stylesheet::StyleSheet;
use sweet::prelude::Pipeline;
use syn::spanned::Spanned;
pub struct ValidateStyleNode;

impl Pipeline<WebTokens, Result<WebTokens>> for ValidateStyleNode {
	fn apply(self, node: WebTokens) -> Result<WebTokens> {
		// doesnt need to be mut but no ref visitor
		node.walk_web_tokens(|html| match html {
			WebTokens::Element {
				component,
				children,
				..
			} if component.tag.to_string() == "style"
				&& let WebTokens::Text { value, .. } = &**children =>
			{
				validate_css(&value.value.value(), &value)
			}
			_ => Ok(()),
		})?;
		Ok(node)
	}
}

pub fn validate_css<'a, T: Spanned>(
	val: &'a str,
	source: &Spanner<T>,
) -> Result<()> {
	let val = val.replace(".em", "em");
	StyleSheet::parse(&val, ParserOptions::default())
		.map(|_| ())
		.map_err(|err| {
			let mut linecol = source.start();
			// attempt to find error location probs wrong but best we can do i guess
			// remember proc_macro lines are 1 based, lightningcss is 0 based
			if let Some(err_loc) = err.loc {
				linecol.line += err_loc.line as usize;
				if err_loc.line == 0 {
					linecol.column += err_loc.column as usize;
				}
			}
			anyhow::anyhow!(
				"CSS Error at approx ({}:{}): {}",
				linecol.line,
				linecol.column,
				err.kind.to_string()
			)
		})
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;
	use syn::LitStr;

	#[test]
	fn works() {
		expect(validate_css(
			"body { color: red; }",
			&Spanner::<LitStr>::new(syn::parse_quote!("foo")),
		))
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
