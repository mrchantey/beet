#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::{prelude::*, style::{CssBuilder, CssToken}};
use beet_core::prelude::*;

// ── Primary ───────────────────────────────────────────────────────────────────

token!(Primary0,   Color);



token!(Primary10,  Color);
token!(Primary20,  Color);

impl CssToken for Primary20 {
	fn declarations(
		builder: &CssBuilder,
		value: &TokenValue,
	) -> Result<Vec<(String, String)>> {
		Ok(vec![(
			builder.ident_to_css(&Self::token_key())?.as_css_key(),
			builder.token_value_to_css::<Color>(value)?,
		)])
	}
}


token!(Primary30,  Color);
token!(Primary40,  Color);
token!(Primary50,  Color);
token!(Primary60,  Color);
token!(Primary70,  Color);
token!(Primary80,  Color);
token!(Primary90,  Color);
token!(Primary95,  Color);
token!(Primary99,  Color);
token!(Primary100, Color);

// ── Secondary ─────────────────────────────────────────────────────────────────

token!(Secondary0,   Color);
token!(Secondary10,  Color);
token!(Secondary20,  Color);
token!(Secondary30,  Color);
token!(Secondary40,  Color);
token!(Secondary50,  Color);
token!(Secondary60,  Color);
token!(Secondary70,  Color);
token!(Secondary80,  Color);
token!(Secondary90,  Color);
token!(Secondary95,  Color);
token!(Secondary99,  Color);
token!(Secondary100, Color);

// ── Tertiary ──────────────────────────────────────────────────────────────────

token!(Tertiary0,   Color);
token!(Tertiary10,  Color);
token!(Tertiary20,  Color);
token!(Tertiary30,  Color);
token!(Tertiary40,  Color);
token!(Tertiary50,  Color);
token!(Tertiary60,  Color);
token!(Tertiary70,  Color);
token!(Tertiary80,  Color);
token!(Tertiary90,  Color);
token!(Tertiary95,  Color);
token!(Tertiary99,  Color);
token!(Tertiary100, Color);

// ── Neutral ───────────────────────────────────────────────────────────────────

token!(Neutral0,   Color);
token!(Neutral10,  Color);
token!(Neutral20,  Color);
token!(Neutral30,  Color);
token!(Neutral40,  Color);
token!(Neutral50,  Color);
token!(Neutral60,  Color);
token!(Neutral70,  Color);
token!(Neutral80,  Color);
token!(Neutral90,  Color);
token!(Neutral95,  Color);
token!(Neutral99,  Color);
token!(Neutral100, Color);

// ── NeutralVariant ────────────────────────────────────────────────────────────

token!(NeutralVariant0,   Color);
token!(NeutralVariant10,  Color);
token!(NeutralVariant20,  Color);
token!(NeutralVariant30,  Color);
token!(NeutralVariant40,  Color);
token!(NeutralVariant50,  Color);
token!(NeutralVariant60,  Color);
token!(NeutralVariant70,  Color);
token!(NeutralVariant80,  Color);
token!(NeutralVariant90,  Color);
token!(NeutralVariant95,  Color);
token!(NeutralVariant99,  Color);
token!(NeutralVariant100, Color);

// ── Error ─────────────────────────────────────────────────────────────────────

token!(Error0,   Color);
token!(Error10,  Color);
token!(Error20,  Color);
token!(Error30,  Color);
token!(Error40,  Color);
token!(Error50,  Color);
token!(Error60,  Color);
token!(Error70,  Color);
token!(Error80,  Color);
token!(Error90,  Color);
token!(Error95,  Color);
token!(Error99,  Color);
token!(Error100, Color);
