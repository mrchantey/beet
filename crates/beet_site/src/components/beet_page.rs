use beet::prelude::*;



#[derive(Node)]
pub struct BeetPage {}


fn beet_page(_: BeetPage) -> RsxRoot {
	set_context(Brand {
		title: "Beet".into(),
		description: "A Rust web framework".into(),
		site_url: "https://beetrsx.dev".into(),
	});

	let brand = get_context::<Brand>();

	rsx! {
		<html>
		<head>
			<slot name="head"/>
		</head>
		<body>
		<h1>{brand.title}</h1>
			<nav>
				<a href="/">Home</a>
				<a href="/contributing">Contributing</a>
				<a href="/contributing">Foobarbsazz</a>
			</nav>
				<slot/>
				<style>
					h1{
						padding-top: 20px;
					}
					nav{
						display: flex;
						flex-direction: column;
					}
				</style>
				<style scope:global>
					body{
						margin:0;
						background-color: black;
						color:white;
					}
				</style>
			</body>
		</html>
	}
}
