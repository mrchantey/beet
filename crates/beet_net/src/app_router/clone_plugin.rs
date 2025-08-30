#![allow(unused)]
use crate::prelude::*;
use beet_rsx::prelude::*;
use bevy::app::Plugins;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use http::StatusCode;
use http::request::Parts;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;

/// For when you need a [`ClonePlugin`] to actually implement [`Clone`].
#[derive(Deref, DerefMut)]
pub struct ClonePluginContainer(pub Box<dyn ClonePlugin>);

impl ClonePluginContainer {
	pub fn new(plugin: impl ClonePlugin) -> Self { Self(Box::new(plugin)) }
}

impl Clone for ClonePluginContainer {
	fn clone(&self) -> Self { Self(self.0.box_clone()) }
}

/// A blanket trait for Clone plugins, so we can clone them
/// without requiring the plugin to be `Clone` or `Sized`.
pub trait ClonePlugin: 'static + Send + Sync + Plugin {
	fn add_to_app(&self, app: &mut App);
	fn is_plugin_added(&self, app: &App) -> bool;
	fn try_add_to_app(&self, app: &mut App) -> Result;
	fn box_clone(&self) -> Box<dyn ClonePlugin>;
}
impl<P> ClonePlugin for P
where
	P: 'static + Send + Sync + Clone + Plugin,
{
	fn add_to_app(&self, app: &mut App) { app.add_plugins(self.clone()); }
	fn is_plugin_added(&self, app: &App) -> bool {
		app.is_plugin_added::<Self>()
	}
	fn try_add_to_app(&self, app: &mut App) -> Result {
		if self.is_plugin_added(app) {
			bevybail!(
				"Plugin already added: {}",
				std::any::type_name::<Self>()
			);
		} else {
			app.add_plugins(self.clone());
			Ok(())
		}
	}
	fn box_clone(&self) -> Box<dyn ClonePlugin> { Box::new(self.clone()) }
}


/// A [`ScheduleConfigs<ScheduleSystem>`] that can be cloned, including closures
/// for use cases like `run_if` which is not trivially cloneable.
pub trait CloneScheduleSystem<M>: Clone {
	fn into_schedule_system(self) -> ScheduleConfigs<ScheduleSystem>;
}

pub struct CloneScheduleSystemMarker;

impl<T, M> CloneScheduleSystem<(CloneScheduleSystemMarker, M)> for T
where
	T: Clone + IntoScheduleConfigs<ScheduleSystem, M>,
{
	fn into_schedule_system(self) -> ScheduleConfigs<ScheduleSystem> {
		self.into_configs()
	}
}


pub struct ClosureCloneScheduleSystemMarker;

impl<F, T, M> CloneScheduleSystem<(ClosureCloneScheduleSystemMarker, T, M)>
	for F
where
	F: Clone + Fn() -> T,
	T: IntoScheduleConfigs<ScheduleSystem, M>,
{
	fn into_schedule_system(self) -> ScheduleConfigs<ScheduleSystem> {
		self().into_configs()
	}
}
