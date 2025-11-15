fn main() {
	cxx_build::bridge("src/bridge.rs").flag_if_supported("-std=c++20").compile("paperback-bridge");
	println!("cargo:rerun-if-changed=src/bridge.rs");
}
