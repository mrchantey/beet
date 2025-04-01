use beet_rsx::as_beet::*;



#[derive(Node)]
pub struct Css;



fn css(_: Css) -> RsxNode {
	rsx! {
		<style scope:global src="typography.css" />
		<style scope:global src="form.css" />
		<style scope:global src="layout.css" />
		<style scope:global src="reset.css" />
		<style scope:global src="utility.css" />
	}
}
