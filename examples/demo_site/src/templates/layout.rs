use beet::prelude::*;


#[template]
pub fn Layout() -> impl Bundle {
	rsx! {
		<!DOCTYPE html>
		<html>
			<head>
				<title>"Beet Demo Site"</title>
				<link rel="icon" href="https://fav.farm/🌱"/>
				<meta name="description" content="The last framework you'll ever need">
				<Style/>
			</head>
			<body>
				<div class="layout">
					<slot name="header" />
					<main>
						<slot />
					</main>
					<slot name="footer" />
				</div>
			</body>
		</html>
	}
}

#[template]
fn Style() -> impl Bundle {
	// css is much easier to write with the rsx_combinator macro
	// as many common css tokens like `1em` or `a:visited` are not valid rust tokens
	rsx_combinator! {r"
<style scope:global>
	.layout {
		display: flex;
		flex-direction: column;
		align-items: center;
	}
	main {
		padding: 1em;
		display: flex;
		flex-direction: column;
		max-width: 800px;
		min-height: 100vh;
		background: #222;
	}
	a {
		color: #90ee90;
	}
	a:visited {
		color: #3399ff;
	}
	body{
		margin: 0;
		font-size: 1.4em;
		font-family: system-ui, sans-serif;
		background: black;
		color: white;
		height: 100vh;
	}
</style>
	"}
}
