use crate::prelude::*;
use anyhow::Result;
use std::pin::Pin;


type OnRun =
	Box<dyn FnOnce(&BeetAppArgs) -> Pin<Box<dyn Future<Output = Result<()>>>>>;
// type OnRun = Box<dyn FnOnce() -> Result<()> + Send + Sync>;

/// Entrypoint for all beet apps:
/// - `static`: building static html files
/// - `server`: serving routes, including via lambda
/// - `wasm`: hydrating a client-side app
pub struct BeetApp {
	/// The root context for this app
	pub context: RootContext,
	/// The router which can be extended by adding routers
	/// as plugins.
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	pub axum_router: beet_server::axum::Router,
	/// A set of functions to execute when running in static
	/// mode.
	pub on_run_static: Vec<OnRun>,
	#[cfg(target_arch = "wasm32")]
	pub on_run_wasm:
		Vec<Box<dyn FnOnce(&BeetAppArgs) -> Result<()> + Send + Sync>>,
}

impl BeetApp {
	pub fn new(context: RootContext) -> Self {
		Self {
			context,
			#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
			axum_router: Default::default(),
			on_run_static: Default::default(),
			#[cfg(target_arch = "wasm32")]
			on_run_wasm: Default::default(),
		}
	}

	pub fn add_collection<M>(mut self, col: impl IntoCollection<M>) -> Self {
		col.into_collection().register(&mut self);
		self
	}

	#[cfg(target_arch = "wasm32")]
	pub fn run(self) { self.run_inner().unwrap(); }

	#[cfg(target_arch = "wasm32")]
	fn run_inner(self) -> Result<()> {
		let args = BeetAppArgs::from_url_params()?;
		self.on_run_wasm.into_iter().try_for_each(|f| f(&args))
	}


	#[cfg(not(target_arch = "wasm32"))]
	pub fn run(self) {
		let result = tokio::runtime::Builder::new_multi_thread()
			.enable_all()
			.build()
			.unwrap()
			.block_on(self.run_inner());
		if let Err(e) = result {
			eprintln!("Error: {}", e);
			std::process::exit(1);
		}
	}

	#[cfg(not(target_arch = "wasm32"))]
	async fn run_inner(self) -> Result<()> {
		use clap::Parser;

		let args = BeetAppArgs::parse().validate()?;

		#[cfg(feature = "serde")]
		if args.root_context {
			let cx =
				ron::ser::to_string_pretty(&self.context, Default::default())?;
			println!("{}", cx);
			return Ok(());
		}


		if args.is_static {
			futures::future::try_join_all(
				self.on_run_static.into_iter().map(|f| f(&args)),
			)
			.await?;
			Ok(())
		} else {
			#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
			beet_server::prelude::BeetServer {
				html_dir: args.html_dir.into(),
				router: self.axum_router,
				..Default::default()
			}
			.serve()
			.await?;
			Ok(())
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn works() { let _app = BeetApp::new(root_cx!()); }
}
