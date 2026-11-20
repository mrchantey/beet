//! A file's core properties, `docProps/core.xml`, read into and written from
//! the root's [`DocMeta`], by the mapping the [module docs](super) tabulate.
use super::xml_part::XmlPart;
use crate::prelude::*;
use beet_core::prelude::*;
use heck::ToKebabCase;

impl DocMeta {
	/// The path a package's core properties are written at.
	pub(crate) const CORE_PROPERTIES_PATH: &str = "docProps/core.xml";

	/// The [`DocMeta`] `package`'s core properties declare, empty when it
	/// relates none.
	pub(crate) fn from_package(package: &OpcPackage) -> Result<Self> {
		match package
			.related(None, OpcRelationship::CORE_PROPERTIES)
			.and_then(|path| package.part(path))
		{
			Some(bytes) => Self::from_core_properties(bytes),
			None => Self::default().xok(),
		}
	}

	/// Reads a core properties part: the title, description, authors split
	/// on `;`, tags split on `,` and `;` each lowercase kebab, and the days
	/// the file was created and modified, every other property dropped.
	pub(crate) fn from_core_properties(bytes: &[u8]) -> Result<Self> {
		let nodes = OpcPackage::read_xml(bytes)?;
		let Some(root) = BsxNode::document_element(&nodes) else {
			return Self::default().xok();
		};
		let field = |local: &str| {
			root.child(local)
				.map(|element| element.text().trim().to_string())
				.filter(|text| !text.is_empty())
		};
		let split = |text: Option<String>, separators: &[char]| {
			text.map(|text| {
				text.split(separators)
					.map(str::trim)
					.filter(|item| !item.is_empty())
					.map(String::from)
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
		};
		// the day a W3CDTF timestamp falls on, ie `2024-03-01` from
		// `2024-03-01T09:30:00Z`
		let day = |local: &str| {
			field(local).and_then(|timestamp| {
				timestamp.get(..10).and_then(|day| Date::parse(day).ok())
			})
		};
		Self {
			title: field("title"),
			description: field("description"),
			authors: split(field("creator"), &[';'])
				.into_iter()
				.map(SmolStr::from)
				.collect(),
			tags: split(field("keywords"), &[',', ';'])
				.into_iter()
				.map(|tag| SmolStr::from(tag.to_kebab_case()))
				.collect(),
			created: day("created"),
			updated: day("modified"),
			..default()
		}
		.xok()
	}

	/// Writes the core properties into `package` and relates them from it,
	/// unless this declares none of them.
	pub(crate) fn write_package(&self, package: &mut OpcPackage) {
		let Some(bytes) = self.core_properties() else {
			return;
		};
		package.insert(
			Self::CORE_PROPERTIES_PATH,
			OpcPackage::CORE_PROPERTIES_TYPE,
			bytes,
		);
		package.relate(
			None,
			OpcRelationship::CORE_PROPERTIES,
			Self::CORE_PROPERTIES_PATH,
		);
	}

	/// The core properties part, `None` when this declares none of them:
	/// authors joined by `; `, tags by `, `, a day as its midnight UTC.
	pub(crate) fn core_properties(&self) -> Option<Vec<u8>> {
		let authors = self.authors.join("; ");
		let tags = self.tags.join(", ");
		let day =
			|day: &Option<Date>| day.map(|day| format!("{day}T00:00:00Z"));
		let (created, updated) = (day(&self.created), day(&self.updated));
		let fields = [
			("dc:title", self.title.as_deref()),
			("dc:description", self.description.as_deref()),
			(
				"dc:creator",
				(!authors.is_empty()).then_some(authors.as_str()),
			),
			("cp:keywords", (!tags.is_empty()).then_some(tags.as_str())),
		];
		if fields.iter().all(|(_, value)| value.is_none())
			&& created.is_none()
			&& updated.is_none()
		{
			return None;
		}
		let mut xml = XmlPart::new();
		xml.open("cp:coreProperties", &[
			("xmlns:cp", OoxmlNamespace::CORE_PROPERTIES),
			("xmlns:dc", OoxmlNamespace::DUBLIN_CORE),
			("xmlns:dcterms", OoxmlNamespace::DUBLIN_CORE_TERMS),
			("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"),
		]);
		for (name, value) in fields {
			if let Some(value) = value {
				xml.text_element(name, &[], value);
			}
		}
		for (name, value) in
			[("dcterms:created", created), ("dcterms:modified", updated)]
		{
			if let Some(value) = value {
				xml.text_element(
					name,
					&[("xsi:type", "dcterms:W3CDTF")],
					&value,
				);
			}
		}
		xml.close("cp:coreProperties");
		xml.into_bytes().xsome()
	}

	/// This as the declarations a root scan reads, named as `component`,
	/// every field it maps from the core properties that is set.
	pub(crate) fn declarations(&self, component: &str) -> RootDeclarations {
		let text = |text: &str| DataLiteral::Scalar(Value::Str(text.into()));
		let list = |items: &[SmolStr]| {
			(!items.is_empty()).then(|| {
				DataLiteral::List(items.iter().map(|item| text(item)).collect())
			})
		};
		let fields = [
			("title", self.title.as_deref().map(text)),
			("description", self.description.as_deref().map(text)),
			("authors", list(&self.authors)),
			("tags", list(&self.tags)),
			("created", self.created.map(|day| text(&day.to_string()))),
			("updated", self.updated.map(|day| text(&day.to_string()))),
		]
		.into_iter()
		.filter_map(|(key, value)| Some((SmolStr::new(key), value?)))
		.collect::<Vec<_>>();
		match fields.is_empty() {
			true => RootDeclarations::default(),
			false => RootDeclarations(vec![NamedLiteral {
				name: component.into(),
				fields: NamedFields::Struct(fields),
			}]),
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Every mapped field reads and writes back: authors split and joined,
	/// tags kebab cased, days as their midnight, and a dropped property
	/// ignored.
	#[beet_core::test]
	fn maps_every_field_both_ways() {
		let meta = DocMeta::from_core_properties(
			format!(
				"<cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\" xmlns:dcterms=\"{}\">\
				 <dc:title>Plan</dc:title><dc:description>The year ahead</dc:description>\
				 <dc:creator>Ada Lovelace; Grace Hopper</dc:creator>\
				 <cp:keywords>Machine Learning, budget;Q3</cp:keywords>\
				 <dc:subject>dropped</dc:subject><cp:revision>3</cp:revision>\
				 <dcterms:created>2024-03-01T09:30:00Z</dcterms:created>\
				 <dcterms:modified>2024-04-02T10:00:00Z</dcterms:modified>\
				 </cp:coreProperties>",
				OoxmlNamespace::CORE_PROPERTIES,
				OoxmlNamespace::DUBLIN_CORE,
				OoxmlNamespace::DUBLIN_CORE_TERMS,
			)
			.as_bytes(),
		)
		.unwrap();
		meta.clone().xpect_eq(DocMeta {
			title: Some("Plan".into()),
			description: Some("The year ahead".into()),
			authors: vec!["Ada Lovelace".into(), "Grace Hopper".into()],
			tags: vec!["machine-learning".into(), "budget".into(), "q3".into()],
			created: Some(Date::parse("2024-03-01").unwrap()),
			updated: Some(Date::parse("2024-04-02").unwrap()),
			..default()
		});
		let written =
			String::from_utf8(meta.core_properties().unwrap()).unwrap();
		written
			.clone()
			.xpect_contains("<dc:creator>Ada Lovelace; Grace Hopper</dc:creator>")
			.xpect_contains("<cp:keywords>machine-learning, budget, q3</cp:keywords>")
			.xpect_contains(
				"<dcterms:created xsi:type=\"dcterms:W3CDTF\">2024-03-01T00:00:00Z</dcterms:created>",
			)
			.xnot()
			.xpect_contains("subject");
		DocMeta::from_core_properties(written.as_bytes())
			.unwrap()
			.xpect_eq(meta);
		DocMeta::default().core_properties().xpect_none();
	}
}
