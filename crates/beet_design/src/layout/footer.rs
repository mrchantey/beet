use crate::prelude::*;
use beet_template::as_beet::*;
use chrono::Datelike;

#[template]
pub fn Footer() -> impl Bundle {
	let Brand { title, version, .. } = ReactiveApp::resource::<Brand>();
	let current_year = chrono::Utc::now().year();
	let footer_text = format!("&copy; {title} {current_year}");

	#[cfg(debug_assertions)]
	let version = format!("v{version} (dev)");
	#[cfg(not(debug_assertions))]
	let version = format!("v{version}");

	// <!-- <a href="/privacy-policy">Privacy</a> -->
	// <!-- <a href="/terms-of-service">Terms</a> -->

	rsx! {
		<footer id="page-footer">
			<span>{footer_text}</span>
			<span id="footer-version">{version}</span>
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
