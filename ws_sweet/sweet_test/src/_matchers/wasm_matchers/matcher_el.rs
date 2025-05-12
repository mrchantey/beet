use crate::matchers::*;
// use sweet_web::prelude::*;
use web_sys::HtmlElement;

impl<T> Matcher<T>
where
	T: AsRef<HtmlElement>,
{
	// pub fn get(&self, selector: &str) -> Matcher<HtmlElement> {
	// 	let parent = self.value.as_ref();
	// 	// let expected = format!(
	// 	// 	"element {} to contain selector '{selector}'",
	// 	// 	parent.tag_name()
	// 	// );
	// 	let received = parent.x_query_selector::<HtmlElement>(selector);
	// 	self.assert_option_with_received_negatable(received.clone());
	// 	Matcher::new(received.unwrap())
	// }

	pub fn to_contain_text(&self, other: &str) {
		let receive = self.value.as_ref().text_content().unwrap_or_default();
		self.assert_contains_text(other, &receive, "text");
	}
	pub fn to_contain_visible_text(&self, other: &str) {
		let receive = self.value.as_ref().inner_text();
		self.assert_contains_text(other, &receive, "visible text");
	}
	pub fn to_contain_html(&self, other: &str) {
		let receive = self.value.as_ref().inner_html();
		self.assert_contains_text(other, &receive, "html");
	}
	fn assert_contains_text(
		&self,
		other: &str,
		receive: &str,
		expect_suffix: &str,
	) {
		let result = receive.contains(other);
		let mut received = receive.chars().take(100).collect::<String>();
		if received.len() == 100 {
			received.push_str("...TRUNCATED...");
		}
		let expected = format!("to contain {} '{}'", expect_suffix, other);

		if !self.is_true_with_negated(result) {
			self.panic_with_expected_received(&expected, &received);
		}
	}
}
