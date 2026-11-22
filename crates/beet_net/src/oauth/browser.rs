use beet_core::prelude::*;

/// Sending the operator to an authorization page: always a notice through
/// the [`OperatorPrompt`] (the url printed, so a terminal makes it a link),
/// and on a desktop the page opened in the default browser as well.
pub struct OAuthBrowser;

impl OAuthBrowser {
	/// Show `url` to the operator as `subject`, ie `an Identity Center sign
	/// in`, opening it where the platform can.
	pub async fn open(subject: &str, url: &str) -> Result {
		OperatorPrompt::ask(
			OperatorAsk::notice(
				subject,
				format!("open this url in a browser to sign in:\n\n  {url}\n"),
			)
			.with_instead(
				"run the login in a terminal, or pass `--no-browser` from a \
				machine without one",
			),
		)
		.await?;
		Self::launch(url);
		OK
	}

	/// The platform's opener, best effort: a machine without one has the
	/// printed url.
	fn launch(url: &str) {
		cfg_if! {
			if #[cfg(all(feature = "fs", not(target_arch = "wasm32")))] {
				let opener = match cfg!(target_os = "macos") {
					true => "open",
					false => "xdg-open",
				};
				if let Err(err) = ChildProcess::new(opener).with_args([url]).spawn() {
					debug!("no browser opened ({err}): the url above is the way in");
				}
			} else {
				let _ = url;
			}
		}
	}
}
