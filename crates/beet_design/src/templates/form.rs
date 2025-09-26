use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::DynamicStruct;



#[template]
pub fn Form(
	/// by default we call prevent_default on trigger, setting this will
	/// not call prevent_default, allowing onsubmit events to bubble up
	#[field(default)]
	bubble_up: bool,
	onsubmit_dyn: Box<dyn 'static + Send + Sync + Fn(DynamicStruct)>,
	#[field(flatten)] attrs: BaseHtmlAttributes,
) -> impl Bundle {
	// let (entity, set_entity) = signal(Entity::PLACEHOLDER);

	#[cfg(target_arch = "wasm32")]
	let onsubmit = move |ev: On<OnSubmit>| {
		use beet_core::exports::js_sys;
		use beet_core::exports::wasm_bindgen::JsCast;
		use beet_core::exports::web_sys;

		if !bubble_up {
			ev.prevent_default();
		}

		let form = ev
			.current_target()
			.unwrap()
			.dyn_into::<web_sys::HtmlFormElement>()
			.unwrap();

		let form_data = web_sys::FormData::new_with_form(&form).unwrap();

		let mut dyn_struct = DynamicStruct::default();

		let entries = js_sys::try_iter(&form_data).unwrap().unwrap();
		for entry in entries {
			let entry = entry.unwrap();
			let arr = js_sys::Array::from(&entry);
			let key = arr.get(0).as_string().unwrap();
			let value = arr.get(1).as_string().unwrap_or_default();
			dyn_struct.insert(key, value);
		}
		onsubmit_dyn(dyn_struct);
	};
	#[cfg(not(target_arch = "wasm32"))]
	let onsubmit = |_: On<OnSubmit>| {};

	rsx! {
		<form {attrs} onsubmit=onsubmit>
			<slot />
		</form>
		<style src="form.css" />
	}
}


#[cfg(test)]
mod test {
	use bevy::prelude::*;
	use bevy::reflect::DynamicStruct;
	use sweet::prelude::*;

	#[test]
	fn works() {
		#[derive(Reflect)]
		struct MyStruct {
			foo: i32,
		}

		let mut dynamic = DynamicStruct::default();
		MyStruct::from_reflect(&dynamic).is_none().xpect_true();
		dynamic.insert("foo", 3);
		MyStruct::from_reflect(&dynamic).unwrap().foo.xpect_eq(3);
	}
}
