use beet::prelude::*;

/// Showcases the [`Button`] and [`Link`] variants, laid out by the site-local
/// typed [`Rule`](crate::style::design_row_rule) (`design-row`).
pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Buttons"</h1>
			<p>
				"The library widget set, laid out side by side by a typed "
				<code>"Rule"</code>" (the Rust twin of a markup "<code>"<Rule>"</code>")."
			</p>
			<h2>"Buttons"</h2>
			<div {Classes::new([crate::style::classes::DESIGN_ROW])}>
				<Button variant=ButtonVariant::Filled>"Filled"</Button>
				<Button variant=ButtonVariant::Tonal>"Tonal"</Button>
				<Button variant=ButtonVariant::Elevated>"Elevated"</Button>
				<Button variant=ButtonVariant::Outlined>"Outlined"</Button>
				<Button variant=ButtonVariant::Text>"Text"</Button>
			</div>
			<h2>"Links"</h2>
			<div {Classes::new([crate::style::classes::DESIGN_ROW])}>
				<Link href="#" variant=ButtonVariant::Filled>"Filled"</Link>
				<Link href="#" variant=ButtonVariant::Tonal>"Tonal"</Link>
				<Link href="#" variant=ButtonVariant::Elevated>"Elevated"</Link>
				<Link href="#" variant=ButtonVariant::Outlined>"Outlined"</Link>
				<Link href="#" variant=ButtonVariant::Text>"Text"</Link>
			</div>
		</article>
	}
}
