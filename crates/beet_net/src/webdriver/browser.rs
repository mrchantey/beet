//! One running browser as one value.
//!
//! [`Browser`] owns the driver process, its session and the primary page,
//! [`Deref`]ing to [`Page`] so the whole webdriver surface is reachable
//! without juggling `(ClientProcess, Page)` tuples. The decomposed layer
//! ([`ClientProcess`], [`Session`], [`Page`]) remains for callers composing
//! differently, eg one session navigated across many urls.

use super::Page;
use super::*;
use beet_core::prelude::*;

/// A running browser: the driver process, its session and the primary page.
pub struct Browser {
	process: ClientProcess,
	page: Page,
}

impl Browser {
	/// Spawn a default (chromium, headless) driver and open a session without
	/// navigating (`about:blank`), eg to attach collectors before the first
	/// navigation.
	pub async fn new() -> Result<Self> {
		Self::new_with(Client::default()).await
	}

	/// [`Self::new`] with a custom `client` (ports, provider, log level).
	pub async fn new_with(client: Client) -> Result<Self> {
		let process = ClientProcess::new_with_opts(client)?;
		let session = process.client().new_session().await?;
		let page = Page::from_session(session).await?;
		Ok(Self { process, page })
	}

	/// Spawn a default (chromium, headless) driver and visit `url`.
	pub async fn visit(url: &str) -> Result<Self> {
		Self::visit_with(Client::default(), url).await
	}

	/// Spawn a driver for `client` (custom ports, provider, log level) and
	/// visit `url`.
	pub async fn visit_with(client: Client, url: &str) -> Result<Self> {
		let mut browser = Self::new_with(client).await?;
		browser.navigate(url).await?;
		Ok(browser)
	}

	/// The primary page (also reachable through deref).
	pub fn page(&self) -> &Page { &self.page }

	/// Gracefully delete the session, then kill the driver process.
	pub async fn kill(self) -> Result<()> {
		self.page.kill().await?;
		self.process.kill()
	}
}

impl core::ops::Deref for Browser {
	type Target = Page;
	fn deref(&self) -> &Self::Target { &self.page }
}

impl core::ops::DerefMut for Browser {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.page }
}
