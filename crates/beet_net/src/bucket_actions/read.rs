use crate::prelude::*;
use beet_core::prelude::*;

const DEFAULT_MAX_LINES: usize = 2000;
const DEFAULT_MAX_BYTES: usize = 50 * 1024; // 50KB

/// Parameters for reading a blob from a bucket.
#[derive(Debug, Clone, Reflect, serde::Serialize, serde::Deserialize)]
pub struct ReadBlobParams {
	/// Path to the blob to read.
	pub path: RelPath,
	/// Line offset to start reading from (0-indexed).
	pub offset: Option<usize>,
	/// Maximum number of lines to return.
	pub limit: Option<usize>,
}

/// Read a blob from the nearest ancestor [`Bucket`].
///
/// For text content, output is truncated based on whichever limit is hit first:
/// - Line limit (default: 2000 lines)
/// - Byte limit (default: 50KB)
#[action]
#[derive(Component, Reflect)]
pub async fn ReadBlob(cx: ActionContext<ReadBlobParams>) -> Result<MediaBytes> {
	let bucket = cx
		.caller
		.with_state::<AncestorQuery<&Bucket>, _>(|entity, query| {
			query.get(entity).cloned()
		})
		.await?;

	let media = bucket.get_media(&cx.input.path).await?;

	if media.media_type().is_text() {
		truncate_text(media, cx.input.offset, cx.input.limit)
	} else {
		media.xok()
	}
}

/// Truncate text content based on line and byte limits.
fn truncate_text(
	media: MediaBytes,
	offset: Option<usize>,
	limit: Option<usize>,
) -> Result<MediaBytes> {
	let text = media.as_utf8()?;
	let max_lines = limit.unwrap_or(DEFAULT_MAX_LINES);
	let max_bytes = DEFAULT_MAX_BYTES;
	let start_line = offset.unwrap_or(0);

	let mut result = String::new();
	let mut line_count = 0;

	for line in text.lines().skip(start_line) {
		if line_count >= max_lines || result.len() + line.len() + 1 > max_bytes {
			break;
		}
		if !result.is_empty() {
			result.push('\n');
		}
		result.push_str(line);
		line_count += 1;
	}

	MediaBytes::new(media.media_type().clone(), result.into_bytes()).xok()
}


#[cfg(test)]
mod test {
	use super::*;

	/// Shared helper: create bucket with a text file at the given path.
	async fn bucket_with_text(path: &str, text: &str) -> Bucket {
		let bucket = Bucket::temp();
		bucket
			.insert(&RelPath::from(path), text.to_owned())
			.await
			.unwrap();
		bucket
	}

	#[beet_core::test]
	async fn reads_text_blob() {
		let bucket = bucket_with_text("hello.txt", "hello world").await;
		let media = bucket.get_media(&RelPath::from("hello.txt")).await.unwrap();
		let text = media.as_utf8().unwrap();
		text.xpect_eq("hello world");
	}

	#[test]
	fn truncates_by_line_limit() {
		let lines: Vec<&str> = (0..10).map(|_| "line").collect();
		let text = lines.join("
");
		let media = MediaBytes::new(
			MediaType::from_extension("txt"),
			text.into_bytes(),
		);
		let result = truncate_text(media, None, Some(3)).unwrap();
		let out = result.as_utf8().unwrap();
		out.lines().count().xpect_eq(3);
	}

	#[test]
	fn truncates_by_byte_limit() {
		// each line is well within the default byte limit individually,
		// but 2000+ lines of 30 chars each exceeds 50KB
		let line = "abcdefghijklmnopqrstuvwxyz0123";
		let lines: Vec<&str> = (0..2500).map(|_| line).collect();
		let text = lines.join("
");
		let media = MediaBytes::new(
			MediaType::from_extension("txt"),
			text.into_bytes(),
		);
		// use a high line limit so byte limit is hit first
		let result = truncate_text(media, None, Some(5000)).unwrap();
		let out = result.as_utf8().unwrap();
		// output should be under 50KB
		(out.len() <= 50 * 1024).xpect_true();
		// and fewer lines than the full 2500
		(out.lines().count() < 2500).xpect_true();
	}

	#[test]
	fn offset_skips_lines() {
		let text = "zero
one
two
three
four";
		let media = MediaBytes::new(
			MediaType::from_extension("txt"),
			text.as_bytes().to_vec(),
		);
		let result = truncate_text(media, Some(2), Some(2)).unwrap();
		let out = result.as_utf8().unwrap();
		out.xpect_eq("two
three");
	}
}
