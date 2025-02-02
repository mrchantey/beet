use beet::prelude::*;




pub struct PageLayout {
	pub title: String,
}


impl Rsx for PageLayout {
	fn into_rsx(self) -> RsxNode {
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
