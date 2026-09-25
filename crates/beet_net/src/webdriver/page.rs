//! WebDriver page abstraction for browser interaction.
//!
//! This module provides the [`Page`] type which wraps a browser session
//! and provides high-level methods for navigation and interaction.

use super::WebElement;
use super::*;
use beet_core::prelude::*;
use bevy::prelude::Result;
use core::time::Duration;
use serde_json::Value;
use serde_json::json;

/// High level ergonomic wrapper over a BiDi `Session` bound to a single
/// top-level browsing context (page / tab).
///
/// Construction Patterns:
/// * `Page::from_session(session)` – bind an already-created session
/// * `Page::visit(url)` – spawn default driver (chromium), create session,
///    navigate, return `(ClientProcess, Page)` so the caller can clean up
/// * `Page::visit_with_client(client, url)` – same but with a custom client
///
/// WebElement querying comes in two speeds: [`Page::find`] (and friends, see
/// `locate.rs`) auto-waits for a match bounded by [`Page::timeout`], while
/// [`Page::query_selector`] probes exactly once.
#[derive(Debug, Clone)]
pub struct Page {
	pub(super) session: Session,
	pub(super) context_id: String,
	/// The auto-wait deadline for [`Page::find`] and the async matchers,
	/// default 4s (sits under the 5s default test timeout).
	timeout: Duration,
}

impl Page {
	/// Bind to the first discovered top-level browsing context of an
	/// existing `Session`.
	pub async fn from_session(session: Session) -> Result<Self> {
		let tree = session
			.command("browsingContext.getTree", json!({"maxDepth": 0}))
			.await?;
		let contexts = tree["result"]["contexts"]
			.as_array()
			.ok_or_else(|| bevyhow!("contexts array missing"))?;
		let first = contexts
			.get(0)
			.and_then(|c| c.get("context"))
			.and_then(|c| c.as_str())
			.ok_or_else(|| bevyhow!("no top-level context discovered"))?;
		Ok(Self {
			session,
			context_id: first.to_string(),
			timeout: poll_ext::DEFAULT_TIMEOUT,
		})
	}

	/// The auto-wait deadline for [`Page::find`] and the async matchers.
	pub fn timeout(&self) -> Duration { self.timeout }

	/// Set the auto-wait deadline for [`Page::find`] and the async matchers.
	pub fn with_timeout(mut self, timeout: Duration) -> Self {
		self.timeout = timeout;
		self
	}

	/// Spawn a default (chromium) driver process, create a session,
	/// navigate to `url` and return both process + page.
	pub async fn visit(url: &str) -> Result<(ClientProcess, Self)> {
		let process = ClientProcess::new()?;
		let page = Self::visit_with_client(&process.client(), url).await?;
		Ok((process, page))
	}

	/// Same as `visit` but reuse a caller-provided client configuration.
	/// The client process must already be running.
	pub async fn visit_with_client(client: &Client, url: &str) -> Result<Self> {
		let session = client.new_session().await?;
		let mut page = Self::from_session(session).await?;
		page.navigate(url).await?;
		Ok(page)
	}

	/// Low level access to underlying session (for advanced use or
	/// interim gaps in high-level wrappers).
	pub fn session(&self) -> &Session { &self.session }

	/// Navigate the bound context to `url`, waiting for `complete`.
	pub async fn navigate(&mut self, url: &str) -> Result<()> {
		self.session
			.command(
				"browsingContext.navigate",
				json!({
					"context": self.context_id,
					"url": url,
					"wait": "complete"
				}),
			)
			.await
			.map_err(|err| {
				bevyhow!("navigate to {url} failed, is it a valid url?\n {err}")
			})?;
		Ok(())
	}

	/// Set the viewport (CSS px) of the bound context, so screen-media layout and
	/// any `innerWidth`-driven script render at the given resolution.
	pub async fn set_viewport(&self, width: u32, height: u32) -> Result<()> {
		self.session
			.command(
				"browsingContext.setViewport",
				json!({
					"context": self.context_id,
					"viewport": { "width": width, "height": height }
				}),
			)
			.await?;
		Ok(())
	}

