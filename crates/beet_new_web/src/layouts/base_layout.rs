use beet::prelude::*;

#[template]
pub fn BaseLayout() -> impl Bundle {
	rsx! {
		<!DOCTYPE html>
		<html lang="en">
			<head>
				<slot name="head" />
				<link rel="icon" href="https://fav.farm/🦄"/>
				<meta name="viewport" content="width=device-width, initial-scale=1" />
				<style scope:global src="./style.css"/>
			</head>
			<body>
				<slot/>
			</body>
		</html>
	}
}
