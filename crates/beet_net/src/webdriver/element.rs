use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

use super::Session;

/// A DOM `Element` backed by a WebDriver BiDi remote reference.
///
/// A node value can carry two reference kinds and we retain both:
/// * `shared_id`, the navigable-scoped stable node id. Required wherever the
///   protocol wants a `SharedReference`: `startNodes` scoping, `input`
///   element origins and screenshot clips. Present on every node produced by
///   `browsingContext.locateNodes` (so [`Page::find`] and friends), and on
///   script results in current Chrome and Firefox.
/// * `handle`, the realm-scoped remote object handle, present when the
///   element came from a script result with `"resultOwnership": "root"`.
///
/// If the node is removed from the DOM the reference goes stale and helpers
/// surface the driver error.
#[derive(Debug, Clone)]
pub struct Element {
	session: Session,
	context_id: String,
	handle: Option<String>,
	shared_id: Option<String>,
}

impl Element {
	/// Create directly; at least one of `handle` / `shared_id` must be set.
	fn new(
		session: &Session,
		context_id: &str,
		handle: Option<String>,
		shared_id: Option<String>,
	) -> Self {
		debug_assert!(handle.is_some() || shared_id.is_some());
		Self {
			session: session.clone(),
			context_id: context_id.to_string(),
			handle,
			shared_id,
		}
	}

	/// Attempt to construct from a BiDi `script.evaluate` (or
	/// `script.callFunction`) response JSON that contains a remote node
	/// value at `/result/result`.
	pub(crate) fn from_bidi_response(
		session: &Session,
		context_id: &str,
		resp: &Value,
	) -> Option<Self> {
		let field = |name: &str| {
			resp.pointer(&format!("/result/result/{name}"))
				.and_then(|v| v.as_str())
				.map(|v| v.to_string())
		};
		let handle = field("handle");
		let shared_id = field("sharedId");
		(handle.is_some() || shared_id.is_some())
			.then(|| Self::new(session, context_id, handle, shared_id))
	}

	/// Construct from a top-level node remote value, eg an entry of
	/// `browsingContext.locateNodes`' `nodes` array.
	pub(crate) fn from_node_value(
		session: &Session,
		context_id: &str,
		node: &Value,
	) -> Result<Self> {
		let field = |name: &str| {
			node.get(name)
				.and_then(|v| v.as_str())
				.map(|v| v.to_string())
		};
		let handle = field("handle");
		let shared_id = field("sharedId");
		if handle.is_none() && shared_id.is_none() {
			bevybail!("node value carries neither sharedId nor handle: {node}");
		}
		Self::new(session, context_id, handle, shared_id).xok()
	}

	/// The `RemoteReference` for `script.callFunction`'s `this`, carrying
	/// whichever reference kinds are present.
	fn remote_ref(&self) -> Value {
		let mut reference = serde_json::Map::new();
		if let Some(shared_id) = &self.shared_id {
			reference.insert("sharedId".into(), json!(shared_id));
		}
		if let Some(handle) = &self.handle {
			reference.insert("handle".into(), json!(handle));
		}
		Value::Object(reference)
	}

	/// The `SharedReference` required by `startNodes`, `input` element
	/// origins and screenshot clips, erroring when only a realm handle is
	/// held (elements from [`Page::find`] and [`Page::query_selector`]
	/// always carry a `sharedId`).
	pub(crate) fn shared_ref(&self) -> Result<Value> {
		let shared_id = self.shared_id.as_ref().ok_or_else(|| {
			bevyhow!("this operation needs a BiDi sharedId, ie an element produced by `find` or `query_selector`")
		})?;
		match &self.handle {
			Some(handle) => json!({"sharedId": shared_id, "handle": handle}),
			None => json!({"sharedId": shared_id}),
		}
		.xok()
	}

	/// The session this element's browsing context belongs to.
	pub(crate) fn session(&self) -> &Session { &self.session }
	/// The browsing context id this element lives in.
	pub(crate) fn context_id(&self) -> &str { &self.context_id }

	/// Low-level helper: invoke a function with `this` bound to element.
	pub(crate) async fn call_function(
		&self,
		function_declaration: &str,
		arguments: &[Value],
		result_ownership_root: bool,
	) -> Result<Value> {
		let ownership = if result_ownership_root {
			"root"
		} else {
			"none"
		};
		self.session
			.command(
				"script.callFunction",
				json!({
					"functionDeclaration": function_declaration,
					"this": self.remote_ref(),
					"arguments": arguments,
					"target": { "context": self.context_id },
					"awaitPromise": true,
					"resultOwnership": ownership
				}),
			)
			.await
	}

