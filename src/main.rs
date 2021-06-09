use wasmir::wasmir;
use toml::toml;

// #[wasmir(
// [dependencies]
// wasm-bindgen = "*"
// [dependencies.web-sys]
// version = "*"
// features = ["Document", "Node", "Element"]
// )]
mod my_module {
   use wasm_bindgen::prelude::*;
   
	#[wasm_bindgen]
	extern "C" {
		pub fn alert(s: &str);
	}

	#[wasm_bindgen]
	pub fn greet(name: &str) {
		unsafe {
			alert(&format!("Hello, {}!", name));
		}
	}
}

fn main() {}
