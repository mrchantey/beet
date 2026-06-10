//! Proves the `rsx!`/`.bsx` equivalence gate: the same UI authored two ways,
//! in-Rust with `rsx!` and as a hand-written `.bsx` file, builds the identical
//! template tree. Both front-ends lower onto the one substrate (`spawn_template`)
//! and render to byte-identical HTML.
//!
//! ```sh
//! cargo run --example bsx_rsx_match --features template
//! ```
use beet::prelude::*;

/// The `.bsx` source, embedded so the example runs from any directory.
const CARD_BSX: &str = include_str!("../assets/bsx/card.bsx");

fn main() {
	// 1. Build the UI from the `.bsx` file: parse the source into a fresh
	// container entity, whose first child is the parsed `<article>` root.
	let mut bsx_world = ui_world();
	let container = {
		let bytes = MediaBytes::new_bsx(CARD_BSX);
		let mut entity = bsx_world.spawn_empty();
		BsxParser::bsx()
			.parse(ParseContext::new(&mut entity, &bytes))
			.unwrap();
		entity.id()
	};
	let bsx_root = bsx_world.entity(container).get::<Children>().unwrap()[0];
	let bsx_html = render_html(&mut bsx_world, bsx_root);

	// 2. Build the identical UI from `rsx!`, lowered through the same
	// `spawn_template` substrate.
	let mut rsx_world = ui_world();
	let rsx_root = rsx_world
		.spawn_template(rsx! {
			<article class="card">
				<h1>"Beet"</h1>
				<p>"A user-modifiable framework."</p>
				<input type="text" name="email" placeholder="you@example.com"/>
			</article>
		})
		.unwrap()
		.id();
	let rsx_html = render_html(&mut rsx_world, rsx_root);

	// 3. The two trees must render identically. The `.bsx` file is authored on
	// multiple lines, so it carries insignificant inter-tag whitespace that the
	// token-sourced `rsx!` does not; collapsing it leaves the meaningful tree,
	// which must match exactly. This is the gate.
	let bsx_collapsed = collapse_inter_tag_whitespace(&bsx_html);
	assert_eq!(
		bsx_collapsed, rsx_html,
		"`.bsx` and `rsx!` produced different trees:\n  bsx: {bsx_collapsed}\n  rsx: {rsx_html}"
	);

	println!("rsx!/.bsx trees match:\n{rsx_html}");
}

/// Render `root` to an HTML string through the substrate's [`HtmlRenderer`].
fn render_html(world: &mut World, root: Entity) -> String {
	HtmlRenderer::new()
		.render(&mut RenderContext::new(root, world))
		.unwrap()
		.to_string()
}

/// Drop whitespace runs that sit entirely between two tags (`>   <` -> `><`),
/// the insignificant formatting whitespace a multi-line `.bsx` file carries but
/// a token-sourced `rsx!` does not. Whitespace inside text content is untouched.
fn collapse_inter_tag_whitespace(html: &str) -> String {
	let mut out = String::with_capacity(html.len());
	let bytes = html.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		// at a `>` followed only by whitespace up to the next `<`, skip the run.
		if bytes[i] == b'>' {
			let mut j = i + 1;
			while j < bytes.len() && bytes[j].is_ascii_whitespace() {
				j += 1;
			}
			if j < bytes.len() && bytes[j] == b'<' {
				out.push('>');
				i = j;
				continue;
			}
		}
		out.push(bytes[i] as char);
		i += 1;
	}
	out
}
