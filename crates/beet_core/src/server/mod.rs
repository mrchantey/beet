pub mod address;
mod router_ext;
pub use self::address::*;
pub use router_ext::*;
pub mod tls;
pub use self::tls::*;
use anyhow::Result;
use axum::Router;
use axum::extract::Request;
use axum::http::Method;
use axum::response::Response;
use axum::routing::get;
use beet_utils::prelude::*;
use clap::Parser;
use std::path::PathBuf;
use std::time::Duration;
use tokio::task::JoinHandle;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::services::ServeFile;
use tower_livereload::LiveReloadLayer;

/// Serve static files with hot reloading
#[derive(Debug, Clone, Parser)]
#[command(name = "serve")]
pub struct Server {
	/// Directory to serve
	#[arg(default_value = ".")]
	pub dir: PathBuf,
	/// Specify port
	#[arg(long, default_value = "3000")]
	pub port: String,
	/// Specify host
	#[arg(long, default_value = "0.0.0.0")]
	pub host: String,
	/// Run with https, *only for development*
	#[arg(long)]
	pub secure: bool,
	// pub address: Address,
	// do not clear the dir
	#[arg(long)]
	pub no_clear: bool,
	#[arg(long)]
	pub quiet: bool,
	/// If a url is not found, fallback to the provided file
	#[arg(long)]
	pub fallback: Option<String>,
	/// Add 'access-control-allow-origin: *' header
	#[arg(long)]
	pub any_origin: bool,
	//
	// [FsWatcher] args, we dont include because of dir/cwd collision
	//
	#[command(flatten)]
	pub filter: GlobFilter,
	/// debounce time in milliseconds
	#[arg(short,long="debounce-millis",value_parser = parse_duration,default_value="50")]
	pub debounce: Duration,
}

impl Default for Server {
	fn default() -> Self { Self::parse_from(&[""]) }
}

impl Server {
	pub fn with_dir(mut self, dir: impl Into<PathBuf>) -> Self {
		self.dir = dir.into();
		self
	}
	pub fn quietly(mut self) -> Self {
		self.quiet = true;
		self
	}
	pub async fn run(self) -> Result<()> {
		self.serve_with_default_reload().await
	}

	pub async fn serve_with_default_reload(self) -> Result<()> {
		let (livereload, _handle) = self.default_reload();
		self.serve_with_reload(livereload).await
	}

	pub async fn serve_with_reload(
		self,
		livereload: LiveReloadLayer,
	) -> Result<()> {
		self.serve_with_options(Some(livereload)).await
	}

	pub async fn serve_with_options(
		self,
		livereload: Option<LiveReloadLayer>,
	) -> Result<()> {
		self.print_start();

		let mut router = Router::new().route_service("/__ping__", get(ping));

		if let Some(fallback) = &self.fallback {
			router = router.fallback_service(
				ServeDir::new(&self.dir)
					.append_index_html_on_directories(true)
					.fallback(ServeFile::new(fallback)),
			);
		} else {
			router = router.fallback_service(
				ServeDir::new(&self.dir).append_index_html_on_directories(true),
			);
		}
		if let Some(livereload) = livereload {
			router = router.layer(livereload);
		}
		if self.any_origin {
			let cors = CorsLayer::new()
				.allow_methods([Method::GET, Method::POST])
				.allow_origin(tower_http::cors::Any);
			router = router.layer(cors);
		}

		if self.secure {
			#[cfg(feature = "rustls")]
			{
				self.serve_secure(router).await
			}
			#[cfg(not(feature = "rustls"))]
			{
				anyhow::bail!(
					"secure mode is not enabled, please enable the 'rustls' feature"
				);
			}
		} else {
			self.serve_insecure(router).await
		}
	}

	fn default_reload(&self) -> (LiveReloadLayer, JoinHandle<Result<()>>) {
		let livereload = LiveReloadLayer::new();
		let reload = livereload.reloader();
		let this = self.clone();
		let reload_handle = tokio::spawn(async move {
			let this2 = this.clone();

			let mut rx = FsWatcher {
				cwd: this.dir.clone(),
				filter: this.filter.clone(),
				debounce: this.debounce.clone(),
			}
			.watch()?;
			while let Some(ev) = rx.recv().await? {
				if let Some(events) = ev.mutated_pretty() {
					reload.reload();
					println!("{}", events);
					this2.print_start();
				}
			}

			Ok(())
		});
		(livereload, reload_handle)
	}

	fn print_start(&self) {
		if self.quiet {
			return;
		}
		if self.no_clear == false {
			// doesnt seem error worthy here
			terminal::clear().ok();
		}
		let any_origin = if self.any_origin {
			"\nany-origin: true"
		} else {
			""
		};
		println!(
			"serving '{}' at {}{any_origin}",
			self.dir.display(),
			self.address().unwrap(),
		);
	}
	pub fn address(&self) -> Result<Address> {
		Ok(Address {
			host: Address::host_from_str(&self.host)?,
			port: self.port.parse::<u16>()?,
			secure: self.secure,
			..Default::default()
		})
	}
}

async fn ping(req: Request) -> Response<String> {
	let body = format!("request was {:?}", req);
	Response::new(body)
}
