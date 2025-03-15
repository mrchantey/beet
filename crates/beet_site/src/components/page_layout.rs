use beet::prelude::*;



#[derive(Node)]
pub struct PageLayout {
	pub title: String,
}


fn page_layout(props: PageLayout) -> RsxRoot {
	rsx! {
		<html>
		<head>
			<slot name="head"/>
		</head>
		<body>
		<h1>{props.title}</h1>
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
