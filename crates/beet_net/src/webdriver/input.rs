//! Trusted input via `input.performActions`.
//!
//! These dispatch real pointer and key events through the browser's input
//! pipeline: hit-testing applies, `isTrusted` is true, and focus follows the
//! pointer, exactly like a user. To poke the DOM regardless of visibility use
//! [`Page::evaluate`] instead.

use super::Page;
use super::WebElement;
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

/// A keyDown/keyUp pair per character of `text`, typed in order.
fn key_presses(text: &str) -> impl Iterator<Item = Value> + '_ {
	text.chars().flat_map(|ch| key_press(&ch.to_string()))
}

impl WebElement {
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

	/// Focus the element as a page script does (`focus()`): the `focusin`
	/// without the click, for a control whose trusted click opens a native
	/// popup no key action reaches in a headless browser (a `<select>`), so
	/// the keys that follow land on the control closed.
	pub async fn focus(&self) -> Result<()> {
		self.call_function(
			"function(){ this.focus(); return true; }",
			&[],
			false,
		)
		.await?;
		Ok(())
	}

	/// Send a trusted key press to the element, [focused](Self::focus) first.
	pub async fn press(&self, key: Key) -> Result<()> {
		self.focus().await?;
		perform(
			self.session(),
			self.context_id(),
			"key",
			key_press(&key_value(&key)?).into(),
		)
		.await
	}

	/// Focus the element with a trusted click, move the caret to the end and
	/// type `text` as trusted key events, character by character: an append,
	/// wherever in the existing text the click landed.
	pub async fn type_text(&self, text: &str) -> Result<()> {
		self.click().await?;
		let actions = key_press(&key_value(&Key::End)?)
			.into_iter()
			.chain(key_presses(text))
			.collect();
		perform(self.session(), self.context_id(), "key", actions).await
	}

	/// Choose the option of a `<select>` whose label starts with `prefix`,
	/// through the browser's own type-ahead: the select [focused](Self::focus)
	/// closed and the prefix typed as trusted keys, so `input` and `change`
	/// fire as they do for a keyboard user. The search starts at the chosen
	/// option and wraps, the first label matching the whole prefix winning,
	/// so name enough of it to be unique; a prefix typed within a second of
	/// the last continues it rather than starting over.
	pub async fn type_ahead(&self, prefix: &str) -> Result<()> {
		self.focus().await?;
		let actions = key_presses(prefix).collect();
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

	/// Typing appends: the click lands where it lands and the caret is moved
	/// to the end before the keys, so the text typed follows the text held,
	/// and the caret stays at the end of what was typed.
	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn typing_appends_and_leaves_the_caret_at_the_end() {
		let url = test_fixtures::page_url(
			"append",
			r#"<html><body>
			<input id="name" value="Garden" style="width: 40em"/>
			</body></html>"#,
		);
		let page = test_fixtures::visit(&url).await;
		let input = page.find("#name").await;
		// the caret at the start, where the trusted click would otherwise
		// leave it in a wide field
		input
			.call_function(
				"function(){ this.setSelectionRange(0, 0); return true; }",
				&[],
				false,
			)
			.await
			.unwrap();
		input.type_text(" Party").await.unwrap();
		input.xpect_value("Garden Party").await;
		page.evaluate_value("document.activeElement.selectionStart")
			.await
			.unwrap()
			.as_u64()
			.xpect_eq(Some("Garden Party".len() as u64));
		page.kill().await.unwrap();
	}

	/// Type-ahead on a focused, closed select chooses the first option whose
	/// label starts with what was typed, and the browser fires `change` for
	/// it as it does for a keyboard user.
	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn type_ahead_picks_an_option() {
		let url = test_fixtures::page_url(
			"type_ahead",
			r#"<html><body>
			<select id="pick">
				<option value="0">#0 main</option>
				<option value="1">#1 h1</option>
				<option value="10">#10 li</option>
				<option value="name">Name</option>
				<option value="nav">Navigator</option>
			</select>
			<div id="out"></div>
			<script>
			const pick = document.querySelector('#pick');
			pick.addEventListener('change', (ev) => {
				if (ev.isTrusted) {
					document.querySelector('#out').textContent = pick.value;
				}
			});
			</script>
			</body></html>"#,
		);
		let page = test_fixtures::visit(&url).await;
		let pick = page.find("#pick").await;
		pick.type_ahead("Na").await.unwrap();
		pick.xpect_value("name").await;
		page.find("#out").await.xpect_text("name").await;
		// a prefix shared by several options picks the first in order, from a
		// fresh session: the browser appends keys typed within a second of the
		// last to the same prefix
		time_ext::sleep(Duration::from_millis(1100)).await;
		pick.type_ahead("#1").await.unwrap();
		pick.xpect_value("1").await;
		page.find("#out").await.xpect_text("1").await;
		page.kill().await.unwrap();
	}
}
