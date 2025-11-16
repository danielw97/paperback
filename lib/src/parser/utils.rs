use crate::{
	document::{DocumentBuffer, MarkerType, TocItem},
	html_to_text::HeadingInfo,
};

pub fn build_toc_from_buffer(buffer: &DocumentBuffer) -> Vec<TocItem> {
	let headings: Vec<HeadingInfo> = buffer
		.markers
		.iter()
		.filter_map(|marker| {
			if matches!(
				marker.marker_type,
				MarkerType::Heading1
					| MarkerType::Heading2
					| MarkerType::Heading3
					| MarkerType::Heading4
					| MarkerType::Heading5
					| MarkerType::Heading6
			) {
				let level = match marker.marker_type {
					MarkerType::Heading1 => 1,
					MarkerType::Heading2 => 2,
					MarkerType::Heading3 => 3,
					MarkerType::Heading4 => 4,
					MarkerType::Heading5 => 5,
					MarkerType::Heading6 => 6,
					_ => 0,
				};
				Some(HeadingInfo { offset: marker.position, level, text: marker.text.clone() })
			} else {
				None
			}
		})
		.collect();
	build_toc_from_headings(&headings)
}

pub fn build_toc_from_headings(headings: &[HeadingInfo]) -> Vec<TocItem> {
	if headings.is_empty() {
		return Vec::new();
	}
	let mut toc = Vec::new();
	let mut stack: Vec<(i32, Vec<usize>)> = Vec::new(); // (level, path to current node)
	for heading in headings {
		let item = TocItem::new(heading.text.clone(), String::new(), heading.offset);
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
