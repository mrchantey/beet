use anyhow::Result;
#[allow(unused)]
use beet_rsx::prelude::*;
use rapidhash::RapidHashMap;


pub struct ClientIslandMountFuncs {
	pub map:
		RapidHashMap<&'static str, Box<dyn Send + Sync + Fn() -> Result<()>>>,
}

impl ClientIslandMountFuncs {
	pub fn new(
		route_funcs: Vec<(
			&'static str,
			Box<dyn Send + Sync + Fn() -> Result<()>>,
		)>,
	) -> Self {
		Self {
			map: route_funcs.into_iter().collect(),
		}
	}
	pub fn with(
		mut self,
		route: &'static str,
		func: impl Send + Sync + Fn() -> Result<()> + 'static,
	) -> Self {
		self.map.insert(route, Box::new(func));
		self
	}

	#[allow(unused)]
	pub fn mount_with_server_url(&self, url: &str) -> Result<()> {
		#[cfg(not(debug_assertions))]
		{
			use beet::prelude::*;
			CallServerAction::set_server_url(RoutePath::new(url));
		}
		#[cfg(target_arch = "wasm32")]
		self.mount()?;
		Ok(())
	}


	#[cfg(target_arch = "wasm32")]
	pub fn mount(&self) -> Result<()> {
		console_error_panic_hook::set_once();

		DomTarget::set(BrowserDomTarget::default());

		let mut path =
			web_sys::window().unwrap().location().pathname().unwrap();
		if path.len() > 1 && path.ends_with('/') {
			path.pop();
		}

		if let Some(mount_fn) = self.map.get(path.as_str()) {
			mount_fn()?;
		} else {
			let received_paths = self.map.keys().collect::<Vec<_>>();
			anyhow::bail!(
				"No mount function found for path: {}\nreceived paths: {:?}",
				path,
				received_paths
			);
		}

		EventRegistry::initialize()?;
		Ok(())
	}
}
