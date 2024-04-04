use crate::prelude::*;
use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use forky_web::AnimationFrame;
use forky_web::SearchParams;
use parking_lot::RwLock;
use std::sync::Arc;



#[derive(Clone, Deref, DerefMut)]
pub struct BeetWebApp(pub Arc<RwLock<App>>);

impl Default for BeetWebApp {
	fn default() -> Self { Self::new() }
}


impl BeetWebApp {
	pub fn new() -> Self {
		console_error_panic_hook::set_once();
		console_log::init_with_level(log::Level::Info).ok();
		let mut app = App::new();

		app.add_plugins(DomSim::<CoreModule>::default().with_url_params());

		Self(Arc::new(RwLock::new(app)))
	}

	pub fn run_forever(self) -> Result<Self> {
		self.clone().run()?.forget();
		Ok(self)
	}

	#[must_use]
	pub fn run(self) -> Result<AnimationFrame> {
		let frame = AnimationFrame::new(move || {
			self.try_write().map(|mut a| a.update());
		});

		Ok(frame)
	}

	pub fn with_test_container(self) -> Self {
		init_test_app(self.0.clone());
		self
	}

	pub fn with(self, func: impl FnOnce(&mut World)) -> Self {
		let mut app = self.write();
		func(app.world_mut());
		drop(app);
		self
	}

	pub fn with_node<M>(self, node: impl IntoBeetBuilder<M>) -> Result<Self> {
		let mut app = self.write();
		let scene = node
			.into_beet_builder()
			.as_prefab()
			.into_scene::<CoreModule>();

		scene.write(&mut app.world_mut())?;
		drop(app);
		Ok(self)
	}

	pub async fn try_load_scene_url(self) -> Result<()> {
		let Some(url) = SearchParams::get("scene") else {
			return Ok(());
		};
		let scene = fetch_scene::<CoreModule>(&url).await?;

		let mut app = self.write();
		let mut world = app.world_mut();
		scene.write(&mut world)?;
		Ok(())
	}
}
