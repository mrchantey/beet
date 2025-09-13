use bevy::prelude::*;

use crate::prelude::*;


const DEFAULT_PORT: u16 = 8338;
// const DEFAULT_DEVTOOLS_PORT: u16 = 9222;
// const DEFAULT_WEBDRIVER_PORT: u16 = 4444;


#[cfg(feature = "webdriver")]
pub type DefaultClient = WebdriverClient;
#[cfg(all(feature = "chrome_dev_tools", not(feature = "webdriver")))]
pub type DefaultClient = DevToolsClient;
#[cfg(not(any(feature = "webdriver", feature = "chrome_dev_tools")))]
pub type DefaultClient = DummyClient;

pub struct DummyClient;
impl DummyClient {
	pub async fn connect() -> Result<Self> { unimplemented!() }
	pub fn page(&self) -> Page {
		unimplemented!();
	}
}



pub struct ConnectOptions {
	/// Whether to launch the browser in headless mode
	pub headless: bool,
	/// The port for the browser process
	pub port: u16,
	/// Spin up the required process
	/// - For chrome devtools this is the actual `chromium` process.
	/// - For webdriver this is the `chromedriver` process,
	/// in webdriver the actual `chromium` process always spawns
	pub spawn_process: bool,
}
impl Default for ConnectOptions {
	fn default() -> Self {
		Self {
			headless: true,
			spawn_process: true,
			port: DEFAULT_PORT,
		}
	}
}

impl ConnectOptions {
	pub fn url(&self) -> String { format!("http://127.0.0.1:{}", self.port) }
}
