use beet::prelude::*;




pub struct PageLayout {
	pub title: String,
}


impl Component for PageLayout {
	fn render(self) -> RsxRoot {
		rsx! {
			<html>
			<head>
				<slot name="head"/>
			</head>
			<body>
			<h1>{self.title}</h1>
				<nav>
					<a href="/">Home</a>
					<a href="/contributing">Contributing</a>
				</nav>
					<slot/>
				</body>
			</html>
			<style>
				nav{
					display: flex;
					flex-direction: column;
				}
			</style>
		}
	}
}
