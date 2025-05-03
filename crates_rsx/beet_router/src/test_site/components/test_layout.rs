use beet_rsx::as_beet::*;

#[derive(Node)]
pub struct PageLayout {
	pub title: String,
}

fn page_layout(props: PageLayout) -> RsxNode {
	rsx! {
		<html>
			<div>
				<h1>{props.title}</h1>
				<slot />
			</div>
		</html>
	}
}
