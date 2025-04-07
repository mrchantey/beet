// TODO use this with chumsky for rsx parsing
// pub enum HtmlNode<'a> {
// 	/// A doctype declaration, ie `<!DOCTYPE html>`
// 	Doctype {
// 		/// The language, ie `html`
// 		value: &'a str,
// 		/// The total span of the doctype
// 		/// including the `<!DOCTYPE ...>` part
// 		span: SimpleSpan<usize>,
// 	},
// 	/// An rsx fragment, ie `<>foo</>`
// 	Fragment {
// 		/// The children of the fragment
// 		children: Vec<HtmlNode<'a>>,
// 		/// The total span of the fragment
// 		span: SimpleSpan<usize>,
// 	},
// 	/// a comment, ie `<!-- foo -->`
// 	Comment {
// 		/// The content of the comment,
// 		value: &'a str,
// 		/// The total span of the comment
// 		/// including the `<!-- ... -->` part
// 		span: SimpleSpan<usize>,
// 	},
// 	/// A text node, ie `<div>foo</div>`
// 	Text {
// 		/// The content of the text node
// 		value: &'a str,
// 		/// The total span of the text node
// 		span: SimpleSpan<usize>,
// 	},
// 	/// A block containing more rsx, ie
// 	/// <outer>{foo}</outer>
// 	Block {
// 		/// The content of the block node
// 		value: &'a str,
// 		/// The total span of the block node
// 		span: SimpleSpan<usize>,
// 	},
// 	/// A tagged element, ie
// 	/// <div>foo</div>
// 	Element {
// 		/// The name of the element, ie `div`
// 		tag: &'a str,
// 		/// The attributes of the element
// 		attributes: Vec<HtmlAttribute<'a>>,
// 		/// The content of the element, ie `foo`
// 		children: Vec<HtmlNode<'a>>,
// 		/// The total span of the element, including
// 		/// both the opening and closing tags
// 		span: SimpleSpan<usize>,
// 	},
// }

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use chumsky::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn hello_chumsky() {
		// expect(parse_html().parse("").into_result()).to_be_ok();
		// expect(parse_html().parse("123").has_errors()).to_be_false();
	}
}
