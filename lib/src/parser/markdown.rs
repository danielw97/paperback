use std::{collections::HashMap, fs, path::Path};

use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};

use crate::{
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags, TocItem},
	parser::Parser,
	utils::encoding::convert_to_utf8,
};

pub struct MarkdownParser;

struct HeadingInfo {
	text: String,
	level: usize,
	position: usize,
}

impl Parser for MarkdownParser {
	fn name(&self) -> &str {
		"Markdown Files"
	}

	fn extensions(&self) -> &[&str] {
		&["md", "markdown", "mdown", "mkdn", "mkd"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_TOC
	}

	fn parse(&self, context: &ParserContext) -> Result<Document, String> {
		let bytes = fs::read(&context.file_path)
			.map_err(|e| format!("Failed to open Markdown file '{}': {}", context.file_path, e))?;
		if bytes.is_empty() {
			return Err(format!("Markdown file is empty: {}", context.file_path));
		}
		let markdown_content = convert_to_utf8(&bytes);
		let (text, headings, links, id_positions) = parse_markdown_to_text(&markdown_content)?;
		// Extract title from filename
		let title =
			Path::new(&context.file_path).file_stem().and_then(|s| s.to_str()).unwrap_or("Untitled").to_string();
		let mut buffer = DocumentBuffer::with_content(text);
		for heading in headings.iter() {
			let marker_type = match heading.level {
				1 => MarkerType::Heading1,
				2 => MarkerType::Heading2,
				3 => MarkerType::Heading3,
				4 => MarkerType::Heading4,
				5 => MarkerType::Heading5,
				_ => MarkerType::Heading6,
			};
			buffer.add_marker(
				Marker::new(marker_type, heading.position)
					.with_text(heading.text.clone())
					.with_level(heading.level as i32),
			);
		}
		for (position, (text, url)) in links {
			buffer.add_marker(Marker::new(MarkerType::Link, position).with_text(text).with_reference(url));
		}
		let toc_items = build_toc_from_headings(&headings);
		let mut doc = Document::new().with_title(title);
		doc.set_buffer(buffer);
		doc.toc_items = toc_items;
		doc.id_positions = id_positions;
		doc.compute_stats();
		Ok(doc)
	}
}

fn parse_markdown_to_text(
	markdown: &str,
) -> Result<(String, Vec<HeadingInfo>, Vec<(usize, (String, String))>, HashMap<String, usize>), String> {
	let parser = pulldown_cmark::Parser::new(markdown);
	let mut text = String::new();
	let mut headings = Vec::new();
	let mut links = Vec::new();
	let mut id_positions = HashMap::new();
	let mut current_heading_level: Option<usize> = None;
	let mut current_heading_text = String::new();
	let mut current_link_text = String::new();
	let mut current_link_url = String::new();
	let mut in_link = false;
	for event in parser {
		match event {
			Event::Start(Tag::Heading { level, id, .. }) => {
				let heading_level = match level {
					HeadingLevel::H1 => 1,
					HeadingLevel::H2 => 2,
					HeadingLevel::H3 => 3,
					HeadingLevel::H4 => 4,
					HeadingLevel::H5 => 5,
					HeadingLevel::H6 => 6,
				};
				current_heading_level = Some(heading_level);
				current_heading_text.clear();
				if let Some(id_str) = id {
					id_positions.insert(id_str.to_string(), text.len());
				}
			}
			Event::End(TagEnd::Heading(_)) => {
				if let Some(level) = current_heading_level {
					let heading_text = current_heading_text.trim().to_string();
					if !heading_text.is_empty() {
						headings.push(HeadingInfo { text: heading_text.clone(), level, position: text.len() });
						text.push_str(&heading_text);
						text.push('\n');
						text.push('\n');
					}
					current_heading_level = None;
					current_heading_text.clear();
				}
			}
			Event::Start(Tag::Link { dest_url, .. }) => {
				in_link = true;
				current_link_url = dest_url.to_string();
				current_link_text.clear();
			}
			Event::End(TagEnd::Link) => {
				if in_link {
					links.push((text.len(), (current_link_text.clone(), current_link_url.clone())));
					text.push_str(&current_link_text);
					in_link = false;
				}
			}
			Event::Text(t) => {
				let content = t.to_string();
				if current_heading_level.is_some() {
					current_heading_text.push_str(&content);
				} else if in_link {
					current_link_text.push_str(&content);
				} else {
					text.push_str(&content);
				}
			}
			Event::Code(code) => {
				let code_str = code.to_string();
				if current_heading_level.is_some() {
					current_heading_text.push_str(&code_str);
				} else if in_link {
					current_link_text.push_str(&code_str);
				} else {
					text.push_str(&code_str);
				}
			}
			Event::SoftBreak | Event::HardBreak => {
				if current_heading_level.is_none() && !in_link {
					text.push('\n');
				} else if in_link {
					current_link_text.push(' ');
				}
			}
			Event::Start(Tag::Paragraph) => {
				if !text.is_empty() && !text.ends_with("\n\n") {
					if !text.ends_with('\n') {
						text.push('\n');
					}
				}
			}
			Event::End(TagEnd::Paragraph) => {
				if !text.ends_with('\n') {
					text.push('\n');
				}
				text.push('\n');
			}
			Event::Start(Tag::List(_)) => {
				if !text.ends_with("\n\n") && !text.is_empty() {
					text.push('\n');
				}
			}
			Event::End(TagEnd::List(_)) => {
				if !text.ends_with('\n') {
					text.push('\n');
				}
			}
			Event::Start(Tag::Item) => {}
			Event::End(TagEnd::Item) => {
				if !text.ends_with('\n') {
					text.push('\n');
				}
			}
			Event::Start(Tag::CodeBlock(_)) => {
				if !text.ends_with("\n\n") && !text.is_empty() {
					text.push('\n');
				}
			}
			Event::End(TagEnd::CodeBlock) => {
				if !text.ends_with('\n') {
					text.push('\n');
				}
				text.push('\n');
			}
			Event::Start(Tag::BlockQuote(_)) => {
				if !text.ends_with("\n\n") && !text.is_empty() {
					text.push('\n');
				}
			}
			Event::End(TagEnd::BlockQuote(_)) => {
				if !text.ends_with('\n') {
					text.push('\n');
				}
				text.push('\n');
			}
			_ => {}
		}
	}
	Ok((text, headings, links, id_positions))
}

fn build_toc_from_headings(headings: &[HeadingInfo]) -> Vec<TocItem> {
	if headings.is_empty() {
		return Vec::new();
	}
	let mut toc = Vec::new();
	let mut stack: Vec<(usize, Vec<usize>)> = Vec::new(); // (level, path to current node)
	for heading in headings {
		let item = TocItem::new(heading.text.clone(), String::new(), heading.position);
		while let Some((level, _)) = stack.last() {
			if *level < heading.level {
				break;
			}
			stack.pop();
		}
		if stack.is_empty() {
			toc.push(item);
			stack.push((heading.level, vec![toc.len() - 1]));
		} else {
			let (_, path) = stack.last().unwrap();
			let mut current = &mut toc;
			for &idx in &path[..path.len() - 1] {
				current = &mut current[idx].children;
			}
			let parent_idx = *path.last().unwrap();
			current[parent_idx].children.push(item);
			let mut new_path = path.clone();
			new_path.push(current[parent_idx].children.len() - 1);
			stack.push((heading.level, new_path));
		}
	}
	toc
}
