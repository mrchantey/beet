use crate::prelude::*;
use beet::prelude::*;

/// The global document shell wrapping every route's body.
///
/// Composes the leaf [`Head`]/[`Header`]/[`Footer`] widgets into a plain
/// `<html>` document with the site sidebar and a `<slot name="main">` that the
/// [`document_shell`] middleware fills with the route's rendered content. The
/// `<head>` carries the web-only stylesheet, color-scheme seed, preflight reset
/// and favicon; the charcell renderer skips `<head>`, so the same shell renders
/// in the terminal.
///
/// The head and header are the site-specific [`BeetHead`]/[`BeetHeader`]
/// `#[scene(system)]` widgets (which emit their full structure directly), since
/// the library `Head`/`Header` slots do not yet compose with caller content.
/// [`Footer`] and [`BeetSidebar`] take no caller content, so the library widget
/// is used directly.
pub fn beet_document_shell() -> impl Scene {
	rsx! {
		<html lang="en">
			<BeetHead/>
			<body>
				<BeetHeader/>
				<div {Classes::new([classes::CONTAINER])}>
					<BeetSidebar/>
					<main {Classes::new(["site-main"])}>
						<slot name="main"/>
					</main>
				</div>
				<Footer/>
			</body>
		</html>
	}
}