	/// Evaluate a JavaScript expression in the page's browsing context.
	/// Returns the full BiDi response JSON (caller can drill down); a script
	/// exception becomes an `Err` carrying the exception text.
	pub async fn evaluate(&self, expression: &str) -> Result<Value> {
		let resp = self
			.session
			.command(
				"script.evaluate",
				json!({
					"expression": expression,
					"target": { "context": self.context_id },
					"awaitPromise": true,
					"resultOwnership": "root"
				}),
			)
			.await?;
		Self::check_exception(&resp)?;
		Ok(resp)
	}

	/// Evaluate a JavaScript expression and return its completion as plain
	/// JSON, with objects and arrays deep-serialized and flattened (see
	/// `bidi_value.rs`), so results compare directly against `json!` values.
	pub async fn evaluate_value(&self, expression: &str) -> Result<Value> {
		let resp = self
			.session
			.command(
				"script.evaluate",
				json!({
					"expression": expression,
					"target": { "context": self.context_id },
					"awaitPromise": true,
					"resultOwnership": "none",
					"serializationOptions": { "maxObjectDepth": 64 }
				}),
			)
			.await?;
		Self::check_exception(&resp)?;
		resp.pointer("/result/result")
			.ok_or_else(|| bevyhow!("script result missing"))?
			.xmap(bidi_value::to_json)
			.xok()
	}

	/// Surface an in-page exception (`result.type == "exception"`) as an
	/// error; protocol-level errors are already handled by
	/// [`Session::command`].
	fn check_exception(resp: &Value) -> Result<()> {
		if resp.pointer("/result/type").and_then(|ty| ty.as_str())
			== Some("exception")
		{
			let text = resp
				.pointer("/result/exceptionDetails/text")
				.and_then(|text| text.as_str())
				.unwrap_or("unknown script exception");
			bevybail!("script threw: {text}");
		}
		Ok(())
	}

	/// The document title.
	pub async fn title(&self) -> Result<String> {
		self.evaluate_value("document.title")
			.await?
			.as_str()
			.map(|title| title.to_string())
			.ok_or_else(|| bevyhow!("missing document title"))
	}

	/// The page's url, auto-waiting through a navigation: a context
	/// mid-navigation has no document to evaluate in and the driver answers
	/// that with a transient error ([`BiDiError::is_transient`]), so a url
	/// read right after a click that navigates retries rather than fails,
	/// bounded by [`Page::timeout`]. Which of the two urls it answers with is
	/// the caller's race; [`Self::wait_for_url`] is the one that waits for a
	/// particular one.
	///
	/// Open-coded rather than through `poll_ext` for the same reasons as
	/// `find_polling`: the expired deadline answers with the last read error
	/// rather than a generic timeout.
	pub async fn current_url(&self) -> Result<String> {
		let start = Instant::now();
		loop {
			let expired = start.elapsed() >= self.timeout;
			match self.read_url().await {
				Ok(url) => return url.xok(),
				Err(err) if expired => return Err(err),
				Err(err)
					if !err
						.downcast_ref::<BiDiError>()
						.is_some_and(BiDiError::is_transient) =>
				{
					return Err(err);
				}
				Err(_) => time_ext::sleep(poll_ext::DEFAULT_INTERVAL).await,
			}
		}
	}

