// use super::RsxLocation;
// use crate::html::HtmlNode;

use super::RsxLocation;
use super::RsxTemplateNode;
use std::collections::HashMap;





/// Save and load rsx templates
#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RsxTemplateIo {
	pub nodes: HashMap<RsxLocation, RsxTemplateNode>,
}

impl RsxTemplateIo {}
