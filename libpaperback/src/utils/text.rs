use std::os::raw::c_char;

use percent_encoding::percent_decode_str;

use crate::ffi::{c_str_to_string, string_to_c_str};

#[must_use]
pub fn remove_soft_hyphens(input: &str) -> String {
	input.replace("\u{00AD}", "")
}

#[must_use]
pub fn url_decode(input: &str) -> String {
	percent_decode_str(input).decode_utf8_lossy().into_owned()
}

#[must_use]
pub fn collapse_whitespace(input: &str) -> String {
	let mut result = String::with_capacity(input.len());
	let mut prev_was_space = false;
	for ch in input.chars() {
		let is_space = ch.is_whitespace() || ch == '\u{00A0}';
		if is_space {
			if !prev_was_space {
				result.push(' ');
				prev_was_space = true;
			}
		} else {
			result.push(ch);
			prev_was_space = false;
		}
	}
	result
}

/// Trims whitespace and non-breaking spaces from the start and end of a string.
#[must_use]
pub fn trim_string(s: &str) -> String {
	// Trim regular whitespace and NBSP (U+00A0)
	s.trim_matches(|c: char| c.is_whitespace() || c == '\u{00A0}').to_string()
}

// FFI exports

#[no_mangle]
pub extern fn paperback_remove_soft_hyphens(input: *const c_char) -> *mut c_char {
	let input_str = match unsafe { c_str_to_string(input) } {
		Some(s) => s,
		None => return std::ptr::null_mut(),
	};

	let result = remove_soft_hyphens(&input_str);
	string_to_c_str(result)
}

#[no_mangle]
pub extern fn paperback_url_decode(encoded: *const c_char) -> *mut c_char {
	let encoded_str = match unsafe { c_str_to_string(encoded) } {
		Some(s) => s,
		None => return std::ptr::null_mut(),
	};

	let result = url_decode(&encoded_str);
	string_to_c_str(result)
}

#[no_mangle]
pub extern fn paperback_collapse_whitespace(input: *const c_char) -> *mut c_char {
	let input_str = match unsafe { c_str_to_string(input) } {
		Some(s) => s,
		None => return std::ptr::null_mut(),
	};

	let result = collapse_whitespace(&input_str);
	string_to_c_str(result)
}

#[no_mangle]
pub extern fn paperback_trim_string(input: *const c_char) -> *mut c_char {
	let input_str = match unsafe { c_str_to_string(input) } {
		Some(s) => s,
		None => return std::ptr::null_mut(),
	};

	let result = trim_string(&input_str);
	string_to_c_str(result)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_remove_soft_hyphens() {
		assert_eq!(remove_soft_hyphens("hel\u{00AD}lo"), "hello");
		assert_eq!(remove_soft_hyphens("no hyphens"), "no hyphens");
		assert_eq!(remove_soft_hyphens("mul\u{00AD}ti\u{00AD}ple"), "multiple");
	}

	#[test]
	fn test_url_decode() {
		assert_eq!(url_decode("hello+world"), "hello world");
		assert_eq!(url_decode("hello%20world"), "hello world");
		assert_eq!(url_decode("test%2Fpath"), "test/path");
		assert_eq!(url_decode("100%25"), "100%");
		// UTF-8 encoded character (é = 0xC3A9)
		assert_eq!(url_decode("caf%C3%A9"), "café");
	}

	#[test]
	fn test_collapse_whitespace() {
		assert_eq!(collapse_whitespace("hello   world"), "hello world");
		assert_eq!(collapse_whitespace("hello\n\nworld"), "hello world");
		assert_eq!(collapse_whitespace("hello\t\tworld"), "hello world");
		assert_eq!(collapse_whitespace("  spaces  "), "  spaces ");
		// NBSP (U+00A0)
		assert_eq!(collapse_whitespace("hello\u{00A0}\u{00A0}world"), "hello world");
	}

	#[test]
	fn test_trim_string() {
		assert_eq!(trim_string("  hello  "), "hello");
		assert_eq!(trim_string("\n\nhello\n\n"), "hello");
		assert_eq!(trim_string("\u{00A0}hello\u{00A0}"), "hello");
		assert_eq!(trim_string("hello"), "hello");
	}
}
