use ariadne::Color;
use ariadne::Label;
use ariadne::Report;
use ariadne::ReportKind;
use ariadne::Source;
use chumsky::prelude::*;
use std::fmt::Formatter;
use std::io::Cursor;


/// A string with a span
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedStr {
	/// The string value
	pub value: String,
	/// The span of the string
	pub span: SimpleSpan<usize>,
}
impl SpannedStr {
	pub fn new(value: String, span: SimpleSpan<usize>) -> Self {
		SpannedStr { value, span }
	}
}

impl Into<SpannedStr> for (String, SimpleSpan<usize>) {
	fn into(self) -> SpannedStr {
		SpannedStr {
			value: self.0,
			span: self.1,
		}
	}
}

impl std::fmt::Display for SpannedStr {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.value)
	}
}

/// JSX Nodes as defined in the [JSX spec](https://facebook.github.io/jsx/)
#[derive(Debug, Clone, PartialEq)]
pub enum JsxNode {
	/// A doctype declaration, ie `<!DOCTYPE html>`
	Doctype {
		/// The language, ie `html`
		value: SpannedStr,
		/// The total span of the doctype
		/// including the `<!DOCTYPE ...>` part
		span: SimpleSpan<usize>,
	},
	/// An rsx fragment, ie `<>foo</>`
	Fragment {
		/// The children of the fragment
		children: Vec<JsxNode>,
		/// The total span of the fragment
		span: SimpleSpan<usize>,
	},
	/// a comment, ie `<!-- foo -->`
	Comment {
		/// The content of the comment,
		value: SpannedStr,
		/// The total span of the comment
		/// including the `<!-- ... -->` part
		span: SimpleSpan<usize>,
	},
	/// A text node, ie `<div>foo</div>`
	Text {
		/// The content of the text node
		value: SpannedStr,
	},
	/// A block containing more rsx, ie
	/// <outer>{foo}</outer>
	Block {
		/// The content of the block node
		value: SpannedStr,
	},
	/// A tagged element, ie
	/// <div>foo</div>
	Element {
		/// The name of the element, ie `div`
		tag: SpannedStr,
		/// The attributes of the element
		attributes: Vec<JsxAttribute>,
		/// The content of the element, ie `foo`
		children: Vec<JsxNode>,

		self_closing: bool,
		/// The total span of the element, including
		/// both the opening and closing tags
		span: SimpleSpan<usize>,
	},
}

impl std::fmt::Display for JsxNode {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			JsxNode::Doctype { value, .. } => {
				write!(f, "<!DOCTYPE {}>", value)
			}
			JsxNode::Fragment { children, .. } => {
				write!(f, "<>")?;
				for child in children {
					write!(f, "{}", child)?;
				}
				write!(f, "</>")
			}
			JsxNode::Comment { value, .. } => write!(f, "<!-- {} -->", value),
			JsxNode::Text { value, .. } => write!(f, "{}", value),
			JsxNode::Block { value, .. } => write!(f, "{{{}}}", value),
			JsxNode::Element {
				tag,
				attributes,
				children,
				self_closing,
				span: _,
			} => {
				write!(f, "<{}", tag)?;
				for attr in attributes {
					write!(f, " {}", attr)?;
				}
				if *self_closing {
					write!(f, "/>")
				} else {
					write!(f, ">")?;
					for child in children {
						write!(f, "{}", child)?;
					}
					write!(f, "</{}>", tag)
				}
			}
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub enum JsxAttribute {
	/// An attribute without a value, ie `foo`
	Key { key: SpannedStr },
	/// A html attribute with a value, ie `foo="bar"`
	KeyValueStr {
		key: SpannedStr,
		value: SpannedStr,
		/// the total span of the attribute
		span: SimpleSpan<usize>,
	},
}

impl std::fmt::Display for JsxAttribute {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			JsxAttribute::Key { key } => write!(f, "{}", key),
			JsxAttribute::KeyValueStr { key, value, .. } => {
				write!(f, "{}=\"{}\"", key, value)
			}
		}
	}
}

