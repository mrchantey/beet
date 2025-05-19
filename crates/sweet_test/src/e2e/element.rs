use crate::prelude::*;

/// A single DOM element on the current page.
///
/// Note that there is a lot of subtlety in how you can interact with an element through WebDriver,
/// which [the WebDriver standard goes into detail on](https://www.w3.org/TR/webdriver1/#elements).
/// The same goes for inspecting [element state](https://www.w3.org/TR/webdriver1/#element-state).
#[derive(Debug)]
pub struct Element {
	pub inner: fantoccini::elements::Element,
}


impl Element {
	pub fn new(inner: fantoccini::elements::Element) -> Self { Self { inner } }
}


impl std::ops::Deref for Element {
	type Target = fantoccini::elements::Element;
	fn deref(&self) -> &Self::Target { &self.inner }
}

impl AsRef<Element> for Element {
	fn as_ref(&self) -> &Element { self }
}

impl<T: AsRef<Element>> Matcher<T> {
	/// Assert that the element has the given text.
	pub async fn to_have_text(&self, text: &str) {
		let value = self.value.as_ref().inner.text().await.unwrap();
		self.assert_correct_with_received(
			value == text,
			&text,
			&value.as_str(),
		);
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn works() {
		let el = visit("https://example.com").await.find_css("h1").await;
		el.as_ref().xpect().to_have_text("Example Domain").await;
		el.xpect().not().to_have_text("foobar").await;
	}
	#[crate::test]
	async fn links() {
		let page = visit("https://example.com").await;
		page.find_link_text("More information...")
			.await
			.click()
			.await
			.unwrap();
		expect(&page)
			.to_have_url("https://www.iana.org/help/example-domains")
			.await;
	}
}
