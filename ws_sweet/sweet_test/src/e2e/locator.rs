use crate::prelude::*;
use fantoccini::Locator;
use fantoccini::error;
use sweet_utils::prelude::*;


#[allow(async_fn_in_trait)]
pub trait LocatorExt {
	async fn find(&self, search: Locator) -> Result<Element, error::CmdError>;
	async fn find_all(
		&self,
		search: Locator,
	) -> Result<Vec<Element>, error::CmdError>;

	/// Find an element matching the given [CSS selector][1].
	///
	/// [1]: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Selectors
	async fn find_css(&self, selector: &str) -> Element {
		self.find(Locator::Css(selector)).await.unwrap()
	}

	/// Find an element using the given [`id`][1].
	///
	/// [1]: https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/id
	async fn find_id(&self, id: &str) -> Element {
		self.find(Locator::Id(id)).await.unwrap()
	}

	/// Find a link element with the given link text.
	///
	/// The text matching is exact.
	async fn find_link_text(&self, text: &str) -> Element {
		self.find(Locator::LinkText(text)).await.unwrap()
	}

	/// Find an element using the given [XPath expression][1].
	///
	/// You can address pretty much any element this way, if you're willing to
	/// put in the time to find the right XPath.
	///
	/// [1]: https://developer.mozilla.org/en-US/docs/Web/XPath
	async fn find_xpath(&self, xpath: &str) -> Element {
		self.find(Locator::XPath(xpath)).await.unwrap()
	}
}


impl LocatorExt for Page {
	async fn find(
		&self,
		search: Locator<'_>,
	) -> Result<Element, error::CmdError> {
		let el = self.client.find(search).await?;
		Ok(Element::new(el))
	}
	async fn find_all(
		&self,
		search: Locator<'_>,
	) -> Result<Vec<Element>, error::CmdError> {
		self.client
			.find_all(search)
			.await?
			.into_iter()
			.map(|el| Element::new(el))
			.collect::<Vec<_>>()
			.xok()
	}
}
impl LocatorExt for Element {
	async fn find(
		&self,
		search: Locator<'_>,
	) -> Result<Element, error::CmdError> {
		let el = self.inner.find(search).await?;
		Ok(Element::new(el))
	}
	async fn find_all(
		&self,
		search: Locator<'_>,
	) -> Result<Vec<Element>, error::CmdError> {
		self.inner
			.find_all(search)
			.await?
			.into_iter()
			.map(|el| Element::new(el))
			.collect::<Vec<_>>()
			.xok()
	}
}
