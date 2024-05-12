use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

/// Plugins used for most beet apps.
#[derive(Default)]
pub struct DefaultBeetPlugins;

impl PluginGroup for DefaultBeetPlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>()
			.add(LifecyclePlugin::default())
			.add(MovementPlugin::default())
			.add(SteerPlugin::default());


		#[cfg(feature = "animation")]
		(builder = builder.add(crate::prelude::AnimationPlugin::default()));

		builder

		// let foo = [
		// ComponentInfo { id: ComponentId(195), descriptor: ComponentDescriptor {
		// "bevy_hierarchy::components::parent::Parent", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (180463040437062246, 9148105329360383216) }), layout: Layout { size: 8, align: 8 (1 << 3) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(196), descriptor: ComponentDescriptor {
		// "bevy_hierarchy::components::children::Children", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (5049763796749338368, 558405129892214494) }), layout: Layout { size: 72, align: 8 (1 << 3) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(201), descriptor: ComponentDescriptor {
		// "bevy_core::name::Name", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (9487631089339943432, 565615200780569786) }), layout: Layout { size: 32, align: 8 (1 << 3) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(202), descriptor: ComponentDescriptor {
		// "bevy_transform::components::transform::Transform", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (18020809967241397133, 1334395248718908402) }), layout: Layout { size: 48, align: 16 (1 << 4) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(203), descriptor: ComponentDescriptor {
		// "bevy_transform::components::global_transform::GlobalTransform", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (13593940926195913987, 455481052785514367) }), layout: Layout { size: 64, align: 16 (1 << 4) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(246), descriptor: ComponentDescriptor {
		// "bevy_render::view::visibility::Visibility", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (1239496932856458774, 2566315166301987278) }), layout: Layout { size: 1, align: 1 (1 << 0) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(247), descriptor: ComponentDescriptor {
		// "bevy_render::view::visibility::InheritedVisibility", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (5718886323202239194, 18329413241462244206) }), layout: Layout { size: 1, align: 1 (1 << 0) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(248), descriptor: ComponentDescriptor {
		// "bevy_render::view::visibility::ViewVisibility", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (13377797711082269878, 9220272970077192985) }), layout: Layout { size: 1, align: 1 (1 << 0) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(272), descriptor: ComponentDescriptor {
		// "bevy_animation::AnimationPlayer", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (301796973186202917, 10409812091787247817) }), layout: Layout { size: 56, align: 8 (1 << 3) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }, ComponentInfo { id: ComponentId(317), descriptor: ComponentDescriptor {
		// "bevy_animation::AnimationTarget", storage_type: Table, is_send_and_sync: true, type_id: Some(TypeId { t: (13529991020310909077, 7330004398698050992) }), layout: Layout { size: 24, align: 8 (1 << 3) } }, hooks: ComponentHooks { on_add: None, on_insert: None, on_remove: None } }] ;
	}
}
