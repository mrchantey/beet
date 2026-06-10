use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;





///
/// ## Errors
/// Errors if sending the request results in a non-200 status code
pub async fn media_exchange(
	render_target: &mut EntityWorldMut<'_>,
	mut request: Request,
) -> Result<Response> {
	let _outer_accept = request
		.headers
		.get::<header::Accept>()
		.map(|headers| headers.ok())
		.flatten()
		.unwrap_or_default();
	// clear the inner request accept,
	// we want one that we can parse
	request.headers.remove::<header::Accept>();

	// TODO prefer postcard
	// TODO prefer json
	// prefer markdown, far less to parse
	#[cfg(feature = "markdown_parser")]
	let request = request.with_accept(MediaType::Markdown);
	#[cfg(feature = "bsx")]
	let request = request.with_accept(MediaType::Html);
	let res = request.send().await?.into_result().await?;
	let content_type = res
		.headers
		.get::<header::ContentType>()
		.map(|headers| headers.ok())
		.flatten()
		.unwrap_or(MediaType::Text);

	let body = res.body.into_bytes().await?;

	parse_body_to_render_target(render_target, content_type, &body.to_vec())?;

	// render_to_media_type(render_target, outer_accept).xok()
	//
	Response::ok().xok()
}


fn parse_body_to_render_target(
	render_target: &mut EntityWorldMut,
	media_type: MediaType,
	bytes: &[u8],
) -> Result {
	let mb = MediaBytes::new(media_type.clone(), bytes);
	let cx = ParseContext::new(render_target, &mb);
	match media_type {
		MediaType::Text => PlainTextParser::default()
			.parse(cx)
			.map_err(|err| bevyhow!("{err}")),
		#[cfg(feature = "bsx")]
		MediaType::Html => BsxParser::html()
			.parse(cx)
			.map_err(|err| bevyhow!("{err}")),
		#[cfg(feature = "markdown_parser")]
		MediaType::Markdown => MarkdownParser::default()
			.parse(cx)
			.map_err(|err| bevyhow!("{err}")),
		MediaType::Json => todo!("beet_ui json parser, TemplateLoader?"),
		MediaType::Bytes | MediaType::Postcard => {
			todo!("beet_ui postcard parser, TemplateLoader?")
		}
		unsupported => {
			bevybail!("Unsupported Content-Type: {unsupported}")
		}
	}
}


// fn render_to_media_type(
// 	server: &mut EntityWorldMut,
// 	render_target: &mut EntityWorldMut,
// 	accept: Vec<MediaType>,
// ) -> Response {
// 	// fn accept_text

// 	// let accept_map = vec![(MediaType::Text, || {}), (MediaType::Text, || {})];



// 	Response::ok()
// }
