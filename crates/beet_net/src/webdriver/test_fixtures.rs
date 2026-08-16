//! Shared utilities for the webdriver smoketests: local html fixtures so no
//! test touches the live network, and per-test driver ports so concurrently
//! running tests never fight over one chromedriver.

use super::*;
use beet_core::prelude::*;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

/// Write `{name}.html` under `target/webdriver-fixtures/` and return its
/// `file://` url.
pub(crate) fn page_url(name: &str, html: &str) -> String {
	let path = fs_ext::workspace_root()
		.join("target/webdriver-fixtures")
		.join(format!("{name}.html"));
	fs_ext::write(&path, html).unwrap();
	format!("file://{}", path.display())
}

/// A [`Client`] with a unique driver/websocket port pair, so concurrently
/// running tests each own their driver process.
pub(crate) fn client() -> Client {
	static NEXT_PORT: AtomicU16 =
		AtomicU16::new(crate::prelude::DEFAULT_WEBDRIVER_PORT + 10);
	let driver_port = NEXT_PORT.fetch_add(2, Ordering::SeqCst);
	Client::default()
		.with_driver_port(driver_port)
		.with_websocket_port(driver_port + 1)
}

/// Spawn a uniquely-ported chromedriver and visit `url`.
pub(crate) async fn visit(url: &str) -> (ClientProcess, Page) {
	let process = ClientProcess::new_with_opts(client()).unwrap();
	let page = Page::visit_with_client(&process.client(), url).await.unwrap();
	(process, page)
}