pub fn parse_jsx<'src>()
-> impl Parser<'src, &'src str, Vec<JsxNode>, extra::Err<Rich<'src, char>>> {
	// let doctype = any()
	// 	.and_is(none_of(" >"))
	// 	.padded()
	// 	.repeated()
	// 	.to_slice()
	// 	// .padded()
	// 	.then_ignore(just('>').padded())
	// 	.map_with(|val: &str, meta| JsxNode::Doctype {
	// 		value: SpannedStr::new(val.trim(), meta.span()),
	// 		span: meta.span(),
	// 	})
	// 	.delimited_by(just("<!DOCTYPE"), just(">"));
	#[allow(unused)]
	recursive(|_value| {
		let doctype = just("<!DOCTYPE")
			.padded()
			.then(any().and_is(none_of(" >")).repeated().to_slice().padded())
			.then_ignore(just('>').padded())
			.map_with(|s: (&str, &str), meta| JsxNode::Doctype {
				value: SpannedStr::new(s.1.to_string(), meta.span()),
				span: meta.span(),
			});

		// let comment = just("<!--")
		// 	.padded()
		// 	.then(any().and_is(none_of(" -->")).repeated().to_slice().padded())
		// 	.then_ignore(just("-->").padded())
		// 	.map_with(|s, meta| JsxNode::Comment {
		// 		value: SpannedStr::new(s.1, meta.span()),
		// 		span: meta.span(),
		// 	});
		// let fragment = just("<>")
		// 	.padded()
		// 	.then(any().and_is(none_of(" <>")).repeated().to_slice().padded())
		// 	.then_ignore(just("<>").padded())
		// 	.map_with(|s: &str, meta| JsxNode::Comment {
		// 		value: SpannedStr::new(s.1.to_string(), meta.span()),
		// 		span: meta.span(),
		// 	});


		choice((
			doctype, doctype,
			// comment,
			//  fragment
		))
		.repeated()
		.collect()
	})
	// doctype.repeated()
}

pub fn with_pretty_errors<'a, O>(
	src: &str,
	result: ParseResult<O, Rich<'a, char>>,
) -> Result<O, (Option<O>, String)> {
	let (val, err) = result.into_output_errors();
	let mut out_err = String::new();
	let no_errors = err.is_empty();
	err.into_iter().for_each(|e| {
		let mut buffer = Vec::new();
		let writer = Cursor::new(&mut buffer);
		Report::build(ReportKind::Error, (), e.span().start)
			.with_message(e.to_string())
			.with_label(
				Label::new(e.span().into_range())
					.with_message(e.reason().to_string())
					.with_color(Color::Red),
			)
			.finish()
			.write(Source::from(src), writer)
			.expect("Failed to write error report");
		out_err.push_str(&String::from_utf8_lossy(&buffer));
	});
	if no_errors && let Some(val) = val {
		Ok(val)
	} else {
		Err((val, out_err))
	}
}



pub fn prettify_errors<'a>(src: &'a str, errs: Vec<Rich<'a, char>>) {
	errs.into_iter().for_each(|e| {
		Report::build(ReportKind::Error, (), e.span().start)
			.with_message(e.to_string())
			.with_label(
				Label::new(e.span().into_range())
					.with_message(e.reason().to_string())
					.with_color(Color::Red),
			)
			.finish()
			.print(Source::from(&src))
			.unwrap()
	});
}

#[cfg(test)]
mod test {
	#![allow(unused)]
	use super::*;
	use crate::prelude::*;
	use chumsky::prelude::*;
	use itertools::Itertools;
	use sweet::prelude::*;

	fn test(val: &str) -> String {
		let (output, errors) = parse_jsx().parse(val).into_output_errors();
		prettify_errors(val, errors);
		output
			.map(|o| o.into_iter().map(|o| o.to_string()).join(""))
			.unwrap_or_default()
	}

	#[test]
	fn hello_chumsky() {
		// doctype
		expect(test("<!DOCTYPE html>")).to_be("<!DOCTYPE html>");
		expect(test("<!DOCTYPE  html>")).to_be("<!DOCTYPE html>");
		expect(test("<!DOCTYPE  html >")).to_be("<!DOCTYPE html>");
		expect(test("  <!DOCTYPE  foobar >   ")).to_be("<!DOCTYPE foobar>");

		expect(test("<!DOCTYPE html><!DOCTYPE html>"))
			.to_be("<!DOCTYPE html><!DOCTYPE html>");
		expect(test("   <!DOCTYPE html>    <!DOCTYPE html>     "))
			.to_be("<!DOCTYPE html><!DOCTYPE html>");

		// comment
		expect(test("<!-- foo -->")).to_be("<!-- foo -->");
		expect(test("\n<!--    foo  \n  -->\t")).to_be("<!-- foo -->");



		// expect(
		// 	parse_doctype()
		// 		.parse("<!DOCTYPE html>")
		// 		.into_output()
		// 		.unwrap()
		// 		.to_string(),
		// )
		// .to_be("foobar");

		// let any = any().and_is(just('>').not()).repeated().padded();

		expect(
			any::<&str, extra::Err<Simple<char>>>()
				.repeated()
				.to_slice()
				.parse("foo")
				.into_result()
				.unwrap(),
		)
		.to_be("foo");

		// expect(parse_doctype().parse("<!DOCTYPE html>").has_errors()).to_be_false();

		// expect(parse_jsx().parse("").into_result()).to_be_ok();
		// expect(parse_jsx().parse("123").has_errors()).to_be_false();
	}
}
