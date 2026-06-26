//! Merging the per-page PDFs a headless browser prints into one document, for
//! the [`ExportPdf`](super::ExportPdf) command.

use beet::prelude::*;
use lopdf::Document;
use lopdf::Object;
use lopdf::ObjectId;
use lopdf::dictionary;
use std::collections::BTreeMap;

/// Merge `pdfs` into a single document, preserving their order as the page
/// sequence. Each input is a self-contained PDF (one chromium print), so the
/// merge renumbers every object to avoid id collisions, reparents all pages onto
/// one `Pages` root, and rebuilds the catalog.
pub fn merge(pdfs: Vec<Vec<u8>>) -> Result<Vec<u8>> {
	if pdfs.is_empty() {
		bevybail!("no pdfs to merge");
	}
	// a single page needs no merge, so return it untouched.
	if pdfs.len() == 1 {
		return Ok(pdfs.into_iter().next().unwrap());
	}

	// load every doc, renumbering ids by a running max so none collide, and
	// collect each doc's page objects (in order) plus all its objects.
	let mut max_id = 1;
	let mut pages = BTreeMap::new();
	let mut objects = BTreeMap::new();
	for bytes in &pdfs {
		let mut doc = Document::load_mem(bytes)?;
		doc.renumber_objects_with(max_id);
		max_id = doc.max_id + 1;
		pages.extend(
			doc.get_pages()
				.into_values()
				.map(|id| (id, doc.get_object(id).unwrap().to_owned()))
				.collect::<BTreeMap<ObjectId, Object>>(),
		);
		objects.extend(doc.objects);
	}

	// carry over every object except the structural ones (`Catalog`/`Pages` are
	// rebuilt, `Page`s are reparented below, outlines are dropped), keeping the
	// first catalog + pages id to reuse.
	let mut document = Document::with_version("1.5");
	let mut catalog_id = None;
	let mut pages_id = None;
	for (id, object) in &objects {
		match object_type(object) {
			Some(b"Catalog") => catalog_id = catalog_id.or(Some(*id)),
			Some(b"Pages") => pages_id = pages_id.or(Some(*id)),
			Some(b"Page") | Some(b"Outlines") | Some(b"Outline") => {}
			_ => {
				document.objects.insert(*id, object.clone());
			}
		}
	}
	let catalog_id =
		catalog_id.ok_or_else(|| bevyhow!("merged pdf has no Catalog"))?;
	let pages_id =
		pages_id.ok_or_else(|| bevyhow!("merged pdf has no Pages root"))?;

	// reparent every collected page onto the single Pages root.
	for (id, object) in &pages {
		let mut dict = object.as_dict()?.clone();
		dict.set("Parent", pages_id);
		document.objects.insert(*id, Object::Dictionary(dict));
	}

	// the rebuilt Pages root references every page, in collected (route) order.
	let kids = pages
		.keys()
		.map(|id| Object::Reference(*id))
		.collect::<Vec<_>>();
	document.objects.insert(
		pages_id,
		Object::Dictionary(dictionary! {
			"Type" => "Pages",
			"Count" => pages.len() as u32,
			"Kids" => kids,
		}),
	);
	document.objects.insert(
		catalog_id,
		Object::Dictionary(dictionary! {
			"Type" => "Catalog",
			"Pages" => pages_id,
		}),
	);
	document.trailer.set("Root", catalog_id);

	document.max_id = document.objects.len() as u32;
	document.renumber_objects();
	document.compress();

	let mut buffer = Vec::new();
	document.save_to(&mut buffer)?;
	Ok(buffer)
}

/// The `/Type` name of a PDF object, read defensively so version churn in
/// lopdf's `type_name` signature never breaks the merge.
fn object_type(object: &Object) -> Option<&[u8]> {
	object
		.as_dict()
		.ok()
		.and_then(|dict| dict.get(b"Type").ok())
		.and_then(|ty| ty.as_name().ok())
}

#[cfg(test)]
mod test {
	use super::*;
	use lopdf::Document;

	/// A minimal one-page PDF carrying `text`, so the merge has real input.
	fn one_page_pdf(text: &str) -> Vec<u8> {
		let mut doc = Document::with_version("1.5");
		let pages_id = doc.new_object_id();
		let content = lopdf::content::Content {
			operations: vec![lopdf::content::Operation::new(
				"Tj",
				vec![Object::string_literal(text)],
			)],
		};
		let content_id =
			doc.add_object(lopdf::Stream::new(dictionary! {}, content.encode().unwrap()));
		let page_id = doc.add_object(dictionary! {
			"Type" => "Page",
			"Parent" => pages_id,
			"Contents" => content_id,
			"MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
		});
		doc.objects.insert(
			pages_id,
			Object::Dictionary(dictionary! {
				"Type" => "Pages",
				"Count" => 1,
				"Kids" => vec![page_id.into()],
			}),
		);
		let catalog_id = doc.add_object(dictionary! {
			"Type" => "Catalog",
			"Pages" => pages_id,
		});
		doc.trailer.set("Root", catalog_id);
		let mut buffer = Vec::new();
		doc.save_to(&mut buffer).unwrap();
		buffer
	}

	#[beet::test]
	fn merges_in_sequence() {
		let merged =
			merge(vec![one_page_pdf("a"), one_page_pdf("b"), one_page_pdf("c")])
				.unwrap();
		// the merged bytes re-parse, and every input page survives in one document
		Document::load_mem(&merged)
			.unwrap()
			.get_pages()
			.len()
			.xpect_eq(3);
	}

	#[beet::test]
	fn single_pdf_passes_through() {
		let pdf = one_page_pdf("solo");
		merge(vec![pdf.clone()]).unwrap().xpect_eq(pdf);
	}
}
