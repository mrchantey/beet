// #![cfg_attr(rustfmt, rustfmt_skip)]

// impl SweetBorrow<HtmlElement> for fn() -> Option<Window> {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self().sweet_borrow() } }
// impl SweetBorrow<HtmlElement> for Option<Window> {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.as_ref().unwrap().sweet_borrow() } }
// impl SweetBorrow<HtmlElement> for Window {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.document().unwrap().sweet_borrow() } }

// impl SweetBorrow<HtmlElement> for fn() -> Option<HtmlIFrameElement> {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self().sweet_borrow() } }
// impl SweetBorrow<HtmlElement> for Option<HtmlIFrameElement> {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.as_ref().unwrap().sweet_borrow() } }
// impl SweetBorrow<HtmlElement> for HtmlIFrameElement {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.content_document().unwrap().sweet_borrow() } }


// impl SweetBorrow<HtmlElement> for fn() -> Option<Document> {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self().sweet_borrow() } }
// impl SweetBorrow<HtmlElement> for Option<Document> {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.as_ref().unwrap().sweet_borrow() } }
// impl SweetBorrow<HtmlElement> for Document {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.body().unwrap() } }

// impl<T> SweetBorrow<HtmlElement> for fn() -> Option<T>
// where
// 	T: SweetBorrow<HtmlElement>,
// {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self().unwrap().sweet_borrow()
// 	}
// }
// impl<T> SweetBorrow<HtmlElement> for T
// where
// 	T: SweetBorrow<HtmlElement>,
// {
// 	fn sweet_borrow(&self) -> HtmlElement { (*self).sweet_borrow() }
// }

// impl<T> SweetBorrow<HtmlElement> for Option<T>
// where
// 	T: SweetBorrow<HtmlElement>,
// {
// 	fn sweet_borrow(&self) -> HtmlElement {
// 		self.as_ref().unwrap().sweet_borrow()
// 	}
// }

//ie for window()
