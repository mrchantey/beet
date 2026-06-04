use crate::prelude::*;
use beet::prelude::*;

/// The site landing page, placed into the [`BeetDocumentShell`] `<main>`.
pub fn get() -> impl Scene {
	rsx! {
		<div {Classes::new(["hero"])}>
			<h1 {Classes::new([classes::TEXT_DISPLAY_MEDIUM])}>"Beet"</h1>
			<p {Classes::new([classes::TEXT_TITLE_LARGE])}>
				<b>"A malleable application framework"</b>
			</p>
			<div {Classes::new([classes::CARD_FILLED])}>
				<p>"🚧 Mind your step! 🚧"</p>
				<p>
					"Beet is under construction. If this project is of interest please come and say hi in the "
					<a href="https://discord.gg/DcURUQCXtx">"Beetmash Discord Server"</a>
					"."
				</p>
				<Link label="GitHub" href="https://github.com/mrchantey/beet" variant=ButtonVariant::Outlined/>
				<Link label="Blog" href=routes::blog::index() variant=ButtonVariant::Filled/>
			</div>
			<p {Classes::new([classes::TEXT_BODY_LARGE])}>
				"Beet is a framework for building user-modifiable applications, like Smalltalk or HyperCard. Everything from the CLI to client applications is a "
				<a href="https://bevy.org">"Bevy App"</a>
				", and all structure and behavior is written in Entity Component System architecture."
			</p>
		</div>
	}
}
