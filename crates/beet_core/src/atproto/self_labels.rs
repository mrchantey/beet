use crate::prelude::*;

/// `com.atproto.label.defs#selfLabels`: the labels a record's author applies
/// to it, effectively content warnings, ie `graphic-media`.
///
/// Written as the member of a union, so it carries its `$type`:
///
/// ```json
/// {"$type":"com.atproto.label.defs#selfLabels","values":[{"val":"graphic-media"}]}
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
	feature = "serde",
	serde(tag = "$type", rename = "com.atproto.label.defs#selfLabels")
)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct SelfLabels {
	/// The labels, at most ten.
	pub values: Vec<SelfLabel>,
}

impl SelfLabels {
	/// `labels` as self labels, `None` when there are none, since an empty set
	/// says nothing a record should carry.
	pub fn from_values(
		labels: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Option<Self> {
		let values = labels
			.into_iter()
			.map(|val| SelfLabel { val: val.into() })
			.collect::<Vec<_>>();
		(!values.is_empty()).then_some(Self { values })
	}
}

/// `com.atproto.label.defs#selfLabel`: one label value, at most 128 bytes.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct SelfLabel {
	/// The label, ie `graphic-media`.
	pub val: SmolStr,
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn round_trips_the_union_member() {
		let json = r#"{"$type":"com.atproto.label.defs#selfLabels","values":[{"val":"graphic-media"}]}"#;
		let labels = SelfLabels::from_values(["graphic-media"]).unwrap();
		serde_json::to_string(&labels).unwrap().xpect_eq(json);
		serde_json::from_str::<SelfLabels>(json)
			.unwrap()
			.xpect_eq(labels);
		SelfLabels::from_values(Vec::<SmolStr>::new()).xpect_none();
	}
}
