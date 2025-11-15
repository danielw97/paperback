use std::{fs, path::Path};

use crate::{
	document::{Document, DocumentBuffer, ParserContext, ParserFlags},
	parser::Parser,
	utils::{encoding::convert_to_utf8, text::remove_soft_hyphens},
};

pub struct TextParser;

impl Parser for TextParser {
	fn name(&self) -> &str {
		"Text Files"
	}

	fn extensions(&self) -> &[&str] {
		&["txt", "log"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::NONE
	}

	fn parse(&self, context: &ParserContext) -> Result<Document, String> {
		let bytes = fs::read(&context.file_path)
			.map_err(|e| format!("Failed to open text file '{}': {}", context.file_path, e))?;
		if bytes.is_empty() {
			return Err(format!("Text file is empty: {}", context.file_path));
		}
		let utf8_content = convert_to_utf8(&bytes);
		let processed = remove_soft_hyphens(&utf8_content);
		let title =
			Path::new(&context.file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
		let mut doc = Document::new().with_title(title);
		doc.set_buffer(DocumentBuffer::with_content(processed));
		doc.compute_stats();
		Ok(doc)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_text_parser_properties() {
		let parser = TextParser;
		assert_eq!(parser.name(), "Text Files");
		assert_eq!(parser.extensions(), &["txt", "log"]);
		assert_eq!(parser.supported_flags(), ParserFlags::NONE);
	}

	#[test]
	fn test_parse_simple_text() {
		use std::io::Write;

		use tempfile::NamedTempFile;

		let mut temp_file = NamedTempFile::new().unwrap();
		writeln!(temp_file, "Hello, World!").unwrap();
		writeln!(temp_file, "This is a test.").unwrap();

		let context = ParserContext::new(temp_file.path().to_string_lossy().to_string());
		let parser = TextParser;
		let result = parser.parse(&context);

		assert!(result.is_ok());
		let doc = result.unwrap();
		assert!(doc.buffer.content.contains("Hello, World!"));
		assert!(doc.buffer.content.contains("This is a test."));
		assert!(doc.stats.word_count > 0);
	}
}
