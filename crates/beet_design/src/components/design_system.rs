use crate::prelude::*;
use beet_rsx::as_beet::*;
use material_colors::theme::Theme;



/// Entry point for the beet design system.
#[derive(Node)]
pub struct DesignSystem {
	pub theme: Theme,
}


fn design_system(props: DesignSystem) -> RsxNode {
	rsx! {
		<ColorScheme theme={props.theme} />
		{form_style()}
		{layout_style()}
	}
}

fn layout_style() -> RsxNode {
	rsx! {
	<style scope:global>
	:root {
		/* spacing */
		--bm-spacing-xs: 0.5rem;
		--bm-spacing-sm: 0.75rem;
		--bm-spacing: 1rem;
		--bm-spacing-lg: 1.5rem;
		--bm-spacing-xl: 2rem;
		/* border */
		--bm-border-radius: 0.5rem;
		--bm-border-radius-sm: 0.25rem;
		--bm-border: var(--bm-border-width) solid var(--bm-color-border);
		--bm-border-width: 0.0625rem;
		--bm-border-width-xs: calc(var(--bm-border-width) * 0.5);
		--bm-border-width-sm: calc(var(--bm-border-width) * 0.75);
		--bm-border-width-lg: calc(var(--bm-border-width) * 2);
		--bm-border-width-xl: calc(var(--bm-border-width) * 4);

		/* animation */
		--bm-transition: .2s ease-in-out;

		/* layout */
		--md-primary-tab-container-height: 32px;
		--header-height: 48px;
		--subheader-height: 48px;
		--footer-height: 32px;
		--max-page-width: 48rem;
		--max-header-width: 56rem;
		/* using vw instead of 100% is consistent even when scrollbar appears */
		--content-padding-width: calc(max((100dvw - var(--max-header-width)) * 0.5, 1.em));
		--bm-main-height: calc(100dvh - var(--header-height) - var(--footer-height));
		--bm-main-height-subheader: calc(100dvh - var(--header-height) - var(--footer-height) - var(--subheader-height));
	}
	</style>
	}
}

fn form_style() -> RsxNode {
	rsx! {
	<style scope:global>
		:root {
			--bt-form-element-height: 3.em;
			--bt-form-element-font-weight: 600;
			--bt-form-element-font-size: 0.8.em;
			--bt-form-element-border-radius-percent: 0.5;
			--bt-form-element-border-radius: calc(var(--bt-form-element-height) * var(--bt-form-element-border-radius-percent) * 0.5);
		}

		.bt-c-button,
		.bt-c-input,
		.bt-c-select {

			pointer-events: auto;

			height: var(--bt-form-element-height);
			padding: var(--bt-spacing);

			border-radius: var(--bt-form-element-border-radius);
			font-size: var(--bt-form-element-font-size);
			font-weight: var(--bt-form-element-font-weight);
			font-family: var(--bt-font-family);
			border: none;

			display: flex;
			color: var(--bt-color-text);
			background-color: var(--bt-color-background);


		}
		</style>
		}
}
