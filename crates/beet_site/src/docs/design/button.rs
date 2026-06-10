use beet::prelude::*;

/// Showcases every [`Button`] and [`Link`] variant.
pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Button"</h1>
			<h2>"Variants"</h2>
			<div {Classes::new([crate::style::classes::DESIGN_ROW])}>
				<Button variant=ButtonVariant::Filled>"Filled"</Button>
				<Button variant=ButtonVariant::Secondary>"Secondary"</Button>
				<Button variant=ButtonVariant::Tertiary>"Tertiary"</Button>
				<Button variant=ButtonVariant::Error>"Error"</Button>
				<Button variant=ButtonVariant::Tonal>"Tonal"</Button>
				<Button variant=ButtonVariant::Elevated>"Elevated"</Button>
				<Button variant=ButtonVariant::Outlined>"Outlined"</Button>
				<Button variant=ButtonVariant::Text>"Text"</Button>
			</div>
			<h2>"Links"</h2>
			<div {Classes::new([crate::style::classes::DESIGN_ROW])}>
				<Link href="#" variant=ButtonVariant::Filled>"Filled"</Link>
				<Link href="#" variant=ButtonVariant::Secondary>"Secondary"</Link>
				<Link href="#" variant=ButtonVariant::Tertiary>"Tertiary"</Link>
				<Link href="#" variant=ButtonVariant::Error>"Error"</Link>
				<Link href="#" variant=ButtonVariant::Tonal>"Tonal"</Link>
				<Link href="#" variant=ButtonVariant::Elevated>"Elevated"</Link>
				<Link href="#" variant=ButtonVariant::Outlined>"Outlined"</Link>
				<Link href="#" variant=ButtonVariant::Text>"Text"</Link>
			</div>
		</article>
	}
}
