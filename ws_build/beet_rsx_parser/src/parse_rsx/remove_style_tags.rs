use crate::prelude::*;
use beet_common::prelude::*;
use rapidhash::RapidHasher;
use std::convert::Infallible;
use std::hash::Hash;
use std::hash::Hasher;
use sweet::prelude::*;

/// Remove the contents of `<style>` tags and replace them with a
/// [`TemplateDirective::StylePlaceholder`].
pub struct RemoveStyleTags;

impl Pipeline<WebTokens, WebTokens> for RemoveStyleTags {
	fn apply(self, mut tokens: WebTokens) -> WebTokens {
		tokens
			.walk_web_tokens_mut::<Infallible>(|tokens| {
				match tokens {
					WebTokens::Element {
						component,
						children,
						self_closing,
					} if component.tag.value() == "style" => {
						let mut hasher = RapidHasher::default_const();
						children.hash(&mut hasher);
						let content_hash = hasher.finish();
						std::mem::take(children);

						*self_closing = true;
						component.meta.template_directives.push(
							TemplateDirective::StylePlaceholder {
								content_hash,
							},
						);
					}
					_ => {}
				}
				Ok(())
			})
			.ok();
		tokens
	}
}
