use crate::prelude::*;
use bevy::prelude::*;

pub fn get() -> impl Bundle {
	rsx! { <Inner client:load /> }
}

#[template]
#[derive(Reflect)]
pub fn Inner() -> impl Bundle {
	let (value, set_value) = signal("veggie".to_string());

	effect(move || {
		beet_utils::log!("value: {}", value());
	});

	rsx! {
			<h2>Variants</h2>
			<div>
				<label>Favorite Pizza</label>
				<Select value=value onchange=move|e|{set_value(e.value())}>
					<option value="hawaiian">Hawaiian</option>
					<option value="pepperoni">Pepperoni</option>
					<option value="margherita">Margherita</option>
					<option value="veggie">Veggie</option>
				</Select>
			</div>
			<style>
			div{
				padding:1.em;
				display: flex;
				flex-direction: row;
				align-items:flex-start;
				gap: 1rem;
			}
			</style>
	}
}
