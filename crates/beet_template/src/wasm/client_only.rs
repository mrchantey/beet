use crate::prelude::*;
use beet_common::node::ClientOnlyDirective;
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;

// all client-only nodes need to render html
// we cant require because downstream crate
pub(super) fn on_add_client_only(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).insert(ToHtml);
}

pub(super) fn mount_html(
	ev: Trigger<OnAdd, RenderedHtml>,
	query: Populated<&RenderedHtml, With<ClientOnlyDirective>>,
) {
	if let Ok(html) = query.get(ev.target()) {
		mount(&html.0);
	}
}
/// ensure all text nodes are collapsed, critical when mounting
/// nodes via append_child
#[allow(unused)]
fn normalize() { web_sys::window().unwrap().document().unwrap().normalize(); }

fn mount(html: &str) {
	// let html = bundle_to_html(bundle);
	let document = web_sys::window().unwrap().document().unwrap();
	let body = document.body().unwrap();
	let current_html = body.inner_html();
	body.set_inner_html(&format!("{}{}", current_html, html));
}

#[allow(unused)]
fn mount_with_id(html: &str, id: &str) {
	let document = web_sys::window().unwrap().document().unwrap();
	let element = document.get_element_by_id(id).unwrap();
	element.set_inner_html(&html);
}



#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	// we cant do anything with this because
	// deno no document, eventually we should mock the dom
	fn works() {
		App::new()
			.add_plugins(TemplatePlugin)
			.add_systems(Startup, setup)
			.run();
	}

	fn setup(mut commands: Commands) {
		commands.spawn(rsx! {
			<Counter client:only initial=7/>
		});
	}

	#[template]
	#[derive(serde::Serialize)]
	fn Counter(initial: u32) -> impl Bundle {
		let (get, set) = signal(initial);

		rsx! {
				<p>Count: {get}</p>
				<button onclick={move ||set(get()+1)}>Increment</button>
		}
	}
}
