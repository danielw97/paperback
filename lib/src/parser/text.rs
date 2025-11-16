use std::{fs, path::Path};

use anyhow::{Context, Result};

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

	fn parse(&self, context: &ParserContext) -> Result<Document> {
		let bytes = fs::read(&context.file_path)
			.with_context(|| format!("Failed to open text file '{}'", context.file_path))?;
		if bytes.is_empty() {
			anyhow::bail!("Text file is empty: {}", context.file_path);
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
