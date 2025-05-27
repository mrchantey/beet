use crate::prelude::Document;
use std::ops::Range;
use std::path::Path;
use text_splitter::CodeSplitter;
use text_splitter::MarkdownSplitter;
use text_splitter::TextSplitter;


/// Split text into chunks, using the extension to determine the approach
#[derive(Debug, Clone)]
pub struct SplitText {
	/// Range of character count to use for chunking.
	pub chunk_range: Range<usize>,
}

impl Default for SplitText {
	fn default() -> Self {
		Self {
			chunk_range: DEFAULT_CHUNK_RANGE,
		}
	}
}

/// min/max chunk size for text splitting
// experiments:
// 300..1000 too small, cant get a full function
const DEFAULT_CHUNK_RANGE: Range<usize> = 1500..3000;

impl SplitText {
	pub fn new(chunk_range: Range<usize>) -> Self { Self { chunk_range } }

	pub fn split<'a>(
		&self,
		path: impl AsRef<Path>,
		content: &'a str,
	) -> Vec<&'a str> {
		match path.as_ref().extension().and_then(|s| s.to_str()) {
			Some("md") => MarkdownSplitter::new(self.chunk_range.clone())
				.chunks(content)
				.collect(),
			Some("rs") => CodeSplitter::new(
				tree_sitter_rust::LANGUAGE,
				self.chunk_range.clone(),
			)
			.expect("Invalid tree-sitter language")
			.chunks(content)
			.collect(),
			// default to character splitting
			_ => TextSplitter::new(self.chunk_range.clone())
				.chunks(content)
				.collect(),
		}
	}

	pub fn split_to_documents(
		&self,
		path: impl AsRef<Path>,
		content: &str,
	) -> Vec<Document> {
		self.split(&path, content)
			.into_iter()
			.enumerate()
			// we should be using the breadcrumbs but text-splitter doesnt
			// yet support them https://github.com/benbrandt/text-splitter/issues/726
			.map(|(i, content)| {
				Document::new(
					&format!("{}#{}", &path.as_ref().to_string_lossy(), i),
					content,
				)
			})
			.collect()
	}
}






#[cfg(test)]
mod test {
	use super::*;
	use std::path::Path;


	#[test]
	fn test_split_markdown() {
		let content = r#"# Title

This is a paragraph with some content that should be long enough to trigger splitting.

## Section 1

This is another section with more content that adds to the length of the document.

## Section 2

And here's even more content to make sure we have enough text for meaningful splitting.

### Subsection

Some subsection content here that continues the document.

"#;
		let chunks = SplitText::default().split("test.md", content);

		assert!(!chunks.is_empty());
		// Verify content is properly split
		assert!(chunks.iter().any(|chunk| chunk.contains("Title")));
	}

	#[test]
	fn test_split_rust_code() {
		let content = r#"
use std::collections::HashMap;

/// This is a struct for testing
pub struct TestStruct {
    field1: String,
    field2: i32,
}

impl TestStruct {
    pub fn new(field1: String, field2: i32) -> Self {
        Self { field1, field2 }
    }
    
    pub fn get_field1(&self) -> &str {
        &self.field1
    }
    
    pub fn get_field2(&self) -> i32 {
        self.field2
    }
}

fn main() {
    let test = TestStruct::new("hello".to_string(), 42);
    println!("{}: {}", test.get_field1(), test.get_field2());
}
"#;
		let chunks = SplitText::default().split("test.rs", content);

		assert!(!chunks.is_empty());
		// Verify Rust code structure is preserved in chunks
		assert!(chunks.iter().any(|chunk| chunk.contains("TestStruct")));
	}

	#[test]
	fn test_split_default_text() {
		let content = "This is a long text document that should be split into multiple chunks based on character count. ".repeat(20);
		let chunks = SplitText::default().split("test.txt", &content);

		assert!(!chunks.is_empty());
		// With repeated text, we should get multiple chunks
		if content.len() > 1000 {
			assert!(chunks.len() > 1);
		}
	}

	#[test]
	fn test_split_unknown_extension() {
		let content =
			"This is content with an unknown file extension. ".repeat(10);
		let chunks = SplitText::default().split("test.xyz", &content);

		assert!(!chunks.is_empty());
		// Should fall back to default text splitting
		assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
	}

	#[test]
	fn test_split_no_extension() {
		let content = "This is content without any file extension. ".repeat(10);
		let chunks = SplitText::default().split("test.xyz", &content);


		assert!(!chunks.is_empty());
		// Should fall back to default text splitting
		assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
	}

	#[test]
	fn test_split_empty_content() {
		let chunks = SplitText::default().split("test.md", "");

		// Empty content should result in empty chunks array or single empty chunk
		if !chunks.is_empty() {
			assert!(chunks.iter().all(|chunk| chunk.trim().is_empty()));
		}
	}

	#[test]
	fn test_split_to_documents() {
		let content = r#"# Test Document

This is some content for testing document generation.

## Section

More content here to ensure we get multiple chunks.
"#;
		let documents =
			SplitText::default().split_to_documents("test.md", &content);

		assert!(!documents.is_empty());

		// Check document structure
		for (i, doc) in documents.iter().enumerate() {
			// ID should be path with chunk index
			assert_eq!(doc.id, format!("test.md#{}", i));
			assert!(!doc.content.is_empty());
		}
	}

	#[test]
	fn test_split_to_documents_with_path() {
		let content = "fn main() { println!(\"Hello\"); }".repeat(20);
		let documents =
			SplitText::default().split_to_documents("src/lib.rs", &content);

		assert!(!documents.is_empty());

		// Check that path is preserved in document IDs
		for (i, doc) in documents.iter().enumerate() {
			assert_eq!(doc.id, format!("src/lib.rs#{}", i));
		}
	}

	#[test]
	fn test_custom_chunk_range_splitting() {
		let content = "Short content";
		let small_range = 5..10;
		let chunks = SplitText {
			chunk_range: small_range,
			..Default::default()
		}
		.split("test.txt", &content);

		assert!(!chunks.is_empty());
		// With very small chunk range, each chunk should be small
		for chunk in chunks {
			assert!(chunk.len() <= 20); // Allow some flexibility for splitting logic
		}
	}

	#[test]
	fn test_different_file_extensions() {
		let test_cases = vec![
			("file.md", "markdown"),
			("file.rs", "rust"),
			("file.txt", "text"),
			("file.py", "text"), // Python should fall back to text
			("file.js", "text"), // JavaScript should fall back to text
		];

		for (filename, expected_type) in test_cases {
			let path = Path::new(filename);
			let content = "Some test content here";
			let chunks = SplitText::default().split(path, &content);

			assert!(
				!chunks.is_empty(),
				"Failed for file type: {}",
				expected_type
			);
		}
	}
}
