# wasmir

A library for embedding high-performance WASM code directly in a Rust program.
This package was created for people who absolutely hate writing Javascript.
The goal of this library is to reduce the amount of overhead required to implement
WASM by automatically compiling WASM modules and statically linking them to
your binary. You will need to have [wasm-bindgen](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm) installed.
If your project stops building, please submit an issue.

# Usage
Add wasmir as a dependency to your Cargo.toml:
```toml
wasmir = "0.1.13"
```

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
`wasm` and `loader`. Serve loader from "my_module.js" and wasm from "my_module_bg.wasm"
Then, in index.js, include the following code:
```js
import init from './my_module_bg.js';
import {greet} from './my_module_bg.js';

function run() {
   greet(\"World\");
}

init().then(run)
```
You can also specify WASM-dependencies like so:
```toml
#[wasmir(
[dependencies]
wasm-bindgen = "*"
[dependencies.web-sys]
version = "*"
features = ["Document", "Node", "Element"]
)]
```