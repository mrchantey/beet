use beet_core::prelude::*;
use bevy::prelude::*;
use serde_json::Value;
use serde_json::json;

use super::Session;

/// A DOM `Element` handle backed by a WebDriver BiDi remote object id
/// (aka handle / sharedId – we probe both field names to remain tolerant
/// across browser implementations while the spec & drivers converge).
///
/// Responsibilities:
/// * Store the BiDi handle so subsequent `script.callFunction` commands
///   can target the same remote element without re-querying.
/// * Provide ergonomic helpers for common DOM interactions (click,
///   get/set attribute, get/set property).
///
/// Lifetimes & Ownership:
/// * The underlying remote handle is kept alive by using
///   `"resultOwnership": "root"` when the element is first produced
///   (expected in the caller – e.g. `Page::query_selector`).
/// * If the node is removed from the DOM the handle may become stale;
///   current helpers will surface driver errors in that case.
///
/// NOTE:
/// The exact BiDi field for referencing a previously returned remote
/// object can be `handle` (Chrome) or `sharedId` (Firefox in some
/// channels). We store whichever we locate first and always send it as
/// `"handle"` in `script.callFunction` params (Chrome & current spec).
#[derive(Debug, Clone)]
pub struct Element {
	session: Session,
	context_id: String,
	handle: String,
}

impl Element {
	/// Create directly (used internally / by `Page`).
	pub(crate) fn new(
		session: &Session,
		context_id: &str,
		handle: &str,
	) -> Self {
		Self {
			session: session.clone(),
			context_id: context_id.to_string(),
			handle: handle.to_string(),
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
		let handle = resp
			.pointer("/result/result/handle")
			.and_then(|v| v.as_str())
			.or_else(|| {
				resp.pointer("/result/result/sharedId")
					.and_then(|v| v.as_str())
			})?;
		Some(Self::new(session, context_id, handle))
	}

	/// Access the raw BiDi handle (debug / advanced use).
	pub fn handle(&self) -> &str { &self.handle }

	/// Low-level helper: invoke a function with `this` bound to element.
	async fn call_function(
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
					"this": { "handle": self.handle },
					"arguments": arguments,
					"target": { "context": self.context_id },
					"awaitPromise": true,
					"resultOwnership": ownership
				}),
			)
			.await
	}

	/// Click the element (dispatches the default `.click()`).
	pub async fn click(&self) -> Result<()> {
		self.call_function(
			"function(){ this.click(); return true; }",
			&[],
			false,
		)
		.await?;
		Ok(())
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
	/// InnerTExt convenience getter.
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
#[cfg(feature = "tokio")]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn visit_and_read_title() {
		App::default()
			.run_io_task(async move {
				let (proc, page) =
					Page::visit("https://example.com").await.unwrap();
				page.current_url()
					.await
					.unwrap()
					.xpect_eq("https://example.com/");

				page.query_selector("h1")
					.await
					.unwrap()
					.unwrap()
					.inner_text()
					.await
					.unwrap()
					.xpect_eq("Example Domain");

				let anchor = page.query_selector("a").await.unwrap().unwrap();
				anchor
					.inner_text()
					.await
					.unwrap()
					.xpect_eq("More information...");
				anchor.click().await.unwrap();

				page.current_url()
					.await
					.unwrap()
					.xpect_eq("https://www.iana.org/help/example-domains");

				page.kill().await.unwrap();
				proc.kill().await.unwrap();
			})
			.await;
	}
}
