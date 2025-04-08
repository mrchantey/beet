#![allow(unused)]
use crate::prelude::*;
use chumsky::prelude::*;
use sweet::prelude::PipelineTarget;

fn self_closing_element<'src>()
-> impl Parser<'src, &'src str, JsxNode, extra::Err<Rich<'src, char>>> {
	just('<')
		.then(
			// parse the tag name
			any()
				.filter(|c: &char| !c.is_whitespace())
				.and_is(just("/>").not())
				.repeated()
				.collect::<String>()
				.map_with(|s, meta| SpannedStr::new(s, meta.span())),
		)
		.map(|v: (char, SpannedStr)| v) //type check
		.then_ignore(text::whitespace())
		.then(key_attribute().repeated().collect::<Vec<JsxAttribute>>())
		.map(|v: ((char, SpannedStr), Vec<JsxAttribute>)| v) //type check
		.then_ignore(text::whitespace())
		.then(just("/>"))
		.map_with(|s: (((char, SpannedStr), Vec<JsxAttribute>), &str), meta| {
			JsxNode::Element {
				tag: s.0.0.1,
				attributes: s.0.1,
				children: Vec::new(),
				self_closing: true,
				span: meta.span(),
			}
		})
}
fn key_attribute<'src>()
-> impl Parser<'src, &'src str, JsxAttribute, extra::Err<Rich<'src, char>>> {
	any()
		.filter(|c: &char| {
			c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ':'
		})
		.repeated()
		.at_least(1)
		.collect::<String>()
		.map_with(|s, meta| JsxAttribute::Key {
			key: SpannedStr::new(s, meta.span()),
		})
		.padded()
}

fn key_value_attribute<'src>()
-> impl Parser<'src, &'src str, JsxAttribute, extra::Err<Rich<'src, char>>> {
	key_attribute()
		.then_ignore(just("=\""))
		.then(
			just('"')
				.ignore_then(
					any().filter(|c| *c != '"').repeated().collect::<String>(),
				)
				.then_ignore(just('"'))
				.or(just('\'')
					.ignore_then(
						any()
							.filter(|c| *c != '\'')
							.repeated()
							.collect::<String>(),
					)
					.then_ignore(just('\'')))
				.map_with(|s, meta| SpannedStr::new(s, meta.span()))
				.or_not(),
		)
		.map_with(|(key, value), meta| JsxAttribute::KeyValueStr {
			key,
			value,
			span: meta.span(),
		})
}

// // Helper function to parse a single attribute
// fn key_value_attribute<'src>()
// -> impl Parser<'src, &'src str, JsxAttribute, extra::Err<Rich<'src, char>>> {
//     let name = any()
//         .filter(|c: &char| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == ':')
//         .repeated()
//         .at_least(1)
//         .collect::<String>()
//         .map_with(|s, meta| SpannedStr::new(s, meta.span()));

//     let value = just('=').ignore_then(
//         just('"')
//             .ignore_then(
//                 any()
//                     .filter(|c| *c != '"')
//                     .repeated()
//                     .collect::<String>()
//             )
//             .then_ignore(just('"'))
//             .or(
//                 just('\'')
//                     .ignore_then(
//                         any()
//                             .filter(|c| *c != '\'')
//                             .repeated()
//                             .collect::<String>()
//                     )
//                     .then_ignore(just('\''))
//             )
//             .map_with(|s, meta| SpannedStr::new(s, meta.span()))
//     );

//     name.then(value.or_not())
//         .map_with(|(name, value), meta| JsxAttribute {
//             name,
//             value,
//             span: meta.span(),
//         })
//         .padded()
// }


fn test(input: &str) -> Result<JsxNode, (Option<JsxNode>, String)> {
	self_closing_element()
		.parse(input)
		.xmap(|result| with_pretty_errors(input, result))
}
fn into(input: &str) -> String {
	test(input)
		.map_err(|err| {
			println!("Error: {}", err.1);
		})
		.ok()
		.unwrap()
		.to_string()
}

pub const FOO: &str = "foo";

#[cfg(test)]
mod test {
	use super::*;
	// use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		expect(into("<hello/>")).to_be("<hello/>");
		expect(into("<hello \n />")).to_be("<hello/>");
		expect(into("<hello \n />")).to_be("<hello/>");
		expect(into("<hello world/>")).to_be("<hello world/>");
		expect(into("<hello   world   world \n  />"))
			.to_be("<hello world world/>");
	}
}
