use std::collections::HashMap;

use anyhow::{Context, Result};
use scraper::{Html, Selector};

use crate::{
	chm_ffi::{CHM_ENUMERATE_ALL, ChmHandle, unit_info_path},
	document::{Document, DocumentBuffer, Marker, MarkerType, ParserContext, ParserFlags, TocItem},
	html_to_text::{HtmlSourceMode, HtmlToText},
	parser::{
		Parser,
		utils::{extract_title_from_path, heading_level_to_marker_type},
	},
	utils::encoding::convert_to_utf8,
};

pub struct ChmParser;

impl Parser for ChmParser {
	fn name(&self) -> &'static str {
		"Compiled HTML Help files"
	}

	fn extensions(&self) -> &[&str] {
		&["chm"]
	}

	fn supported_flags(&self) -> ParserFlags {
		ParserFlags::SUPPORTS_TOC
	}

	fn parse(&self, context: &ParserContext) -> Result<Document> {
		let mut chm = ChmHandle::open(&context.file_path)
			.with_context(|| format!("Failed to open CHM file: {}", context.file_path))?;
		let mut html_files = Vec::new();
		let mut hhc_file = String::new();
		chm.enumerate(CHM_ENUMERATE_ALL, |ui| {
			let path = unit_info_path(ui);
			let lower_path = path.to_lowercase();
			if lower_path.contains(".hhc") {
				if hhc_file.is_empty() || lower_path.contains("index.hhc") {
					hhc_file = path.clone();
				}
			}
			if (lower_path.contains(".htm") || lower_path.contains(".html"))
				&& !path.contains("/#")
				&& !path.contains("/$")
			{
				html_files.push(path);
			}
			true
		})?;
		html_files.sort();
		let title = parse_system_file(&mut chm).unwrap_or_else(|| extract_title_from_path(&context.file_path));
		let mut toc_items = if !hhc_file.is_empty() { parse_hhc_file(&mut chm, &hhc_file)? } else { Vec::new() };
		let ordered_files = build_ordered_file_list(&html_files, &toc_items);
		let mut buffer = DocumentBuffer::new();
		let mut id_positions = HashMap::new();
		let mut file_positions = HashMap::new();
		for file_path in ordered_files {
			let section_start = buffer.current_position();
			let content_bytes = match chm.read_file(&file_path) {
				Ok(bytes) => bytes,
				Err(_) => continue,
			};
			if content_bytes.is_empty() {
				continue;
			}
			let content = convert_to_utf8(&content_bytes);
			let mut converter = HtmlToText::new();
			if !converter.convert(&content, HtmlSourceMode::NativeHtml) {
				continue;
			}
			let text = converter.get_text();
			let headings = converter.get_headings();
			let links = converter.get_links();
			let section_id_positions = converter.get_id_positions();
			let normalized_path = normalize_path(&file_path);
			file_positions.insert(normalized_path.clone(), section_start);
			for (id, relative_pos) in section_id_positions {
				let absolute_pos = section_start + relative_pos;
				id_positions.insert(format!("{}#{}", normalized_path, id), absolute_pos);
			}
			buffer.append(&text);
			for heading in headings {
				let marker_type = heading_level_to_marker_type(heading.level);
				buffer.add_marker(
					Marker::new(marker_type, section_start + heading.offset)
						.with_text(heading.text.clone())
						.with_level(heading.level),
				);
			}
			for link in links {
				let resolved_href = resolve_link(&file_path, &link.reference);
				buffer.add_marker(
					Marker::new(MarkerType::Link, section_start + link.offset)
						.with_text(link.text.clone())
						.with_reference(resolved_href),
				);
			}
			if !buffer.content.ends_with('\n') {
				buffer.append("\n");
			}
		}
		calculate_toc_offsets(&mut toc_items, &file_positions, &id_positions);
		let mut document = Document::new().with_title(title);
		document.set_buffer(buffer);
		document.id_positions = id_positions;
		document.toc_items = toc_items;
		Ok(document)
	}
}

fn parse_system_file(chm: &mut ChmHandle) -> Option<String> {
	let content = chm.read_file("/#SYSTEM").ok()?;
	if content.len() < 4 {
		return None;
	}
	let read_le16 = |data: &[u8], offset: usize| -> u16 { u16::from_le_bytes([data[offset], data[offset + 1]]) };
	let mut index = 4;
	while index + 4 <= content.len() {
		let code = read_le16(&content, index);
		let length = read_le16(&content, index + 2) as usize;
		if index + 4 + length > content.len() {
			break;
		}
		// Code 3 is the title.
		if code == 3 && length > 0 {
			let title_bytes = &content[index + 4..index + 4 + length];
			let title_bytes =
				if title_bytes.last() == Some(&0) { &title_bytes[..title_bytes.len() - 1] } else { title_bytes };
			let title = String::from_utf8_lossy(title_bytes).to_string();
			if !title.trim().is_empty() {
				return Some(title);
			}
		}
		index += 4 + length;
	}
	None
}