	/// Poll the url until `predicate` holds, bounded by `timeout`, answering
	/// the url that satisfied it: the wait after a click that navigates, for
	/// a destination named by a property rather than a full string (left the
	/// login page, reached the dashboard). A transient read is a miss, as in
	/// [`Self::current_url`]; the error names the last url seen, so a wait
	/// that expires says where it got stuck.
	pub async fn wait_for_url(
		&self,
		timeout: Duration,
		predicate: impl Send + Fn(&str) -> bool,
	) -> Result<String> {
		let start = Instant::now();
		let mut last = None;
		loop {
			let expired = start.elapsed() >= timeout;
			match self.read_url().await {
				Ok(url) if predicate(&url) => return url.xok(),
				Ok(url) => last = Some(url),
				Err(err)
					if !err
						.downcast_ref::<BiDiError>()
						.is_some_and(BiDiError::is_transient) =>
				{
					return Err(err);
				}
				Err(_) => {}
			}
			if expired {
				bevybail!(
					"url did not satisfy the predicate within {timeout:?}: {}",
					match &last {
						Some(url) => format!("still at {url}"),
						None => "no url was read".to_string(),
					}
				);
			}
			time_ext::sleep(poll_ext::DEFAULT_INTERVAL).await;
		}
	}

	/// One `location.href` read, the probe both url methods poll.
	async fn read_url(&self) -> Result<String> {
		self.evaluate("location.href")
			.await?
			.pointer("/result/result/value")
			.and_then(|value| value.as_str())
			.ok_or_else(|| bevyhow!("missing location.href value"))?
			.to_string()
			.xok()
	}

	/// Query a single element. Returns `Ok(None)` if no match.
	/// When an element is found we extract its BiDi remote handle
	/// (handle/sharedId) so subsequent operations can target it
	/// without re‑querying.
	pub async fn query_selector(
		&self,
		selector: &str,
	) -> Result<Option<WebElement>> {
		let expr = format!("document.querySelector({selector:?})");
		let resp = self.evaluate(&expr).await?;
		let ty = resp
			.pointer("/result/result/type")
			.and_then(|v| v.as_str())
			.unwrap_or("undefined");
		if ty == "null" || ty == "undefined" {
			return Ok(None);
		}
		if let Some(el) = WebElement::from_bidi_response(
			&self.session,
			&self.context_id,
			&resp,
		) {
			Ok(Some(el))
		} else {
			bevybail!(
				"query_selector: element present but missing BiDi handle/sharedId"
			);
		}
	}

	/// Kills the browser session associated with this page.
	pub async fn kill(self) -> Result<()> { self.session.kill().await }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::webdriver::test_fixtures;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn visits_and_evaluates() {
		let url = test_fixtures::page_url(
			"page",
			"<html><body><h1>Example Domain</h1></body></html>",
		);
		let page = test_fixtures::visit(&url).await;
		page.current_url().await.unwrap().xpect_eq(url);

		page.evaluate_value("document.querySelector('h1')?.textContent")
			.await
			.unwrap()
			.xpect_eq(json!("Example Domain"));

		// objects deep-serialize into plain json
		page.evaluate_value(
			"({count: 1, items: [1, 2], nested: {flag: true}})",
		)
		.await
		.unwrap()
		.xpect_eq(
			json!({"count": 1, "items": [1, 2], "nested": {"flag": true}}),
		);

		// in-page exceptions surface as errors
		page.evaluate("(() => { throw new Error('boom') })()")
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("boom");

		page.kill().await.unwrap();
	}

	/// A url read straight after a click that navigates never errors on the
	/// torn-down context, and `wait_for_url` waits for the destination.
	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn reads_the_url_through_a_navigation() {
		let landing = test_fixtures::page_url(
			"nav_landing",
			"<html><body><h1>Landing</h1></body></html>",
		);
		let start = test_fixtures::page_url(
			"nav_start",
			&format!(
				r#"<html><body><a id="go" href="{landing}">go</a></body></html>"#
			),
		);
		let page = test_fixtures::visit(&start).await;
		page.click("#go").await.unwrap();
		// the read that used to fail with `Cannot find context with specified id`
		page.current_url().await.unwrap();
		page.wait_for_url(Duration::from_secs(10), |url| {
			url.ends_with("nav_landing.html")
		})
		.await
		.unwrap()
		.xpect_eq(landing);
		// a predicate that never holds names where it got stuck
		page.wait_for_url(Duration::from_millis(200), |url| {
			url.ends_with("never.html")
		})
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("still at")
		.xpect_contains("nav_landing.html");
		page.kill().await.unwrap();
	}
}
