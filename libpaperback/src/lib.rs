#![allow(non_camel_case_types)]

use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
	ptr,
	time::Duration,
};

use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;

const RELEASE_URL: &str = "https://api.github.com/repos/trypsynth/paperback/releases/latest";

#[repr(C)]
#[derive(Clone, Copy)]
pub enum paperback_update_status {
	PAPERBACK_UPDATE_STATUS_AVAILABLE = 0,
	PAPERBACK_UPDATE_STATUS_UP_TO_DATE = 1,
	PAPERBACK_UPDATE_STATUS_HTTP_ERROR = 2,
	PAPERBACK_UPDATE_STATUS_NETWORK_ERROR = 3,
	PAPERBACK_UPDATE_STATUS_INVALID_RESPONSE = 4,
	PAPERBACK_UPDATE_STATUS_NO_DOWNLOAD = 5,
	PAPERBACK_UPDATE_STATUS_INVALID_INPUT = 6,
	PAPERBACK_UPDATE_STATUS_INTERNAL_ERROR = 7,
}

#[repr(C)]
pub struct paperback_update_result {
	pub status: paperback_update_status,
	pub http_status: i32,
	pub latest_version: *mut c_char,
	pub download_url: *mut c_char,
	pub release_notes: *mut c_char,
	pub error_message: *mut c_char,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
	name: String,
	browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
	tag_name: String,
	body: Option<String>,
	assets: Option<Vec<ReleaseAsset>>,
}

enum UpdateOutcome {
	Available { latest_version: String, release_notes: String, download_url: String },
	UpToDate { latest_version: String },
	HttpError { status: i32 },
	NetworkError { message: String },
	InvalidResponse { message: String },
	NoDownload { message: String },
	InvalidInput { message: String },
}

fn sanitize_for_c(text: &str) -> String {
	text.replace('\0', " ")
}

fn opt_string_to_c(value: Option<String>) -> *mut c_char {
	match value {
		Some(val) if !val.is_empty() => match CString::new(sanitize_for_c(&val)) {
			Ok(cstr) => cstr.into_raw(),
			Err(_) => ptr::null_mut(),
		},
		_ => ptr::null_mut(),
	}
}

fn drop_c_string(ptr: *mut c_char) {
	if ptr.is_null() {
		return;
	}
	unsafe {
		drop(CString::from_raw(ptr));
	}
}

fn parse_semver_value(value: &str) -> Option<Version> {
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return None;
	}
	let normalized = trimmed.trim_start_matches(|c| c == 'v' || c == 'V');
	Version::parse(normalized).ok()
}

fn pick_download_url(is_installer: bool, assets: &[ReleaseAsset]) -> Option<String> {
	let preferred_name = if is_installer { "paperback_setup.exe" } else { "paperback.zip" };
	for asset in assets {
		if asset.name.eq_ignore_ascii_case(preferred_name) {
			return Some(asset.browser_download_url.clone());
		}
	}
	None
}

fn fetch_latest_release(user_agent: &str) -> Result<GithubRelease, UpdateOutcome> {
	let client = Client::builder()
		.user_agent(user_agent)
		.timeout(Duration::from_secs(15))
		.build()
		.map_err(|err| UpdateOutcome::NetworkError { message: format!("Failed to create HTTP client: {err}") })?;
	match client.get(RELEASE_URL).header("Accept", "application/vnd.github+json").send() {
		Ok(resp) => {
			if !resp.status().is_success() {
				return Err(UpdateOutcome::HttpError { status: resp.status().as_u16() as i32 });
			}
			resp.json::<GithubRelease>().map_err(|err| UpdateOutcome::InvalidResponse {
				message: format!("Failed to parse release JSON: {err}"),
			})
		}
		Err(err) => Err(UpdateOutcome::NetworkError { message: format!("Network error: {err}") }),
	}
}

