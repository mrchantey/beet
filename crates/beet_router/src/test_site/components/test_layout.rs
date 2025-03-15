use beet_rsx::as_beet::*;

#[derive(Node)]
pub struct PageLayout {
	pub title: String,
}


impl Component for PageLayout {
	fn render(self) -> RsxRoot {
		rsx! {
			<html>
				<div>
					<h1>{self.title}</h1>
					<slot />
				</div>
			</html>
		}
	}
}
