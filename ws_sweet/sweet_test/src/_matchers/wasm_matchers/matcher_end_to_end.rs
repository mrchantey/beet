// use anyhow::Result;
// use sweet_web::prelude::*;
// use web_sys::Document;
// use web_sys::HtmlIFrameElement;

// const NO_IFRAME: &str = r#"
// iframe only available in e2e tests:

// it e2e "works" {
//   let page = visit(my_url)?.await;
// }
// "#;


// pub async fn set_proxy_origin(url: &str) -> Result<()> {
// 	let set_url = format!("/_proxy_set_/{url}");
// 	fetch(&set_url).await?;
// 	Ok(())
// }

// pub async fn visit(url: &str) -> Result<HtmlIFrameElement> {
// 	set_proxy_origin(url).await?;
// 	match Document::x_query_selector::<HtmlIFrameElement>("iframe") {
// 		None => Err(anyhow::anyhow!(NO_IFRAME)),
// 		Some(iframe) => {
// 			let url = format!("/_proxy_/{url}");
// 			iframe.x_set_source_async(&url).await;
// 			Ok(iframe)
// 		}
// 	}
// }
