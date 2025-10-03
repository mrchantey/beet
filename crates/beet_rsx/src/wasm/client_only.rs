use crate::prelude::*;
use beet_core::prelude::*;
use beet_dom::prelude::*;

pub(crate) fn mount_client_only(
	query: Populated<
		(Entity, &ClientOnlyDirective),
		Added<ClientOnlyDirective>,
	>,
	mut diff: DomDiff,
) -> Result {
	for (entity, directive) in query.iter() {
		let root = web_sys::window()
			.unwrap()
			.document()
			.unwrap()
			.get_element_by_id(&directive.root_id)
			.ok_or_else(|| {
				bevyhow!(
					"Client Only: Root Element with id '{}' not found",
					&directive.root_id
				)
			})?;
		// the entity is a SnippetRoot not an element so only diff children
		diff.diff_children(entity, &root)?;
	}
	Ok(())
}
// /// ensure all text nodes are collapsed, critical when mounting
// /// nodes via append_child
// #[allow(unused)]
// fn normalize() { web_sys::window().unwrap().document().unwrap().normalize(); }

// fn mount(html: &str) {
// 	// let html = HtmlFragment::parse_bundle(bundle);
// 	let document = web_sys::window().unwrap().document().unwrap();
// 	let body = document.body().unwrap();
// 	let current_html = body.inner_html();
// 	body.set_inner_html(&format!("{}{}", current_html, html));
// }

// #[allow(unused)]
// fn mount_with_id(html: &str, id: &str) {
// 	let document = web_sys::window().unwrap().document().unwrap();
// 	let element = document.get_element_by_id(id).unwrap();
// 	element.set_inner_html(&html);
// }

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	// use sweet::prelude::*;

	#[test]
	// we cant do anything with this because
	// deno no document, eventually we should mock the dom
	fn works() {
		App::new()
			.add_plugins(ApplyDirectivesPlugin)
			.add_systems(Startup, setup)
			.set_runner(ReactiveApp::runner)
			.run();
	}

	fn setup(mut commands: Commands) {
		commands.spawn(rsx! { <Counter client:only initial=7 /> });
	}

	#[template]
	#[derive(Reflect)]
	fn Counter(initial: u32) -> impl Bundle {
		let (get, set) = signal(initial);

		rsx! {
			<p>Count: {get}</p>
			<button onclick=move || set(get() + 1)>Increment</button>
		}
	}
}
