//! Trusted input via `input.performActions`.
//!
//! These dispatch real pointer and key events through the browser's input
//! pipeline: hit-testing applies, `isTrusted` is true, and focus follows the
//! pointer, exactly like a user. To poke the DOM regardless of visibility use
//! [`Page::evaluate`] instead.

use super::Element;
use super::Page;
use super::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// WebDriver key codepoints for named keys, see
/// <https://www.w3.org/TR/webdriver2/#keyboard-actions>.
fn key_value(key: &str) -> String {
	match key {
		"Backspace" => "\u{E003}",
		"Tab" => "\u{E004}",
		"Enter" => "\u{E007}",
		"Shift" => "\u{E008}",
		"Control" => "\u{E009}",
		"Alt" => "\u{E00A}",
		"Escape" => "\u{E00C}",
		"PageUp" => "\u{E00E}",
		"PageDown" => "\u{E00F}",
		"End" => "\u{E010}",
		"Home" => "\u{E011}",
		"ArrowLeft" => "\u{E012}",
		"ArrowUp" => "\u{E013}",
		"ArrowRight" => "\u{E014}",
		"ArrowDown" => "\u{E015}",
		"Delete" => "\u{E017}",
		other => other,
	}
	.to_string()
}

/// A keyDown/keyUp pair for one key value.
fn key_press(value: &str) -> [Value; 2] {
	[
		json!({"type": "keyDown", "value": value}),
		json!({"type": "keyUp", "value": value}),
	]
}

/// Issue one `input.performActions` with a single source of `actions`.
async fn perform(
	session: &Session,
	context_id: &str,
	source_type: &str,
	actions: Vec<Value>,
) -> Result<()> {
	session
		.command(
			"input.performActions",
			json!({
				"context": context_id,
				"actions": [{
					"type": source_type,
					"id": format!("beet {source_type}"),
					"actions": actions,
				}],
			}),
		)
		.await?;
	Ok(())
}

impl Element {
	/// Click the element center with a trusted pointer action. Hit-testing
	/// applies: a covered or invisible element does not receive the click.
	pub async fn click(&self) -> Result<()> {
		// mirror classic WebDriver element click: scroll into view first,
		// since a pointer element origin errors on out-of-viewport targets
		self.call_function(
			"function(){ this.scrollIntoView({block: 'center', inline: 'center', behavior: 'instant'}); return true; }",
			&[],
			false,
		)
		.await?;
		perform(self.session(), self.context_id(), "pointer", vec![
			json!({
				"type": "pointerMove",
				"x": 0,
				"y": 0,
				"origin": {"type": "element", "element": self.shared_ref()?},
			}),
			json!({"type": "pointerDown", "button": 0}),
			json!({"type": "pointerUp", "button": 0}),
		])
		.await
	}

	/// Focus the element with a trusted click, then type `text` as trusted
	/// key events, character by character.
	pub async fn type_text(&self, text: &str) -> Result<()> {
		self.click().await?;
		let actions = text
			.chars()
			.flat_map(|ch| key_press(&ch.to_string()))
			.collect();
		perform(self.session(), self.context_id(), "key", actions).await
	}
}

impl Page {
	/// Send a single trusted key press to the focused element. Named keys
	/// (`"Enter"`, `"Tab"`, `"Escape"`, `"Backspace"`, `"ArrowDown"`, ...)
	/// map to their WebDriver codepoints; any other string is sent literally.
	pub async fn press(&self, key: &str) -> Result<()> {
		perform(
			&self.session,
			&self.context_id,
			"key",
			key_press(&key_value(key)).into(),
		)
		.await
	}

	/// Find the first css match and click it, re-querying on staleness: a
	/// re-render between locate and click invalidates the node reference
	/// ("no such element"), so each retry starts from a fresh find. Bounded
	/// by [`Page::timeout`]. Open-coded loops for the same `Send`-inference
	/// reason as `find_polling`.
	pub async fn click(&self, selector: &str) -> Result<()> {
		let start = Instant::now();
		loop {
			let expired = start.elapsed() >= self.timeout();
			match self.find(selector).await?.click().await {
				Ok(()) => return Ok(()),
				Err(err) if expired => return Err(err),
				Err(_) => time_ext::sleep(poll_ext::DEFAULT_INTERVAL).await,
			}
		}
	}

	/// [`Self::click`] by exact rendered text instead of a css selector.
	pub async fn click_text(&self, text: &str) -> Result<()> {
		let start = Instant::now();
		loop {
			let expired = start.elapsed() >= self.timeout();
			match self.find_text(text).await?.click().await {
				Ok(()) => return Ok(()),
				Err(err) if expired => return Err(err),
				Err(_) => time_ext::sleep(poll_ext::DEFAULT_INTERVAL).await,
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::webdriver::test_fixtures;
	use beet_core::prelude::*;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn types_trusted_text() {
		App::default()
			.run_io_task_local(async move {
				let url = test_fixtures::page_url(
					"input",
					r#"<html><body>
					<input id="name" />
					<div id="out"></div>
					<script>
					const input = document.querySelector('#name');
					input.addEventListener('keydown', (ev) => {
						// isTrusted proves the events came through the real
						// input pipeline, not a synthetic dispatch
						if (ev.key === 'Enter' && ev.isTrusted) {
							document.querySelector('#out').textContent = input.value;
						}
					});
					</script>
					</body></html>"#,
				);
				let (proc, page) = test_fixtures::visit(&url).await;
				let input = page.get("#name").await;
				input.type_text("hello beet").await.unwrap();
				input.xpect_value("hello beet").await;
				page.press("Enter").await.unwrap();
				page.get("#out").await.xpect_text("hello beet").await;
				page.kill().await.unwrap();
				proc.kill().unwrap();
			})
			.await;
	}
}
