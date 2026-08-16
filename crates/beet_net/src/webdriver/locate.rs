//! Element location via `browsingContext.locateNodes`.
//!
//! [`Page::find`] and friends poll until a match appears (the playwright and
//! cypress auto-wait shape), bounded by [`Page::timeout`];
//! [`Page::query_selector`] in `page.rs` remains the immediate probe.

use super::Element;
use super::Page;
use super::*;
use beet_core::prelude::*;
use core::time::Duration;
use serde_json::Value;
use serde_json::json;

/// A node locator strategy for `browsingContext.locateNodes`.
#[derive(Debug, Clone)]
pub(crate) enum Locator<'a> {
	/// A css selector, eg `#count`.
	Css(&'a str),
	/// An exact rendered-text match.
	InnerText(&'a str),
}

impl Locator<'_> {
	fn to_json(&self) -> Value {
		match self {
			Self::Css(value) => json!({"type": "css", "value": value}),
			Self::InnerText(value) => {
				json!({"type": "innerText", "value": value})
			}
		}
	}
}

/// Issue a single `browsingContext.locateNodes`, returning an [`Element`]
/// per matched node.
pub(crate) async fn locate_nodes(
	session: &Session,
	context_id: &str,
	locator: &Locator<'_>,
	start_node: Option<&Element>,
	max_count: Option<u64>,
) -> Result<Vec<Element>> {
	let mut params = json!({
		"context": context_id,
		"locator": locator.to_json(),
	});
	if let Some(max_count) = max_count {
		params.set_field("maxNodeCount", json!(max_count))?;
	}
	if let Some(start_node) = start_node {
		params.set_field("startNodes", json!([start_node.shared_ref()?]))?;
	}
	session
		.command("browsingContext.locateNodes", params)
		.await?
		.pointer("/result/nodes")
		.and_then(|nodes| nodes.as_array())
		.ok_or_else(|| bevyhow!("locateNodes: missing nodes array"))?
		.iter()
		.map(|node| Element::from_node_value(session, context_id, node))
		.collect::<Result<Vec<_>>>()
}

/// Poll [`locate_nodes`] until at least one node matches, bounded by
/// `timeout`, returning the first match.
///
/// Open-coded rather than through `poll_ext`: the async-closure's
/// higher-ranked environment lifetime breaks `Send` inference for callers
/// whose action futures must be `Send` (rustc's "implementation of `Send` is
/// not general enough").
pub(crate) async fn find_polling(
	session: &Session,
	context_id: &str,
	locator: &Locator<'_>,
	start_node: Option<&Element>,
	timeout: Duration,
) -> Result<Element> {
	let start = Instant::now();
	loop {
		let expired = start.elapsed() >= timeout;
		match locate_nodes(session, context_id, locator, start_node, Some(1))
			.await?
			.into_iter()
			.next()
		{
			Some(element) => return Ok(element),
			None if expired => {
				bevybail!("no node matching {locator:?}")
			}
			None => time_ext::sleep(poll_ext::DEFAULT_INTERVAL).await,
		}
	}
}

impl Page {
	/// Find the first element matching the css `selector`, polling until one
	/// appears, bounded by [`Page::timeout`].
	pub async fn try_find(&self, selector: &str) -> Result<Element> {
		find_polling(
			&self.session,
			&self.context_id,
			&Locator::Css(selector),
			None,
			self.timeout(),
		)
		.await
	}

	/// Find the first element whose rendered text is exactly `text`, polling
	/// until one appears, bounded by [`Page::timeout`].
	pub async fn try_find_text(&self, text: &str) -> Result<Element> {
		find_polling(
			&self.session,
			&self.context_id,
			&Locator::InnerText(text),
			None,
			self.timeout(),
		)
		.await
	}

	/// All elements currently matching the css `selector`, without waiting.
	pub async fn try_find_all(&self, selector: &str) -> Result<Vec<Element>> {
		locate_nodes(
			&self.session,
			&self.context_id,
			&Locator::Css(selector),
			None,
			None,
		)
		.await
	}
}

impl Element {
	/// Find the first descendant matching the css `selector`, polling until
	/// one appears (the page-default timeout).
	pub async fn try_find(&self, selector: &str) -> Result<Element> {
		find_polling(
			self.session(),
			self.context_id(),
			&Locator::Css(selector),
			Some(self),
			poll_ext::DEFAULT_TIMEOUT,
		)
		.await
	}

	/// All descendants currently matching the css `selector`, without
	/// waiting.
	pub async fn try_find_all(&self, selector: &str) -> Result<Vec<Element>> {
		locate_nodes(
			self.session(),
			self.context_id(),
			&Locator::Css(selector),
			Some(self),
			None,
		)
		.await
	}
}

impl Page {
	/// [`Self::try_find`], panicking on timeout: the test-and-script shape,
	/// so chains read without unwraps.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn find(&self, selector: &str) -> Element {
		self.try_find(selector)
			.await
			.unwrap_or_else(|err| panic!("{err}"))
	}

	/// [`Self::try_find_text`], panicking on timeout.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn find_text(&self, text: &str) -> Element {
		self.try_find_text(text)
			.await
			.unwrap_or_else(|err| panic!("{err}"))
	}

	/// [`Self::try_find_all`], panicking on error (an empty match is `[]`,
	/// not a panic).
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn find_all(&self, selector: &str) -> Vec<Element> {
		self.try_find_all(selector)
			.await
			.unwrap_or_else(|err| panic!("{err}"))
	}
}

impl Element {
	/// [`Self::try_find`], panicking on timeout.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn find(&self, selector: &str) -> Element {
		self.try_find(selector)
			.await
			.unwrap_or_else(|err| panic!("{err}"))
	}

	/// [`Self::try_find_all`], panicking on error.
	#[cfg_attr(feature = "nightly", track_caller)]
	pub async fn find_all(&self, selector: &str) -> Vec<Element> {
		self.try_find_all(selector)
			.await
			.unwrap_or_else(|err| panic!("{err}"))
	}
}
