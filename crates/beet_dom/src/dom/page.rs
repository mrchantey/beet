use beet_core::prelude::*;
use bevy::prelude::Result;
use serde_json::Value;
use serde_json::json;

use super::Client;
use super::ClientProcess;
use super::Element;
use super::Session;

/// High level ergonomic wrapper over a BiDi `Session` bound to a single
/// top-level browsing context (page / tab).
///
/// Responsibilities:
/// * Discover an initial browsing context (via `browsingContext.getTree`)
/// * Provide convenience helpers (`navigate`, `evaluate`, `get_current_url`)
/// * Future: element querying returning typed `Element` handles that track
///   BiDi object ids (currently a stub until `Element` is expanded)
///
/// Construction Patterns:
/// * `Page::from_session(session)` – bind an already-created session
/// * `Page::visit(url)` – spawn default driver (chromium), create session,
///    navigate, return `(ClientProcess, Page)` so the caller can clean up
/// * `Page::visit_with_client(client, url)` – same but with a custom client
///
#[derive(Debug, Clone)]
pub struct Page {
	session: Session,
	context_id: String,
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
		})
	}

	/// Spawn a default (chromium) driver process, create a session,
	/// navigate to `url` and return both process + page.
	#[cfg(all(feature = "tokio", feature = "webdriver"))]
	pub async fn visit(url: &str) -> Result<(ClientProcess, Self)> {
		let process = ClientProcess::new()?;
		let session = process.new_session().await?;
		let mut page = Self::from_session(session).await?;
		page.navigate(url).await?;
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
			.await?;
		Ok(())
	}

	/// Evaluate a JavaScript expression in the page's browsing context.
	/// Returns the full BiDi response JSON (caller can drill down).
	pub async fn evaluate(&self, expression: &str) -> Result<Value> {
		self.session
			.command(
				"script.evaluate",
				json!({
					"expression": expression,
					"target": { "context": self.context_id },
					"awaitPromise": true,
					"resultOwnership": "root"
				}),
			)
			.await
	}

	/// Get current page URL (string convenience wrapper).
	pub async fn get_current_url(&self) -> Result<String> {
		self.evaluate("location.href")
			.await?
			.pointer("/result/result/value")
			.and_then(|v| v.as_str())
			.ok_or_else(|| bevyhow!("missing location.href value"))?
			.to_string()
			.xok()
	}

	/// Placeholder for PDF export (varies per driver – often requires
	/// CDP or print-to-PDF endpoints). Stub for now.
	pub async fn export_pdf(&self) -> Result<Vec<u8>> {
		Err(bevyhow!("export_pdf not yet implemented"))
	}

	/// Query a single element. Currently returns a stub `Element`
	/// (no object id tracking yet). Returns `Ok(None)` if not found.
	///
	/// Future: augment `Element` to store the BiDi remote object / handle
	/// id, validate liveness, and lazily recover if discarded.
	pub async fn query_selector(
		&self,
		selector: &str,
	) -> Result<Option<Element>> {
		let expr = format!("document.querySelector({selector:?})");
		let resp = self.evaluate(&expr).await?;
		let ty = resp
			.pointer("/result/result/type")
			.and_then(|v| v.as_str())
			.unwrap_or("undefined");
		if ty == "null" {
			return Ok(None);
		}
		Ok(Some(Element {}))
	}

	pub async fn kill(self) -> Result<()> { self.session.kill().await }
}

#[cfg(all(test, feature = "tokio", feature = "webdriver"))]
mod test {
	use super::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn visit_and_read_title() {
		App::default()
			.run_io_task(async move {
				let (proc, page) =
					Page::visit("https://example.com").await.unwrap();
				page.get_current_url()
					.await
					.unwrap()
					.xpect_eq("https://example.com/");

				page.evaluate("document.querySelector('h1')?.textContent")
					.await
					.unwrap()
					.pointer("/result/result/value")
					.and_then(|v| v.as_str())
					.unwrap()
					.to_string()
					.xpect_eq("Example Domain");
				page.kill().await.unwrap();
				proc.kill().await.unwrap();
			})
			.await;
	}
}
