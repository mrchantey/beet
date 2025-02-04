use beet_rsx::as_beet::beet;
use beet_rsx::prelude::*;


pub struct PageLayout {
	pub title: String,
}


impl Component for PageLayout {
	fn render(self) -> impl Rsx {
		rsx! {
			<html>
				<div>
						<h1>{self.title}</h1>
						<slot/>
				</div>
			</html>
		}
	}
}
