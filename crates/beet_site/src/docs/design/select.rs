use beet::prelude::*;

/// Shows the [`Select`] widget variants with `<option>` children.
///
/// The legacy `onchange` signal demo is dropped; the static variants render
/// across web and terminal.
pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Select"</h1>
			<h2>"Variants"</h2>
			<label>"Outlined"</label>
			<Select variant=SelectVariant::Outlined name="pizza-outlined">
				<option value="hawaiian">"Hawaiian"</option>
				<option value="pepperoni">"Pepperoni"</option>
				<option value="margherita">"Margherita"</option>
				<option value="veggie">"Veggie"</option>
			</Select>
			<label>"Filled"</label>
			<Select variant=SelectVariant::Filled name="pizza-filled">
				<option value="hawaiian">"Hawaiian"</option>
				<option value="pepperoni">"Pepperoni"</option>
				<option value="margherita">"Margherita"</option>
				<option value="veggie">"Veggie"</option>
			</Select>
			<label>"Text"</label>
			<Select variant=SelectVariant::Text name="pizza-text">
				<option value="hawaiian">"Hawaiian"</option>
				<option value="pepperoni">"Pepperoni"</option>
				<option value="margherita">"Margherita"</option>
				<option value="veggie">"Veggie"</option>
			</Select>
		</article>
	}
}
