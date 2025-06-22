use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsMeta {
	#[serde(default)]
	pub title: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub draft: bool,
	#[serde(default)]
	pub sidebar: SidebarInfo,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidebarInfo {
	#[serde(default)]
	pub label: Option<String>,
	#[serde(default)]
	pub order: Option<u32>,
}
