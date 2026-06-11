use beet::prelude::*;

/// A remote PNG rendered at a variety of sizes.
///
/// On the web these are ordinary `<img>` elements; on the terminal each
/// renders through the kitty graphics protocol (kitty/ghostty/WezTerm),
/// fetched in the background and scaled to its cell box. Terminals without
/// graphics support show the `[image]: alt` fallback instead.
///
/// The wiki thumbnailer serves webp by default; `format=png` requests the PNG
/// the kitty protocol accepts.
const SRC: &str = "https://static.wikia.nocookie.net/teen-titans-go/images/5/52/Vlcsnap-2014-10-10-07h19m55s124.png/revision/latest?cb=20241013160009&format=png";

pub fn get() -> impl Bundle {
	rsx! {
		<article>
			<h1>"Images"</h1>
			<h2>"Intrinsic size"</h2>
			<p>"No constraints: the box derives from the image's pixel size, clamped to the content width."</p>
			<img src=SRC alt="teen titans, intrinsic size"/>
			<h2>"Explicit width"</h2>
			<p>"A fixed width; the height follows the aspect ratio like a CSS replaced element."</p>
			<img src=SRC alt="teen titans, 20rem wide" {inline_class![(common_props::Width, Length::Rem(20.))]}/>
			<h2>"Explicit height"</h2>
			<p>"A fixed height; the width follows the aspect ratio."</p>
			<img src=SRC alt="teen titans, 6rem tall" {inline_class![(common_props::Height, Length::Rem(6.))]}/>
			<h2>"Stretched"</h2>
			<p>"Both dimensions fixed, distorting the aspect."</p>
			<img src=SRC alt="teen titans, stretched" {inline_class![
				(common_props::Width, Length::Rem(40.)),
				(common_props::Height, Length::Rem(4.)),
			]}/>
			<h2>"Viewport relative"</h2>
			<p>"Half the viewport width, aspect preserved."</p>
			<img src=SRC alt="teen titans, half viewport" {inline_class![(common_props::Width, Length::ViewportWidth(50.))]}/>
		</article>
	}
}
