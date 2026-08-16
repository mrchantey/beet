//! In-page DOM element matchers for wasm browser tests, the cypress-style
//! twin of the webdriver matchers in `beet_net`: same `xpect_*` vocabulary,
//! same auto-retry semantics, asserting on a [`web_sys::HtmlElement`] from
//! inside the page instead of driving the browser from outside.
//!
//! Polling yields to the js event loop between attempts, so these require the
//! async wasm runner (a browser-hosted test suite); under a runner that never
//! yields they would only ever attempt once.

use crate::prelude::*;
use web_sys::HtmlElement;

/// Extension trait adding auto-retrying assertions to DOM elements.
#[extend::ext(name=MatcherHtmlElement)]
pub impl<T> T
where
	T: AsRef<HtmlElement>,
{
	/// Assert the trimmed `textContent` equals `expected`, polling until it
	/// does or the default poll timeout expires.
	async fn xpect_text(&self, expected: &str) -> &Self {
		let text = || {
			self.as_ref()
				.text_content()
				.unwrap_or_default()
				.trim()
				.to_string()
		};
		poll_ext::poll(|| {
			(text() == expected)
				.then_some(())
				.ok_or_else(|| bevyhow!("mismatch"))
		})
		.await
		.unwrap_or_else(|_| {
			panic_ext::panic_expected_received_display(expected, text())
		});
		self
	}

	/// Assert the trimmed `textContent` contains `expected`, polling.
	async fn xpect_contains_text(&self, expected: &str) -> &Self {
		let text = || {
			self.as_ref()
				.text_content()
				.unwrap_or_default()
				.trim()
				.to_string()
		};
		poll_ext::poll(|| {
			text()
				.contains(expected)
				.then_some(())
				.ok_or_else(|| bevyhow!("mismatch"))
		})
		.await
		.unwrap_or_else(|_| {
			panic_ext::panic_expected_received_display(
				format!("text containing {expected:?}"),
				text(),
			)
		});
		self
	}

	/// Assert the `innerHTML` equals `expected`, polling.
	async fn xpect_html(&self, expected: &str) -> &Self {
		let html = || self.as_ref().inner_html();
		poll_ext::poll(|| {
			(html() == expected)
				.then_some(())
				.ok_or_else(|| bevyhow!("mismatch"))
		})
		.await
		.unwrap_or_else(|_| {
			panic_ext::panic_expected_received_display(expected, html())
		});
		self
	}

	/// Assert the attribute `name` equals `expected`, polling.
	async fn xpect_attr(&self, name: &str, expected: &str) -> &Self {
		let attr = || {
			self.as_ref()
				.get_attribute(name)
				.unwrap_or_else(|| "None".to_string())
		};
		poll_ext::poll(|| {
			(attr() == expected)
				.then_some(())
				.ok_or_else(|| bevyhow!("mismatch"))
		})
		.await
		.unwrap_or_else(|_| {
			panic_ext::panic_expected_received_display(expected, attr())
		});
		self
	}
}
