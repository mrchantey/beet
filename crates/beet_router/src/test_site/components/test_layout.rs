use beet_template::as_beet::*;

#[template]
fn PageLayout(title: String) -> impl Bundle {
	rsx! {
		<html>
			<div>
				<h1>{title}</h1>
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
