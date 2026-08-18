//! Shared utilities for the webdriver smoketests: local html fixtures so no
//! test touches the live network, and per-test driver ports (via
//! [`Client::unique`]) so concurrently running tests never fight over one
//! chromedriver.

use super::*;
use beet_core::prelude::*;

/// Write `{name}.html` under `target/webdriver-fixtures/` and return its
/// `file://` url.
pub(crate) fn page_url(name: &str, html: &str) -> String {
	let path = fs_ext::workspace_root()
		.join("target/webdriver-fixtures")
		.join(format!("{name}.html"));
	fs_ext::write(&path, html).unwrap();
	format!("file://{}", path.display())
}

/// A uniquely-ported [`Client`], so concurrently running tests each own their
/// driver process.
pub(crate) fn client() -> Client { Client::unique() }

/// Spawn a uniquely-ported chromedriver and visit `url`.
pub(crate) async fn visit(url: &str) -> Browser {
	Browser::visit_with(client(), url).await.unwrap()
}
