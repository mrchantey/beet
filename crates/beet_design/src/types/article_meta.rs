use crate::prelude::SidebarInfo;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// General metadata common for blog posts, docs, etc.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Component,
)]
pub struct ArticleMeta {
	#[serde(default)]
	pub title: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub draft: bool,
	#[serde(default)]
	pub sidebar: SidebarInfo,
}
