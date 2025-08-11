use crate::prelude::*;
use chrono::Datelike;

#[template]
pub fn Footer(config: Res<PackageConfig>) -> impl Bundle {
	let PackageConfig {
		name,
		version,
		stage,
		..
	} = config.as_ref();

	let current_year = chrono::Utc::now().year();
	let footer_text = format!("&copy; {name} {current_year}");

	let mut build_text = format!("v{version}");

	#[cfg(debug_assertions)]
	build_text.push_str(" | build=debug");

	if stage != "prod" {
		build_text.push_str(&format!(" | stage={stage}"));
	}

	// <!-- <a href="/privacy-policy">Privacy</a> -->
	// <!-- <a href="/terms-of-service">Terms</a> -->

	rsx! {
		<footer id="page-footer">
			<span>{footer_text}</span>
			<slot/>
			<span>{build_text}</span>
		</footer>
		<style>
		footer {
			display: flex;
			height: var(--bt-footer-height);
			padding: 0.em 1.em;
			gap: 1.em;
			align-items: center;
			justify-content: space-between;
			background-color: var(--bt-color-surface-container-high);
		}

		footer::before {
			content: "";
			width: 3.em;
		}
	</style>

		}
}