fn run_update_check(current_version: &str, is_installer: bool, user_agent: &str) -> UpdateOutcome {
	let current = match parse_semver_value(current_version) {
		Some(v) => v,
		None => {
			return UpdateOutcome::InvalidInput {
				message: "Current version was not a valid semantic version.".to_string(),
			}
		}
	};
	let release = match fetch_latest_release(user_agent) {
		Ok(rel) => rel,
		Err(err) => return err,
	};
	let latest_version_value = parse_semver_value(&release.tag_name);
	let latest_semver = match latest_version_value {
		Some(v) => v,
		None => {
			return UpdateOutcome::InvalidResponse {
				message: "Latest release tag does not contain a valid semantic version.".to_string(),
			}
		}
	};
	if current >= latest_semver {
		return UpdateOutcome::UpToDate { latest_version: release.tag_name };
	}
	let download_url = match release.assets.as_ref() {
		Some(list) if !list.is_empty() => match pick_download_url(is_installer, list) {
			Some(url) => url,
			None => {
				return UpdateOutcome::NoDownload {
					message: "Update is available but no matching download asset was found.".to_string(),
				}
			}
		},
		_ => {
			return UpdateOutcome::NoDownload {
				message: "Latest release does not include downloadable assets.".to_string(),
			}
		}
	};
	UpdateOutcome::Available {
		latest_version: release.tag_name,
		release_notes: release.body.unwrap_or_default(),
		download_url,
	}
}

fn make_result(
	status: paperback_update_status,
	http_status: i32,
	latest_version: Option<String>,
	download_url: Option<String>,
	release_notes: Option<String>,
	error_message: Option<String>,
) -> *mut paperback_update_result {
	let result = paperback_update_result {
		status,
		http_status,
		latest_version: opt_string_to_c(latest_version),
		download_url: opt_string_to_c(download_url),
		release_notes: opt_string_to_c(release_notes),
		error_message: opt_string_to_c(error_message),
	};
	Box::into_raw(Box::new(result))
}

fn ptr_to_string(ptr: *const c_char) -> Result<String, String> {
	if ptr.is_null() {
		return Err("Null pointer provided.".to_string());
	}
	unsafe {
		CStr::from_ptr(ptr)
			.to_str()
			.map(|s| s.to_string())
			.map_err(|_| "Input contained invalid UTF-8 data.".to_string())
	}
}

#[no_mangle]
pub extern fn paperback_check_for_updates(
	current_version: *const c_char,
	is_installer_flag: u8,
) -> *mut paperback_update_result {
	let current_version_value = match ptr_to_string(current_version) {
		Ok(value) => value,
		Err(message) => {
			return make_result(
				paperback_update_status::PAPERBACK_UPDATE_STATUS_INVALID_INPUT,
				0,
				None,
				None,
				None,
				Some(message),
			)
		}
	};
	let user_agent = format!("paperback/{}", env!("CARGO_PKG_VERSION"));
	let is_installer = is_installer_flag != 0;
	let outcome = run_update_check(&current_version_value, is_installer, &user_agent);
	match outcome {
		UpdateOutcome::Available { latest_version, release_notes, download_url } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_AVAILABLE,
			0,
			Some(latest_version),
			Some(download_url),
			Some(release_notes),
			None,
		),
		UpdateOutcome::UpToDate { latest_version } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_UP_TO_DATE,
			0,
			Some(latest_version),
			None,
			None,
			None,
		),
		UpdateOutcome::HttpError { status } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_HTTP_ERROR,
			status,
			None,
			None,
			None,
			Some(format!("GitHub returned HTTP status {status}.")),
		),
		UpdateOutcome::NetworkError { message } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_NETWORK_ERROR,
			0,
			None,
			None,
			None,
			Some(message),
		),
		UpdateOutcome::InvalidResponse { message } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_INVALID_RESPONSE,
			0,
			None,
			None,
			None,
			Some(message),
		),
		UpdateOutcome::NoDownload { message } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_NO_DOWNLOAD,
			0,
			None,
			None,
			None,
			Some(message),
		),
		UpdateOutcome::InvalidInput { message } => make_result(
			paperback_update_status::PAPERBACK_UPDATE_STATUS_INVALID_INPUT,
			0,
			None,
			None,
			None,
			Some(message),
		),
	}
}

#[no_mangle]
pub extern fn paperback_free_update_result(result: *mut paperback_update_result) {
	if result.is_null() {
		return;
	}
	unsafe {
		drop_c_string((*result).latest_version);
		drop_c_string((*result).download_url);
		drop_c_string((*result).release_notes);
		drop_c_string((*result).error_message);
		drop(Box::from_raw(result));
	}
}
