#![allow(unused)]
use crate::prelude::*;
use beet::prelude::*;


#[template]
pub fn BeetHead() -> impl Bundle {
	rsx! {
		// <link rel="icon" href="/assets/branding/favicon.ico"/>
		<link rel="icon" href="/assets/branding/favicon-32x32.png"/>
	}
}
