# wasmir

A library for embedding high-performance WASM code directly in a Rust program.
This package was created for people who absolutely hate writing Javascript.
The goal of this library is to reduce the amount of overhead required to implement
WASM by automatically compiling WASM modules and statically linking them to
the your binary. You will need to have [wasm-bindgen](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm) installed.
If your project stops building, please submit an issue.


Add wasmir as a dependency to your Cargo.toml:

```toml
wasmir = "0.1.1"
```

# Usage
Code must be declared inside a module. The typical usage is as follows:
```rs
use wasmir::wasmir;

#[wasmir]
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
```
Once the proc_macro does its work, the above module will then contain two binary blob constants,
`wasm` and `js_loader`. It is important that `js_loader` is served with your web app (inside index.html) and that the
wasm binary is served at the correct address of "./my_module_bg.wasm"
