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
				<style src="./layout.css"/>
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