fn parse_hhc_file(chm: &mut ChmHandle, hhc_path: &str) -> Result<Vec<TocItem>> {
	let content_bytes = chm.read_file(hhc_path).with_context(|| format!("Failed to read .hhc file: {}", hhc_path))?;
	if content_bytes.is_empty() {
		return Ok(Vec::new());
	}
	let content = convert_to_utf8(&content_bytes);
	let document = Html::parse_document(&content);
	let body_selector = Selector::parse("body").unwrap();
	let Some(body) = document.select(&body_selector).next() else {
		return Ok(Vec::new());
	};
	let mut toc_items = Vec::new();
	parse_hhc_node(body, &mut toc_items);
	Ok(toc_items)
}

fn parse_hhc_node(node: scraper::ElementRef, items: &mut Vec<TocItem>) {
	let li_selector = Selector::parse("li").unwrap();
	let object_selector = Selector::parse("object").unwrap();
	let param_selector = Selector::parse("param").unwrap();
	let ul_selector = Selector::parse("ul").unwrap();
	for li in node.select(&li_selector) {
		let mut name = String::new();
		let mut local = String::new();
		for object in li.select(&object_selector) {
			for param in object.select(&param_selector) {
				let param_name = param.value().attr("name").unwrap_or("").to_lowercase();
				let param_value = param.value().attr("value").unwrap_or("");
				match param_name.as_str() {
					"name" => name = param_value.to_string(),
					"local" => local = param_value.to_string(),
					_ => {}
				}
			}
		}
		if !name.is_empty() {
			let mut item = TocItem::new(name, local, usize::MAX);
			for ul in li.select(&ul_selector) {
				parse_hhc_node(ul, &mut item.children);
			}
			items.push(item);
		}
	}
}

fn build_ordered_file_list(html_files: &[String], toc_items: &[TocItem]) -> Vec<String> {
	if toc_items.is_empty() {
		return html_files.to_vec();
	}
	let mut ordered = Vec::new();
	let mut seen = std::collections::HashSet::new();
	let mut path_map = HashMap::new();
	for file in html_files {
		let normalized = normalize_path(file);
		path_map.insert(normalized, file.clone());
	}
	let mut toc_files = Vec::new();
	collect_toc_files(toc_items, &mut toc_files);
	for toc_file in toc_files {
		let normalized = normalize_path(&toc_file);
		if let Some(actual_path) = path_map.get(&normalized) {
			if seen.insert(normalized) {
				ordered.push(actual_path.clone());
			}
		}
	}
	for file in html_files {
		let normalized = normalize_path(file);
		if !seen.contains(&normalized) {
			ordered.push(file.clone());
		}
	}
	ordered
}

fn collect_toc_files(items: &[TocItem], files: &mut Vec<String>) {
	for item in items {
		if !item.reference.is_empty() {
			let file_path =
				if let Some(pos) = item.reference.find('#') { &item.reference[..pos] } else { &item.reference };
			if !file_path.is_empty() && !files.contains(&file_path.to_string()) {
				files.push(file_path.to_string());
			}
		}
		collect_toc_files(&item.children, files);
	}
}

fn normalize_path(path: &str) -> String {
	let mut result = path.replace('\\', "/").to_lowercase();
	if !result.starts_with('/') {
		result.insert(0, '/');
	}
	result
}

fn resolve_link(current_file: &str, href: &str) -> String {
	let href_lower = href.to_lowercase();
	if href_lower.starts_with("http:") || href_lower.starts_with("https:") || href_lower.starts_with("mailto:") {
		return href.to_string();
	}
	let current_path = std::path::Path::new(current_file);
	let current_dir = current_path.parent().unwrap_or_else(|| std::path::Path::new("/"));
	let resolved = current_dir.join(href);
	resolved.to_string_lossy().replace('\\', "/")
}

fn calculate_toc_offsets(
	items: &mut [TocItem],
	file_positions: &HashMap<String, usize>,
	id_positions: &HashMap<String, usize>,
) {
	for item in items {
		if !item.reference.is_empty() {
			item.offset = calculate_offset_from_reference(&item.reference, file_positions, id_positions);
		}
		calculate_toc_offsets(&mut item.children, file_positions, id_positions);
	}
}

fn calculate_offset_from_reference(
	reference: &str,
	file_positions: &HashMap<String, usize>,
	id_positions: &HashMap<String, usize>,
) -> usize {
	let (file_path, fragment) = if let Some(pos) = reference.find('#') {
		(&reference[..pos], Some(&reference[pos + 1..]))
	} else {
		(reference.as_ref(), None)
	};
	let normalized_path = normalize_path(file_path);
	if let Some(fragment_id) = fragment {
		let id_key = format!("{}#{}", normalized_path, fragment_id);
		if let Some(&offset) = id_positions.get(&id_key) {
			return offset;
		}
	}
	file_positions.get(&normalized_path).copied().unwrap_or(usize::MAX)
}
