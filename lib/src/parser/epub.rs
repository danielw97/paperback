use std::{
	collections::HashMap,
	path::{Component, Path, PathBuf},
};

use epub::doc::{EpubDoc, NavPoint};

use crate::{
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags, TocItem},
	html_to_text::{HeadingInfo, HtmlSourceMode, HtmlToText, LinkInfo, ListInfo, ListItemInfo},
	parser::Parser,
	utils::text::trim_string,
	xml_to_text::XmlToText,
};

struct SectionContent {
	text: String,
	headings: Vec<HeadingInfo>,
	links: Vec<LinkInfo>,
	lists: Vec<ListInfo>,
	list_items: Vec<ListItemInfo>,
	id_positions: HashMap<String, usize>,
}

struct SectionMeta {
	path: String,
	start: usize,
	end: usize,
}

pub struct EpubParser;

impl Parser for EpubParser {
	fn name(&self) -> &str {
		"EPUB Books"
	}

	fn extensions(&self) -> &[&str] {
		&["epub"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_SECTIONS | ParserFlags::SUPPORTS_TOC | ParserFlags::SUPPORTS_LISTS
	}

	fn parse(&self, context: &ParserContext) -> Result<Document, String> {
		let mut epub = EpubDoc::new(&context.file_path).map_err(|e| format!("Failed to open EPUB '{}': {e}", context.file_path))?;
		let mut buffer = DocumentBuffer::new();
		let mut id_positions = HashMap::new();
		let mut sections = Vec::new();
		let manifest_items: HashMap<String, String> = epub
			.resources
			.iter()
			.map(|(id, item)| (id.clone(), normalize_path(&item.path)))
			.collect();
		let mut conversion_errors = Vec::new();
		let spine_items: Vec<_> = epub.spine.iter().map(|item| item.idref.clone()).collect();
		for (index, idref) in spine_items.iter().enumerate() {
			let (resource_mime, resource_path) = {
				let Some(resource) = epub.resources.get(idref) else {
					continue;
				};
				(resource.mime.clone(), resource.path.clone())
			};
			if !is_textual_mime(&resource_mime) {
				continue;
			}
			let (content, _) = match epub.get_resource_str(idref) {
				Some(tuple) => tuple,
				None => {
					conversion_errors.push(idref.clone());
					continue;
				}
			};
			let section_path = normalize_path(&resource_path);
			let section_start = buffer.current_position();
			let section_label = format!("Section {}", index + 1);
			buffer.add_marker(Marker::new(MarkerType::SectionBreak, section_start).with_text(section_label));
			match convert_section(&content) {
				Ok(section) => {
					for (id, relative) in section.id_positions {
						id_positions.insert(id, section_start + relative);
					}
					for heading in section.headings {
						let marker_type = heading_marker_type(heading.level);
						buffer.add_marker(
							Marker::new(marker_type, section_start + heading.offset).with_text(heading.text.clone()).with_level(heading.level),
						);
					}
					for link in section.links {
						let resolved = resolve_href(&section_path, &link.reference);
						buffer.add_marker(
							Marker::new(MarkerType::Link, section_start + link.offset)
								.with_text(link.text.clone())
								.with_reference(resolved),
						);
					}
					for list in section.lists {
						buffer.add_marker(Marker::new(MarkerType::List, section_start + list.offset).with_level(list.item_count));
					}
					for list_item in section.list_items {
						buffer.add_marker(
							Marker::new(MarkerType::ListItem, section_start + list_item.offset)
								.with_text(list_item.text.clone())
								.with_level(list_item.level),
						);
					}
					buffer.append(&section.text);
					if !buffer.content.ends_with('\n') {
						buffer.append("\n");
					}
					let section_end = buffer.current_position();
					sections.push(SectionMeta { path: section_path.clone(), start: section_start, end: section_end });
				}
				Err(err) => {
					conversion_errors.push(format!("{} ({err})", idref));
				}
			}
		}
		if sections.is_empty() {
			let reason = if conversion_errors.is_empty() {
				String::from("no readable spine items")
			} else {
				format!("failed to convert spine items: {}", conversion_errors.join(", "))
			};
			return Err(format!("EPUB has no readable content ({reason})"));
		}
		let title = epub
			.get_title()
			.filter(|t| !t.trim().is_empty())
			.unwrap_or_else(|| fallback_title(&context.file_path));
		let author = epub
			.mdata("creator")
			.map(|item| trim_string(&item.value))
			.filter(|s| !s.is_empty())
			.unwrap_or_default();
		let toc_items = build_toc(&epub.toc, &sections, &id_positions);
		let mut document = Document::new().with_title(title).with_author(author);
		document.set_buffer(buffer);
		document.id_positions = id_positions;
		document.spine_items = epub.spine.iter().map(|item| item.idref.clone()).collect();
		document.manifest_items = manifest_items;
		document.toc_items = toc_items;
		document.compute_stats();
		Ok(document)
	}
}

fn fallback_title(path: &str) -> String {
	Path::new(path)
		.file_stem()
		.and_then(|stem| stem.to_str())
		.unwrap_or("Untitled")
		.to_string()
}

fn convert_section(content: &str) -> Result<SectionContent, String> {
	let mut xml_converter = XmlToText::new();
	if xml_converter.convert(content) {
		return Ok(SectionContent {
			text: xml_converter.get_text(),
			headings: xml_converter.get_headings().to_vec(),
			links: xml_converter.get_links().to_vec(),
			lists: xml_converter.get_lists().to_vec(),
			list_items: xml_converter.get_list_items().to_vec(),
			id_positions: xml_converter.get_id_positions().clone(),
		});
	}
	let mut html_converter = HtmlToText::new();
	if html_converter.convert(content, HtmlSourceMode::NativeHtml) {
		return Ok(SectionContent {
			text: html_converter.get_text(),
			headings: html_converter.get_headings().to_vec(),
			links: html_converter.get_links().to_vec(),
			lists: html_converter.get_lists().to_vec(),
			list_items: html_converter.get_list_items().to_vec(),
			id_positions: html_converter.get_id_positions().clone(),
		});
	}
	Err("unsupported content".into())
}

fn heading_marker_type(level: i32) -> MarkerType {
	match level {
		1 => MarkerType::Heading1,
		2 => MarkerType::Heading2,
		3 => MarkerType::Heading3,
		4 => MarkerType::Heading4,
		5 => MarkerType::Heading5,
		_ => MarkerType::Heading6,
	}
}

fn resolve_href(current_path: &str, target: &str) -> String {
	let lower = target.to_ascii_lowercase();
	if lower.starts_with("http:") || lower.starts_with("https:") || lower.starts_with("mailto:") {
		return target.to_string();
	}
	if target.starts_with('#') {
		return target.to_string();
	}
	let (path_part, fragment) = split_href(target);
	let resolved = if path_part.is_empty() {
		current_path.to_string()
	} else {
		let mut base = PathBuf::from(current_path);
		base.pop();
		let joined = if path_part.starts_with('/') {
			PathBuf::from(path_part.trim_start_matches('/'))
		} else {
			base.join(path_part)
		};
		normalize_path(&joined)
	};
	if let Some(frag) = fragment {
		if frag.is_empty() {
			resolved
		} else {
			format!("{resolved}#{frag}")
		}
	} else {
		resolved
	}
}

fn split_href(input: &str) -> (String, Option<String>) {
	let trimmed = input.strip_prefix("epub://").unwrap_or(input);
	if let Some((path, frag)) = trimmed.split_once('#') {
		(path.to_string(), Some(frag.to_string()))
	} else {
		(trimmed.to_string(), None)
	}
}

fn normalize_path(path: &Path) -> String {
	let mut components = Vec::new();
	for component in path.components() {
		match component {
			Component::ParentDir => {
				components.pop();
			}
			Component::Normal(part) => components.push(part.to_string_lossy().to_string()),
			Component::CurDir => {}
			_ => {}
		}
	}
	components.join("/")
}

fn build_toc(navpoints: &[NavPoint], sections: &[SectionMeta], id_positions: &HashMap<String, usize>) -> Vec<TocItem> {
	navpoints.iter().map(|nav| convert_navpoint(nav, sections, id_positions)).collect()
}

fn convert_navpoint(nav: &NavPoint, sections: &[SectionMeta], id_positions: &HashMap<String, usize>) -> TocItem {
	let reference = normalize_path(&nav.content);
	let offset = compute_navpoint_offset(&reference, sections, id_positions);
	let mut item = TocItem::new(nav.label.clone(), reference.clone(), offset);
	item.children = nav.children.iter().map(|child| convert_navpoint(child, sections, id_positions)).collect();
	item
}

fn compute_navpoint_offset(reference: &str, sections: &[SectionMeta], id_positions: &HashMap<String, usize>) -> usize {
	let (path_part, fragment) = split_href(reference);
	if let Some(section) = sections.iter().find(|section| section.path == path_part) {
		if let Some(frag) = fragment.as_deref() {
			if let Some(offset) = id_positions.get(frag) {
				if *offset >= section.start && *offset < section.end {
					return *offset;
				}
			}
		}
		return section.start;
	}
	if let Some(frag) = fragment {
		if let Some(offset) = id_positions.get(&frag) {
			return *offset;
		}
	}
	0
}

fn is_textual_mime(mime: &str) -> bool {
	let mime_lower = mime.to_ascii_lowercase();
	mime_lower == "application/xhtml+xml"
		|| mime_lower == "application/x-dtbook+xml"
		|| mime_lower == "application/xml"
		|| mime_lower == "text/xml"
		|| mime_lower == "text/html"
		|| mime_lower.ends_with("+xml")
		|| mime_lower.starts_with("text/")
}
