//! Demonstrates the char-cell inline formatting context: a block container
//! flows its inline descendants (text, emphasis, links, inline code) into
//! wrapped, styled lines, while `<pre>` preserves whitespace and newlines.
use beet_core::prelude::*;
use beet_ui::prelude::*;
use beet_ui::*;
use bevy::math::UVec2;

fn main() {
	println!("=== Beet Inline Formatting Demo ===");

	render("Mixed inline runs", 40, mixed_runs);
	render("Wrapping a long paragraph", 24, wrapping);
	render("Nested emphasis", 40, nested_emphasis);
	render("Inline link + code", 40, link_and_code);
	render("Heading then paragraph (block stack)", 40, block_stack);
	render("Preformatted block", 30, preformatted);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn render<B: Bundle>(name: &str, width: u32, setup: fn() -> B) {
	let out = Buffer::render_oneshot_sized(UVec2::new(width, 12), setup())
		.trim_lines();
	println!("\n{name} (width {width}):\n{out}");
}

// ── Demos ───────────────────────────────────────────────────────────────────

fn mixed_runs() -> impl Bundle {
	rsx_direct!{
		<p>"Plain text, "<em>"emphasised"</em>", "<strong>"strong"</strong>
			" and "<code>"inline_code()"</code>" on one flowing line."</p>
	}
}

fn wrapping() -> impl Bundle {
	rsx_direct!{
		<p>"This paragraph is wider than the column, so it wraps onto several
			lines at word boundaries."</p>
	}
}

fn nested_emphasis() -> impl Bundle {
	rsx_direct!{ <p>"A "<em><strong>"bold italic"</strong></em>" phrase."</p> }
}

fn link_and_code() -> impl Bundle {
	rsx_direct!{
		<p>"See "<a href="https://beetstack.dev">"the docs"</a>
			" or run "<code>"cargo test"</code>"."</p>
	}
}

fn block_stack() -> impl Bundle {
	rsx_direct!{
		<div>
			<h2>"A Heading"</h2>
			<p>"Followed by a paragraph that flows its own inline content
				independently of the heading above."</p>
		</div>
	}
}

fn preformatted() -> impl Bundle {
	rsx_direct!{
		<pre>"fn main() {\n    let x = 1;\n    println!(\"{x}\");\n}"</pre>
	}
}
