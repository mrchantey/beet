use beet_rsx::as_beet::*;




pub fn get() -> RsxNode {
	rsx! {
		<h1>This is a H1 Heading</h1>
		<h2>This is a H2 Heading</h2>
		<h3>This is a H3 Heading</h3>
		<h4>This is a H4 Heading</h4>
		<h5>This is a H5 Heading</h5>
		<h6>This is a H6 Heading</h6>
		<p>This is a paragraph</p>
		<p>This is a paragraph with <a href="https://example.com">a link</a></p>
		<p>This is a paragraph with <strong>strong text</strong></p>// aka bold
		<p>This is a paragraph with <em>emphasized text</em></p>// aka italic
		<p>This is a paragraph with <code>inline code</code></p>// aka monospace
		<p>This is a paragraph with <mark>highlighted text</mark></p>// aka highlighted
		<p>This is a paragraph with <del>deleted text</del></p>// aka strikethrough
		<p>This is a paragraph with <ins>inserted text</ins></p>// aka underline
		<p>This is a paragraph with <small>small text</small></p>
		<p>This is a paragraph with <sub>subscript text</sub></p>
		<p>This is a paragraph with <sup>superscript text</sup></p>
		<p>This is a paragraph with <blockquote>blockquoted text</blockquote></p>
		<p>This is a paragraph with <q>quoted text</q></p>
		<p>This is a paragraph with <cite>citation text</cite></p>
		<p>This is a paragraph with <abbr title="abbreviation">abbreviated text</abbr></p>
		<p>This is a paragraph with <time datetime="2023-10-01">time text</time></p>
		<p>This is a paragraph with <address>address text</address></p>
		<p>This is a paragraph with <bdi>bdi text</bdi></p>
		<p>This is a paragraph with <bdo dir="rtl">bdo text</bdo></p>
		<p>This is a paragraph with <samp>samp text</samp></p>
		<p>This is a paragraph with <kbd>kbd text</kbd></p>
		<p>This is a paragraph with <var>var text</var></p>
		<p>This is a paragraph with <dfn>dfn text</dfn></p>
		<p>This is a very long paragraph that should be broken into multiple lines to test the line height and spacing between lines. This is a very long paragraph that should be broken into multiple lines to test the line height and spacing between lines. This is a very long paragraph that should be broken into multiple lines to test the line height and spacing between lines. This is a very long paragraph that should be broken into multiple lines to test the line height and spacing between lines.</p>

	}
}
