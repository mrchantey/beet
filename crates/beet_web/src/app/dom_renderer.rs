use beet::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::utils::HashMap;
use forky_web::DocumentExt;
use web_sys::Document;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;

pub fn has_renderer(world: &World) -> bool {
	world.get_non_send_resource::<DomRenderer>().is_some()
}

#[derive(Component)]
pub struct HasElement;

#[derive(Clone)]
pub struct DomRenderer {
	pub container: HtmlElement,
	pub elements: HashMap<Entity, HtmlDivElement>,
}

impl DomRenderer {
	pub fn new(container: HtmlElement) -> Self {
		Self {
			container,
			elements: HashMap::default(),
		}
	}

	pub fn clear(&mut self) {
		for el in self.elements.values() {
			el.remove();
		}
		self.elements.clear();
	}
}

#[derive(Resource)]
pub struct DomSimRendererMarker;

pub fn despawn_elements(
	mut renderer: NonSendMut<DomRenderer>,
	mut removed: RemovedComponents<Transform>,
) {
	for removed in removed.read() {
		if let Some(el) = renderer.elements.remove(&removed) {
			el.remove();
		}
	}
}

pub fn create_elements(
	mut commands: Commands,
	mut renderer: NonSendMut<DomRenderer>,
	query: Query<(Entity, &RenderText), Without<HasElement>>,
) {
	for (entity, text) in query.iter() {
		let div = Document::x_create_div();
		div.set_inner_text(text.0.as_ref());
		div.set_class_name("entity");
		renderer.container.append_child(&div).unwrap();
		renderer.elements.insert(entity, div);
		commands.entity(entity).insert(HasElement);
	}
}


pub fn clear_world_with_dom_renderer(world: &mut World) {
	world.clear_entities();
	world.non_send_resource_mut::<DomRenderer>().clear();
}

pub fn remove_renderer(world: &mut World) {
	for entity in world
		.query_filtered::<Entity, With<HasElement>>()
		.iter(world)
		.collect::<Vec<_>>()
	{
		world.entity_mut(entity).remove::<HasElement>();
	}
	if let Some(mut renderer) = world.remove_non_send_resource::<DomRenderer>()
	{
		renderer.clear();
	}
}

pub fn trigger_transform_change(world: &mut World) {
	// dont use has element filter, none have it if we just turned renderer back on
	for mut transform in world.query::<&mut Transform>().iter_mut(world) {
		//trigger change
		let _changed = transform.as_mut();
	}
}

pub fn add_renderer(world: &mut World, container: &HtmlElement) {
	world.insert_non_send_resource(DomRenderer::new(container.clone()));
	trigger_transform_change(world);
}
