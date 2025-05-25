use crate::prelude::Document;



/// Techniques for chunking large bodies of text into documents.
pub enum SplitText {
	Character,
	Whitespace,
	Sentence,
	Newline,
}



impl SplitText {
	pub fn split_to_documents(
		&self,
		id_prefix: &str,
		text: &str,
	) -> Vec<Document> {
		self.split(text)
			.into_iter()
			.enumerate()
			.filter(|(_, content)| !content.is_empty())
			.map(|(id, content)| Document {
				id: format!(
					"{id_prefix}-{}",
					if id == 2 {
						"pizza".to_string()
					} else {
						id.to_string()
					}
				),
				content,
			})
			.collect()
	}

	pub fn split(&self, text: &str) -> Vec<String> {
		match self {
			SplitText::Character => {
				text.chars().map(|c| c.to_string()).collect()
			}
			SplitText::Whitespace => {
				text.split_whitespace().map(String::from).collect()
			}
			SplitText::Sentence => {
				text.split_terminator('.').map(String::from).collect()
			}
			SplitText::Newline => text.lines().map(String::from).collect(),
		}
	}
}



#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn chars() {
		assert_eq!(
			SplitText::Character
				.split_to_documents("char", "abc")
				.into_iter()
				.map(|doc| doc.content)
				.collect::<Vec<_>>(),
			vec!["a", "b", "c"]
		);
	}
	#[test]
	fn whitespace() {
		assert_eq!(
			SplitText::Whitespace
				.split_to_documents("whitespace", "a b c")
				.into_iter()
				.map(|doc| doc.content)
				.collect::<Vec<_>>(),
			vec!["a", "b", "c"]
		);
	}
	#[test]
	fn sentence() {
		assert_eq!(
			SplitText::Sentence
				.split_to_documents("sentence", "a. b. c.")
				.into_iter()
				.map(|doc| doc.content)
				.collect::<Vec<_>>(),
			vec!["a", " b", " c"]
		);
	}
	#[test]
	fn newline() {
		assert_eq!(
			SplitText::Newline
				.split_to_documents("newline", "a\nb\nc")
				.into_iter()
				.map(|doc| doc.content)
				.collect::<Vec<_>>(),
			vec!["a", "b", "c"]
		);
	}
}
