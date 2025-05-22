use beet_rsx::as_beet::*;

#[derive(derive_template)]
pub struct PageLayout {
	pub title: String,
}

fn page_layout(props: PageLayout) -> WebNode {
	rsx! {
		<html>
			<div>
				<h1>{props.title}</h1>
				<slot />
			</div>
			<style>
				h1{
					color: red;
				}
			</style>
			<script>
				alert("your fridge is running");
			</script>
		</html>
	}
}
