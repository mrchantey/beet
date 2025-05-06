use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;


/// Standard parsing of [`HtmlTokens`].
#[derive(Default)]
pub struct ParseHtmlTokens;

impl Pipeline<HtmlTokens, Result<HtmlTokens>> for ParseHtmlTokens {
	fn apply(self, tokens: HtmlTokens) -> Result<HtmlTokens> {
		tokens.xpipe(ApplyDefaultTemplateDirectives)?.xok()
	}
}
