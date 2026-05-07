#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::{prelude::*, style::CssTokenMap};
use beet_core::prelude::*;


pub fn token_map() -> CssTokenMap {
	CssTokenMap::default()
		.insert(Primary0)
		.insert(Primary10)
		.insert(Primary20)
		.insert(Primary30)
		.insert(Primary40)
		.insert(Primary50)
		.insert(Primary60)
		.insert(Primary70)
		.insert(Primary80)
		.insert(Primary90)
		.insert(Primary95)
		.insert(Primary99)
		.insert(Primary100)

		.insert(Secondary0)
		.insert(Secondary10)
		.insert(Secondary20)
		.insert(Secondary30)
		.insert(Secondary40)
		.insert(Secondary50)
		.insert(Secondary60)
		.insert(Secondary70)
		.insert(Secondary80)
		.insert(Secondary90)
		.insert(Secondary95)
		.insert(Secondary99)
		.insert(Secondary100)

	  .insert(Tertiary0)
	  .insert(Tertiary10)
	  .insert(Tertiary20)
	  .insert(Tertiary30)
	  .insert(Tertiary40)
	  .insert(Tertiary50)
	  .insert(Tertiary60)
	  .insert(Tertiary70)
	  .insert(Tertiary80)
	  .insert(Tertiary90)
	  .insert(Tertiary95)
	  .insert(Tertiary99)
	  .insert(Tertiary100)

	  .insert(Neutral0)
	  .insert(Neutral10)
	  .insert(Neutral20)
	  .insert(Neutral30)
	  .insert(Neutral40)
	  .insert(Neutral50)
	  .insert(Neutral60)
	  .insert(Neutral70)
	  .insert(Neutral80)
	  .insert(Neutral90)
	  .insert(Neutral95)
	  .insert(Neutral99)
	  .insert(Neutral100)

		.insert(NeutralVariant0)
		.insert(NeutralVariant10)
		.insert(NeutralVariant20)
		.insert(NeutralVariant30)
		.insert(NeutralVariant40)
		.insert(NeutralVariant50)
		.insert(NeutralVariant60)
		.insert(NeutralVariant70)
		.insert(NeutralVariant80)
		.insert(NeutralVariant90)
		.insert(NeutralVariant95)
		.insert(NeutralVariant99)
		.insert(NeutralVariant100)

		.insert(Error0)
		.insert(Error10)
		.insert(Error20)
		.insert(Error30)
		.insert(Error40)
		.insert(Error50)
		.insert(Error60)
		.insert(Error70)
		.insert(Error80)
		.insert(Error90)
		.insert(Error95)
		.insert(Error99)
		.insert(Error100)
}

// ── Primary ───────────────────────────────────────────────────────────────────

css_variable!(Primary0,   Color);



css_variable!(Primary10,  Color);
css_variable!(Primary20,  Color);

css_variable!(Primary30,  Color);
css_variable!(Primary40,  Color);
css_variable!(Primary50,  Color);
css_variable!(Primary60,  Color);
css_variable!(Primary70,  Color);
css_variable!(Primary80,  Color);
css_variable!(Primary90,  Color);
css_variable!(Primary95,  Color);
css_variable!(Primary99,  Color);
css_variable!(Primary100, Color);

// ── Secondary ─────────────────────────────────────────────────────────────────

css_variable!(Secondary0,   Color);
css_variable!(Secondary10,  Color);
css_variable!(Secondary20,  Color);
css_variable!(Secondary30,  Color);
css_variable!(Secondary40,  Color);
css_variable!(Secondary50,  Color);
css_variable!(Secondary60,  Color);
css_variable!(Secondary70,  Color);
css_variable!(Secondary80,  Color);
css_variable!(Secondary90,  Color);
css_variable!(Secondary95,  Color);
css_variable!(Secondary99,  Color);
css_variable!(Secondary100, Color);

// ── Tertiary ──────────────────────────────────────────────────────────────────

css_variable!(Tertiary0,   Color);
css_variable!(Tertiary10,  Color);
css_variable!(Tertiary20,  Color);
css_variable!(Tertiary30,  Color);
css_variable!(Tertiary40,  Color);
css_variable!(Tertiary50,  Color);
css_variable!(Tertiary60,  Color);
css_variable!(Tertiary70,  Color);
css_variable!(Tertiary80,  Color);
css_variable!(Tertiary90,  Color);
css_variable!(Tertiary95,  Color);
css_variable!(Tertiary99,  Color);
css_variable!(Tertiary100, Color);

// ── Neutral ───────────────────────────────────────────────────────────────────

css_variable!(Neutral0,   Color);
css_variable!(Neutral10,  Color);
css_variable!(Neutral20,  Color);
css_variable!(Neutral30,  Color);
css_variable!(Neutral40,  Color);
css_variable!(Neutral50,  Color);
css_variable!(Neutral60,  Color);
css_variable!(Neutral70,  Color);
css_variable!(Neutral80,  Color);
css_variable!(Neutral90,  Color);
css_variable!(Neutral95,  Color);
css_variable!(Neutral99,  Color);
css_variable!(Neutral100, Color);

// ── NeutralVariant ────────────────────────────────────────────────────────────

css_variable!(NeutralVariant0,   Color);
css_variable!(NeutralVariant10,  Color);
css_variable!(NeutralVariant20,  Color);
css_variable!(NeutralVariant30,  Color);
css_variable!(NeutralVariant40,  Color);
css_variable!(NeutralVariant50,  Color);
css_variable!(NeutralVariant60,  Color);
css_variable!(NeutralVariant70,  Color);
css_variable!(NeutralVariant80,  Color);
css_variable!(NeutralVariant90,  Color);
css_variable!(NeutralVariant95,  Color);
css_variable!(NeutralVariant99,  Color);
css_variable!(NeutralVariant100, Color);

// ── Error ─────────────────────────────────────────────────────────────────────

css_variable!(Error0,   Color);
css_variable!(Error10,  Color);
css_variable!(Error20,  Color);
css_variable!(Error30,  Color);
css_variable!(Error40,  Color);
css_variable!(Error50,  Color);
css_variable!(Error60,  Color);
css_variable!(Error70,  Color);
css_variable!(Error80,  Color);
css_variable!(Error90,  Color);
css_variable!(Error95,  Color);
css_variable!(Error99,  Color);
css_variable!(Error100, Color);
