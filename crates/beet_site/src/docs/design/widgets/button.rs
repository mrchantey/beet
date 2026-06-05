use beet::prelude::*;

/// Showcases every [`Button`] and [`Link`] variant.
pub fn get() -> impl Scene {
	rsx! {
		<article>
			<h1>"Button"</h1>
			<h2>"Variants"</h2>
			<div {Classes::new(["design-row"])}>
				<Button label="Filled" variant=ButtonVariant::Filled/>
				<Button label="Secondary" variant=ButtonVariant::Secondary/>
				<Button label="Tertiary" variant=ButtonVariant::Tertiary/>
				<Button label="Error" variant=ButtonVariant::Error/>
				<Button label="Tonal" variant=ButtonVariant::Tonal/>
				<Button label="Elevated" variant=ButtonVariant::Elevated/>
				<Button label="Outlined" variant=ButtonVariant::Outlined/>
				<Button label="Text" variant=ButtonVariant::Text/>
			</div>
			<h2>"Links"</h2>
			<div {Classes::new(["design-row"])}>
				<Link label="Filled" href="#" variant=ButtonVariant::Filled/>
				<Link label="Secondary" href="#" variant=ButtonVariant::Secondary/>
				<Link label="Tertiary" href="#" variant=ButtonVariant::Tertiary/>
				<Link label="Error" href="#" variant=ButtonVariant::Error/>
				<Link label="Outlined" href="#" variant=ButtonVariant::Outlined/>
				<Link label="Text" href="#" variant=ButtonVariant::Text/>
			</div>
		</article>
	}
}
