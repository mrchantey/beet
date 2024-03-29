use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::utils::HashMap;
use forky_web::DocumentExt;
use web_sys::Document;
use web_sys::HtmlDivElement;

pub fn has_renderer(world: &World) -> bool {
	world.get_non_send_resource::<DomRenderer>().is_some()
}


#[derive(Component, Deref, DerefMut)]
pub struct DomText(pub String);
#[derive(Component)]
pub struct HasElement;

#[derive(Clone)]
pub struct DomRenderer {
	pub container: HtmlDivElement,
	pub elements: HashMap<Entity, HtmlDivElement>,
}

impl DomRenderer {
	pub fn new(container: HtmlDivElement) -> Self {
		Self {
			container,
			elements: HashMap::default(),
		}
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
	query: Query<(Entity, &DomText), Without<HasElement>>,
) {
	for (entity, text) in query.iter() {
		let div = Document::x_create_div();
		div.set_inner_text(text.as_str());
		div.set_class_name("entity");
		renderer.container.append_child(&div).unwrap();
		renderer.elements.insert(entity, div);
		commands.entity(entity).insert(HasElement);
	}
}


pub fn remove_renderer(world: &mut World) {
	world.remove_non_send_resource::<DomRenderer>();
	for entity in world
		.query_filtered::<Entity, With<HasElement>>()
		.iter(world)
		.collect::<Vec<_>>()
	{
		world.entity_mut(entity).remove::<HasElement>();
	}
}

pub fn trigger_transform_change(world: &mut World) {
	// dont use has element filter, none have it if we just turned renderer back on
	for mut transform in world.query::<&mut Transform>().iter_mut(world) {
		//trigger change
		let _changed = transform.as_mut();
	}
}

pub fn add_renderer(world: &mut World, container: HtmlDivElement) {
	world.insert_non_send_resource(DomRenderer::new(container));
	trigger_transform_change(world);
}
