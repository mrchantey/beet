use crate::prelude::*;

#[template]
pub fn Css() -> impl Bundle {
	rsx! {
		<style scope:global src="./elements/code.css" />
		<style scope:global src="./elements/details.css" />
		<style scope:global src="./elements/embedded.css" />
		<style scope:global src="./elements/headings.css" />
		<style scope:global src="./elements/table.css" />
		<style scope:global src="./elements/text.css" />
		<style scope:global src="./variables/color.css" />
		<style scope:global src="./variables/typography.css" />
		<style scope:global src="./accessibility.css" />
		<style scope:global src="./form.css" />
		<style scope:global src="./layout.css" />
		<style scope:global src="./reset.css" />
		<style scope:global src="./utility.css" />
	}
}