	/// Get an attribute value (None if absent).
	pub async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
		let resp = self
			.call_function(
				"function(n){ return this.getAttribute(n); }",
				&[json!({ "type": "string", "value": name })],
				true,
			)
			.await?;
		let val = resp
			.pointer("/result/result/value")
			.and_then(|v| v.as_str())
			.map(|s| s.to_string());
		Ok(val)
	}

	/// Set an attribute value (returns ()).
	pub async fn set_attribute(&self, name: &str, value: &str) -> Result<()> {
		self.call_function(
			"function(n,v){ this.setAttribute(n,v); return true; }",
			&[
				json!({ "type": "string", "value": name }),
				json!({ "type": "string", "value": value }),
			],
			false,
		)
		.await?;
		Ok(())
	}

	/// Remove an attribute if present.
	pub async fn remove_attribute(&self, name: &str) -> Result<()> {
		self.call_function(
			"function(n){ this.removeAttribute(n); return true; }",
			&[json!({ "type": "string", "value": name })],
			false,
		)
		.await?;
		Ok(())
	}

	/// Get a (enumerable) JS property. Returns the BiDi remote value
	/// JSON (caller can interpret / extract).
	pub async fn get_property(&self, name: &str) -> Result<Value> {
		self.call_function(
			"function(n){ return this[n]; }",
			&[json!({ "type": "string", "value": name })],
			true,
		)
		.await
	}

	/// Set a JS property (primitive string / number / bool / null only
	/// for now). Returns the updated remote value JSON.
	pub async fn set_property(
		&self,
		name: &str,
		value: Value,
	) -> Result<Value> {
		// Convert simple JSON value into a BiDi argument remote value.
		let bidi_arg = if value.is_string() {
			json!({ "type": "string", "value": value.as_str().unwrap() })
		} else if value.is_number() {
			if let Some(n) = value.as_i64() {
				json!({ "type": "number", "value": n })
			} else if let Some(n) = value.as_u64() {
				json!({ "type": "number", "value": n })
			} else if let Some(f) = value.as_f64() {
				json!({ "type": "number", "value": f })
			} else {
				json!({ "type": "undefined" })
			}
		} else if value.is_boolean() {
			json!({ "type": "boolean", "value": value.as_bool().unwrap() })
		} else if value.is_null() {
			json!({ "type": "null" })
		} else {
			// Fallback: stringify complex values client-side then send.
			json!({ "type": "string", "value": value.to_string() })
		};

		self.call_function(
			"function(n,v){ this[n] = v; return this[n]; }",
			&[json!({ "type": "string", "value": name }), bidi_arg],
			true,
		)
		.await
	}

	/// InnerHTML convenience getter.
	pub async fn inner_html(&self) -> Result<String> {
		let resp = self
			.call_function("function(){ return this.innerHTML; }", &[], true)
			.await?;
		resp.pointer("/result/result/value")
			.and_then(|v| v.as_str())
			.ok_or_else(|| bevyhow!("innerHTML missing in response"))?
			.to_string()
			.xok()
	}

	/// Set innerHTML
	pub async fn set_inner_html(&self, html: &str) -> Result<()> {
		self.call_function(
			"function(h){ this.innerHTML = h; return true; }",
			&[json!({ "type": "string", "value": html })],
			false,
		)
		.await?;
		Ok(())
	}
	/// InnerText convenience getter.
	pub async fn inner_text(&self) -> Result<String> {
		let resp = self
			.call_function("function(){ return this.innerText; }", &[], true)
			.await?;
		resp.pointer("/result/result/value")
			.and_then(|v| v.as_str())
			.ok_or_else(|| bevyhow!("innerText missing in response"))?
			.to_string()
			.xok()
	}

	/// Set innerText
	pub async fn set_inner_text(&self, html: &str) -> Result<()> {
		self.call_function(
			"function(h){ this.innerText = h; return true; }",
			&[json!({ "type": "string", "value": html })],
			false,
		)
		.await?;
		Ok(())
	}

	/// Text content convenience getter.
	pub async fn text_content(&self) -> Result<Option<String>> {
		let resp = self
			.call_function("function(){ return this.textContent; }", &[], true)
			.await?;
		let val = resp
			.pointer("/result/result/value")
			.and_then(|v| v.as_str())
			.map(|s| s.to_string());
		Ok(val)
	}
}

#[cfg(test)]
mod test {
	use crate::webdriver::test_fixtures;
	use beet_core::prelude::*;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn finds_reads_and_navigates() {
		App::default()
			.run_io_task_local(async move {
				let page_b = test_fixtures::page_url(
					"element_b",
					"<html><body><h1>Page B</h1></body></html>",
				);
				let page_a = test_fixtures::page_url(
					"element_a",
					&format!(
						r#"<html><body>
						<h1>Example Domain</h1>
						<p><a href="{page_b}">More information...</a></p>
						</body></html>"#
					),
				);
				let (proc, page) = test_fixtures::visit(&page_a).await;

				// auto-waiting css find + matcher chain
				page.get("h1")
					.await
					.xpect_text("Example Domain")
					.await
					.xpect_not_text("foobar")
					.await;
				page.find_all("a").await.unwrap().len().xpect_eq(1);

				// element-scoped find reads the real attribute
				page.get("p")
					.await
					.get("a")
					.await
					.xpect_attr("href", &page_b)
					.await;
				page.xpect_no_selector("#missing").await;

				// text locator + trusted click navigates; xpect_url waits
				page.find_text("More information...")
					.await
					.unwrap()
					.click()
					.await
					.unwrap();
				page.xpect_url(&page_b).await;
				page.get("h1").await.xpect_text("Page B").await;

				page.kill().await.unwrap();
				proc.kill().unwrap();
			})
			.await;
	}
}
