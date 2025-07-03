#![allow(unused)]
use axum::extract::FromRequest;
use axum::extract::FromRequestParts;
use axum::extract::Request;
use axum::handler::Handler;
use axum::response::IntoResponse;
use axum::routing::MethodRouter;
use beet_template::prelude::*;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use http::StatusCode;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
// use bevy::platform::collections::HashMap;
use crate::prelude::*;
use bevy::prelude::*;
use http::request::Parts;



/// The base state for the [`AppRouter`], this can be extended with the standard
/// axum state pattern of implementing [`AsRef<AppRouterState>`] for the derived state.

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
///
/// let router = AppRouter::new(MyState::default())
///
/// ```
pub struct AppRouterState {
	/// The app used to instantiate and render bundles.
	/// This should include the [`TemplatePlugin`].
	create_app: Box<dyn CreateApp>,
}

impl AppRouterState {
	pub fn with_create_app<F>(mut self, create_app: impl CreateApp) -> Self {
		self.create_app = Box::new(create_app);
		self
	}
}

impl AsRef<AppRouterState> for AppRouterState {
	fn as_ref(&self) -> &AppRouterState { self }
}

impl Default for AppRouterState {
	fn default() -> Self {
		Self {
			create_app: Box::new(|| App::new()),
		}
	}
}

impl Clone for AppRouterState {
	fn clone(&self) -> Self {
		Self {
			create_app: self.create_app.box_clone(),
		}
	}
}
/// The method used to create the app for each request.
pub trait CreateApp: 'static + Send + Sync {
	fn create_app(&self) -> App;
	fn box_clone(&self) -> Box<dyn CreateApp>;
}
impl<F> CreateApp for F
where
	F: 'static + Send + Sync + Clone + Fn() -> App,
{
	fn create_app(&self) -> App { (self.clone())() }
	fn box_clone(&self) -> Box<dyn CreateApp> { Box::new(self.clone()) }
}

/// Helper trait for less verbose state definitions.
pub trait DerivedAppState:
	'static + Send + Sync + Clone + AsRef<AppRouterState>
{
}
impl<S: 'static + Send + Sync + Clone + AsRef<AppRouterState>> DerivedAppState
	for S
{
}
