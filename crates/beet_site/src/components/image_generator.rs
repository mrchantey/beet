// use crate::prelude::*;
use beet::prelude::*;



#[template]
#[derive(Reflect)]
pub fn ImageGenerator() -> impl Bundle {
	let (image_url, set_image_url) = signal(None);
	let (prompt, set_prompt) = signal("TODO images".to_string());
	let (response, set_response) = signal("".to_string());

	let on_submit = move || {
		async_ext::spawn_local(async move {
			let body = prompt().into_content_vec();
			let content = Request::post("/generate_image")
				.with_json_body(&body)
				.unwrap()
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.unwrap()
				.json::<ContentVec>()
				.await
				.unwrap();
			if let Some(text) = content.first_text() {
				set_response(text.to_string());
			}
			if let Some(file) = content.first_file() {
				set_image_url(Some(file.into_url()));
			}
		})
		.detach();
	};

	rsx! {
		<h1>Agent Images</h1>
		<TextArea value=prompt onchange=move |e|set_prompt(e.value())/>
		<Button onclick=move |_|on_submit()>Submit</Button>
		<br/>
		<p>{response}</p>
		<br/>
		<img src={image_url}/>
	}
}
