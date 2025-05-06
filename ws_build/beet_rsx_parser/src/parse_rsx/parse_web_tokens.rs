use crate::prelude::*;
use anyhow::Result;
use sweet::prelude::*;


/// Standard parsing of [`WebTokens`].
#[derive(Default)]
pub struct ParseWebTokens;

impl Pipeline<WebTokens, Result<WebTokens>> for ParseWebTokens {
	fn apply(self, tokens: WebTokens) -> Result<WebTokens> {
		tokens.xpipe(ApplyDefaultTemplateDirectives)?.xok()
	}
}
