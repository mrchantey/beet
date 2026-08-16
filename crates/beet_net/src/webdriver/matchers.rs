//! Async auto-retrying matchers on [`Page`] and [`Element`].
//!
//! The same `xpect_*` vocabulary and panic semantics as the sync matchers in
//! `beet_core::testing`, but async and auto-waiting (and, under the `nightly`
//! feature, `#[track_caller]` so panics print the callsite): each assertion polls
//! until it passes or the deadline expires ([`Page::timeout`] on pages, the
//! default poll timeout on elements), so tests read like their cypress and
//! playwright counterparts with no explicit waits:
//!
//! ```no_run
//! # use beet_core::prelude::*;
//! # use beet_net::prelude::*;
//! # async fn demo(page: webdriver::Page) {
//! page.find("#count").await.xpect_text("count is 0").await;
//! page.find("#inc").await.click().await.unwrap();
//! page.find("#count").await.xpect_text("count is 2").await;
//! page.xpect_no_selector("#modal").await;
//! # }
//! ```
//!
//! Negation comes as dedicated `xpect_not_*`/`xpect_no_*` methods (the
//! blanket sync [`xnot`](beet_core::prelude::MatcherNot::xnot) carrier
//! resolves first on any type, so the async matchers mirror the
//! [`xpect_not_eq`](beet_core::prelude::MatcherEq::xpect_not_eq) shape
//! instead). A negated matcher inverts the predicate and therefore *waits
//! for* absence or mismatch, the `should('not.exist')` shape. Available in
//! test builds and behind the `testing` feature.

use super::Element;
use super::Page;
use super::locate;
use beet_core::prelude::*;
use core::fmt::Display;
use core::time::Duration;

/// Poll `received` until `predicate` holds (or with `negated`, until it does
/// not), panicking pretty on timeout with the last received value. Fetch
/// errors (a stale element, a mid-navigation page) also retry.
async fn assert_eventually<T: Display>(
	timeout: Duration,
	mut received: impl AsyncFnMut() -> Result<T>,
	predicate: impl Fn(&T) -> bool,
	expected: impl Display,
	negated: bool,
) {
	let mut last = None;
	let result = poll_ext::poll_async_with(
		async || {
			let value = received().await?;
			let matched = predicate(&value);
			last = Some(value);
			(matched != negated)
				.then_some(())
				.ok_or_else(|| bevyhow!("assertion does not hold"))
		},
		timeout,
		poll_ext::DEFAULT_INTERVAL,
	)
	.await;
	let Err(err) = result else { return };
	let expected = match negated {
		true => format!("NOT {expected}"),
		false => expected.to_string(),
	};
	match last {
		Some(received) => {
			panic_ext::panic_expected_received_display(expected, received)
		}
		None => panic_ext::panic_str(format!(
			"Expected: {expected}\nReceived: no value, {err}"
		)),
	}
}

/// Render an optional string, eg an absent attribute, for panic output.
fn display_option(value: &Option<String>) -> String {
	match value {
		Some(value) => value.clone(),
		None => "None".to_string(),
	}
}

impl Page {
	/// Assert the page url equals `expected`, auto-waiting, so a click that
	/// navigates needs no explicit wait before this.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_url(&self, expected: &str) -> &Self {
		assert_eventually(
			self.timeout(),
			async || self.current_url().await,
			|url| url == expected,
			expected,
			false,
		)
		.await;
		self
	}

	/// Assert the document title equals `expected`, auto-waiting.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_title(&self, expected: &str) -> &Self {
		assert_eventually(
			self.timeout(),
			async || self.title().await,
			|title| title == expected,
			expected,
			false,
		)
		.await;
		self
	}
}

impl Page {
	/// Assert no element matches `selector`, waiting for removal, the
	/// `should('not.exist')` shape.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_no_selector(&self, selector: &str) -> &Self {
		assert_eventually(
			self.timeout(),
			async || self.try_find_all(selector).await.map(|all| all.len()),
			|count| *count > 0,
			format!("elements matching {selector:?}"),
			true,
		)
		.await;
		self
	}

	/// Assert the page url does not equal `expected`, waiting to leave it.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_not_url(&self, expected: &str) -> &Self {
		assert_eventually(
			self.timeout(),
			async || self.current_url().await,
			|url| url == expected,
			expected,
			true,
		)
		.await;
		self
	}
}

impl Element {
	/// The trimmed `textContent`, the value [`Self::xpect_text`] asserts on.
	async fn text_trimmed(&self) -> Result<String> {
		self.text_content()
			.await?
			.unwrap_or_default()
			.trim()
			.to_string()
			.xok()
	}

	/// Assert the trimmed `textContent` equals `expected`, auto-waiting.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_text(&self, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || self.text_trimmed().await,
			|text| text == expected,
			expected,
			false,
		)
		.await;
		self
	}

	/// Assert the trimmed `textContent` contains `expected`, auto-waiting.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_contains_text(&self, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || self.text_trimmed().await,
			|text| text.contains(expected),
			format!("text containing {expected:?}"),
			false,
		)
		.await;
		self
	}

	/// Assert the `innerHTML` equals `expected`, auto-waiting.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_html(&self, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || self.inner_html().await,
			|html| html == expected,
			expected,
			false,
		)
		.await;
		self
	}

	/// Assert the attribute `name` equals `expected`, auto-waiting.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_attr(&self, name: &str, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || self.get_attribute(name).await.map(|a| display_option(&a)),
			|attr| attr == expected,
			expected,
			false,
		)
		.await;
		self
	}

	/// Assert the `value` property equals `expected`, auto-waiting, eg an
	/// `<input>`'s current text.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_value(&self, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || {
				self.get_property("value")
					.await?
					.pointer("/result/result/value")
					.and_then(|value| value.as_str())
					.unwrap_or_default()
					.to_string()
					.xok()
			},
			|value| value == expected,
			expected,
			false,
		)
		.await;
		self
	}

}

impl Element {
	/// Assert the trimmed `textContent` does not equal `expected`, waiting
	/// for it to change.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_not_text(&self, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || self.text_trimmed().await,
			|text| text == expected,
			expected,
			true,
		)
		.await;
		self
	}

	/// Assert the trimmed `textContent` does not contain `expected`.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_not_contains_text(&self, expected: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || self.text_trimmed().await,
			|text| text.contains(expected),
			format!("text containing {expected:?}"),
			true,
		)
		.await;
		self
	}

	/// Assert no descendant matches `selector`, waiting for removal.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn xpect_no_selector(&self, selector: &str) -> &Self {
		assert_eventually(
			poll_ext::DEFAULT_TIMEOUT,
			async || {
				locate::locate_nodes(
					self.session(),
					self.context_id(),
					&locate::Locator::Css(selector),
					Some(self),
					None,
				)
				.await
				.map(|all| all.len())
			},
			|count| *count > 0,
			format!("descendants matching {selector:?}"),
			true,
		)
		.await;
		self
	}
}
