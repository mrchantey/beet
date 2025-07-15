#![allow(unused)]
use crate::prelude::*;
use axum::extract::FromRequest;
use axum::extract::FromRequestParts;
use axum::extract::Request;
use axum::handler::Handler;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::MethodRouter;
use beet_rsx::prelude::*;
use bevy::app::Plugins;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use http::StatusCode;
use http::request::Parts;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;



/// The base state for the [`AppRouter`], this can be extended with the standard
/// axum state pattern of implementing [`AsRef<AppRouterState>`] for the derived state,
/// with one additional requirement that the state must implement [`AsMut<AppRouterState>`]
/// as well, so that derived state can share the builder pattern.

/// ## Example
/// ```rust
/// # use beet_server::prelude::*;
///
/// #[derive(Default, Clone)]
/// struct MyState {
///   my_state: usize,
///   base_state: AppRouterState,
/// }
/// impl AsRef<AppRouterState> for MyState {
/// 	fn as_ref(&self) -> &AppRouterState {
/// 		&self.base_state
/// 	}
/// }
/// impl AsMut<AppRouterState> for MyState {
/// 	fn as_mut(&mut self) -> &mut AppRouterState {
/// 		&mut self.base_state
/// 	}
/// }
///
/// let router = AppRouter::new(MyState::default());
///
/// ```
pub struct AppRouterState {
	pub template_config: Option<TemplateConfig>,
	plugin: Box<dyn ClonePlugin>,
}

impl AppRouterState {
	pub fn test() -> Self {
		Self {
			template_config: Some(TemplateConfig::default()),
			plugin: Box::new(|app: &mut App| {
				app.insert_resource(TemplateFlags::None);
			}),
		}
	}
}

impl AsRef<AppRouterState> for AppRouterState {
	fn as_ref(&self) -> &AppRouterState { self }
}
impl AsMut<AppRouterState> for AppRouterState {
	fn as_mut(&mut self) -> &mut AppRouterState { self }
}

impl Default for AppRouterState {
	fn default() -> Self {
		Self {
			template_config: None,
			plugin: Box::new(|_: &mut App| {}),
		}
	}
}

impl Clone for AppRouterState {
	fn clone(&self) -> Self {
		Self {
			template_config: self.template_config.clone(),
			plugin: self.plugin.box_clone(),
		}
	}
}

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

/// Helper trait for less verbose state definitions.
pub trait DerivedAppState:
	'static + Send + Sync + Clone + AsRef<AppRouterState> + AsMut<AppRouterState>
{
	fn set_plugins<M>(
		&mut self,
		plugins: impl 'static + Clone + Send + Sync + Plugins<M>,
	) -> &mut Self {
		let this = self.as_mut();
		this.plugin = Box::new(move |app: &mut App| {
			plugins.clone().add_to_app(app);
		});
		self
	}

	fn create_app(&self) -> App {
		let this = self.as_ref();
		let mut app = App::new();
		app.add_plugins((
			this.template_config.clone().unwrap_or_default(),
			AppRouterPlugin,
			TemplatePlugin,
		));
		#[cfg(all(not(test), feature = "build"))]
		app.add_plugins(beet_build::prelude::BuildPlugin::default());
		// add plugin last for opportunity to override
		this.plugin.add_to_app(&mut app);
		app
	}

	fn render_bundle(&self, bundle: impl Bundle) -> Html<String> {
		let mut app = self.create_app();
		let entity = app.world_mut().spawn((HtmlDocument, bundle)).id();
		app.update();
		let html = app
			.world_mut()
			.run_system_cached_with(render_fragment, entity)
			.unwrap();
		app.world_mut().despawn(entity);
		Html(html)
	}
}
impl<
	S: 'static
		+ Send
		+ Sync
		+ Clone
		+ AsRef<AppRouterState>
		+ AsMut<AppRouterState>,
> DerivedAppState for S
{
}
