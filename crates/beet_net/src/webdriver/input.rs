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
use bevy::input::keyboard::Key;
use serde_json::Value;
use serde_json::json;

/// The WebDriver codepoint for a logical [`Key`], see
/// <https://www.w3.org/TR/webdriver2/#keyboard-actions>. Characters pass
/// through literally; keys with no WebDriver encoding error.
fn key_value(key: &Key) -> Result<String> {
	match key {
		Key::Character(text) => text.to_string(),
		Key::Space => " ".to_string(),
		Key::Backspace => "\u{E003}".to_string(),
		Key::Tab => "\u{E004}".to_string(),
		Key::Enter => "\u{E007}".to_string(),
		Key::Shift => "\u{E008}".to_string(),
		Key::Control => "\u{E009}".to_string(),
		Key::Alt => "\u{E00A}".to_string(),
		Key::Escape => "\u{E00C}".to_string(),
		Key::PageUp => "\u{E00E}".to_string(),
		Key::PageDown => "\u{E00F}".to_string(),
		Key::End => "\u{E010}".to_string(),
		Key::Home => "\u{E011}".to_string(),
		Key::ArrowLeft => "\u{E012}".to_string(),
		Key::ArrowUp => "\u{E013}".to_string(),
		Key::ArrowRight => "\u{E014}".to_string(),
		Key::ArrowDown => "\u{E015}".to_string(),
		Key::Delete => "\u{E017}".to_string(),
		other => bevybail!("key has no webdriver encoding: {other:?}"),
	}
	.xok()
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
	/// Send a single trusted key press to the focused element, eg
	/// `page.press(Key::Enter)` or `page.press(Key::Character("a".into()))`.
	pub async fn press(&self, key: Key) -> Result<()> {
		perform(
			&self.session,
			&self.context_id,
			"key",
			key_press(&key_value(&key)?).into(),
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
			match self.try_find(selector).await?.click().await {
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
			match self.try_find_text(text).await?.click().await {
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
	use bevy::input::keyboard::Key;

	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn types_trusted_text() {
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
		let page = test_fixtures::visit(&url).await;
		let input = page.find("#name").await;
		input.type_text("hello beet").await.unwrap();
		input.xpect_value("hello beet").await;
		page.press(Key::Enter).await.unwrap();
		page.find("#out").await.xpect_text("hello beet").await;
		page.kill().await.unwrap();
	}
}
