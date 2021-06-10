#![feature(string_remove_matches)]

//! A library for embedding high-performance WASM code directly in a Rust program.
//! This package was created for people who absolutely hate writing Javascript.
//! The goal of this library is to reduce the amount of overhead required to implement
//! WASM by automatically compiling WASM modules and statically linking them to
//! your binary. You will need to have [wasm-bindgen](https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_wasm) installed.
//! If your project stops building, please submit an issue. You may also try deleting .wasmir directory.

//! # Usage
//! Code must be declared inside a module. The typical usage is as follows:
//! ```
//! use wasmir::wasmir;
//!
//! #[wasmir]
//! mod my_module {
//!    use wasm_bindgen::prelude::*;
//!
//!    #[wasm_bindgen]
//!    extern "C" {
//!       pub fn alert(s: &str);
//!    }
//!
//!    #[wasm_bindgen]
//!    pub fn greet(name: &str) {
//!       unsafe {
//!          alert(&format!("Hello, {}!", name));
//!       }
//!    }
//! }
//! ```
//! Once the proc_macro does its work, the above module will then contain two binary blob constants,
//! `wasm` and `loader`. Serve loader from "my_module.js" and wasm from "my_module_bg.wasm"
//! Then, in index.js, include the following code:
//! ```js
//! import init from './my_module_bg.js';
//! import {greet} from './my_module_bg.js';
//!
//! function run() {
//!    greet(\"World\");
//! }
//!
//! init().then(run)
//! ```
//! You can also specify WASM-dependencies like so:
//! ```toml
//! #[wasmir(
//! [dependencies]
//! wasm-bindgen = "*"
//! [dependencies.web-sys]
//! version = "*"
//! features = ["Document", "Node", "Element"]
//! )]
//! ```

// Macro gets applied to module, function, struct, etc.
// Macro calls compiler with web assembly target on code.
// Macro puts the resulting binary in the code.

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use std::env;
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::prelude::*;
use std::process::Command;
use toml;
use toml::Value;

fn token_tree_to_toml(tree: TokenTree, prev: &Option<TokenTree>) -> String {
	let mut buf = String::new();
	let newline_group = if let Some(TokenTree::Punct(_)) = prev {
		""
	} else {
		"\n"
	};
	buf = match tree {
		TokenTree::Group(group) => match group.delimiter() {
			Delimiter::Brace => format![
				"{}{}{{{}}}",
				newline_group,
				buf,
				token_stream_to_toml(group.stream())
			],
			Delimiter::Bracket => format![
				"{}{}[{}]",
				newline_group,
				buf,
				token_stream_to_toml(group.stream())
			],
			Delimiter::Parenthesis => {
				format![
					"{}{}({})",
					newline_group,
					buf,
					token_stream_to_toml(group.stream())
				]
			}
			Delimiter::None => format![
				"{}{}{}",
				newline_group,
				buf,
				token_stream_to_toml(group.stream())
			],
		},
		TokenTree::Ident(ident) => {
			if let Some(TokenTree::Group(_) | TokenTree::Literal(_)) = prev {
				format!["\n{}{}", buf, ident.to_string()]
			} else {
				format!["{}{}", buf, ident.to_string()]
			}
		}
		TokenTree::Literal(literal) => {
			if let Some(TokenTree::Group(_)) = prev {
				format!["\n{}{}", buf, literal.to_string()]
			} else {
				format!["{}{}", buf, literal.to_string()]
			}
		}
		TokenTree::Punct(punct) => {
			if let Some(TokenTree::Group(_)) = prev {
				format!["\n{}{}", buf, punct.to_string()]
			} else {
				format!["{}{}", buf, punct.to_string()]
			}
		}
	};
	buf
}

fn token_stream_to_toml(tokens: TokenStream2) -> String {
	let mut buf = String::new();
	let mut prev = None;

	for token in tokens.into_iter() {
		buf = format!["{}{}", buf, token_tree_to_toml(token.clone(), &prev)];
		prev = Some(token);
	}

	buf
}

#[proc_macro_attribute]
pub fn wasmir(attr: TokenStream, input: TokenStream) -> TokenStream {
	let project_root = std::path::PathBuf::from(
		std::env::var("CARGO_MANIFEST_DIR")
			.expect("couldn't read CARGO_MANIFEST_DIR environment variable"),
	);
	let wasmir_dir = std::path::PathBuf::from(project_root.clone()).join(".wasmir");
	create_dir_all(wasmir_dir.clone()).expect("couldn't create WASMIR temp directory");

	let attr = TokenStream2::from(attr);
	let dependencies = token_stream_to_toml(attr);
	println!["{}", dependencies];

	let input = TokenStream2::from(input);
	let mut module_name = String::new();
	let mut module_stream: TokenStream2 = TokenStream2::new();

	for item in input.clone().into_iter() {
		match item {
			TokenTree::Ident(ident) => match ident.to_string().as_str() {
				"pub" => {
					continue;
				}
				"mod" => {
					continue;
				}
				name => {
					module_name = name.to_string();
				}
			},
			TokenTree::Group(group) => {
				module_stream = group.stream();
				break;
			}
			_ => {
				continue;
			}
		}
	}

	let module_text = module_stream.to_string();
	let module_root = wasmir_dir.join(module_name.clone());
	// Create the module dir
	env::set_current_dir(wasmir_dir.clone()).expect("could not set current directory");
	match Command::new("cargo")
		.arg("new")
		.arg("--lib")
		.arg(module_name.clone())
		.output()
	{
		Ok(o) => {
			println!["{}", String::from_utf8(o.stderr).unwrap()];
		}
		Err(_) => {}
	};

	// attempt to write to lib.rs in module root
	let mut file = File::create(module_root.join("src").join("lib.rs"))
		.expect("could not open lib.rs for editing");
	let buf: Vec<u8> = module_text.as_bytes().iter().map(|b| *b).collect();
	file.write_all(&buf).expect("could not write to lib.rs");

	// Configure the module
	let mut buf = String::new();
	let mut file = OpenOptions::new()
		.write(true)
		.read(true)
		.open(module_root.join("Cargo.toml"))
		.expect("no Cargo.toml in module root");

	file.read_to_string(&mut buf)
		.expect("failed to read from Cargo.toml");

	let mut cargo_toml: toml::Value =
		toml::from_str(&mut buf.as_str()).expect("failed to parse toml for module");

	let cdylib: Value = Value::Array(vec![toml::Value::String("cdylib".to_string())]);

	match cargo_toml.get_mut("lib") {
		Some(Value::Table(lib)) => {
			lib.insert(
				"crate-type".to_string(),
				Value::Array(vec![Value::String("cdylib".to_string())]),
			);
		}
		_ => {
			if let Some(table) = cargo_toml.as_table_mut() {
				let mut map = toml::map::Map::new();
				map.insert("crate-type".to_string(), cdylib);
				let map = Value::Table(map);
				table.insert("lib".to_string(), map);
			}
		}
	}

	let wasm_bindgen_dep: Value = Value::String("*".to_string());

	match cargo_toml.get_mut("dependencies") {
		Some(Value::Table(lib)) => {
			lib.insert("wasm-bindgen".to_string(), Value::String("*".to_string()));
		}
		_ => {
			if let Some(table) = cargo_toml.as_table_mut() {
				let mut map = toml::map::Map::new();
				map.insert("wasm-bindgen".to_string(), wasm_bindgen_dep);
				let map = Value::Table(map);
				table.insert("dependencies".to_string(), map);
			}
		}
	}

	let mut dependencies_toml: Value =
		toml::from_str(&dependencies).expect("failed to parse dependencies toml");
	match dependencies_toml.get_mut("dependencies") {
		Some(Value::Table(deps)) => {
			if let Some(Value::Table(lib_deps)) = cargo_toml.get_mut("dependencies") {
				lib_deps.extend(deps.iter().map(|(k, v)| (k.clone(), v.clone())));
			}
		}
		_ => {}
	}

	let mut file =
		File::create(module_root.join("Cargo.toml")).expect("failed to write toml/create file");
	file.write(&format!["{}", cargo_toml].bytes().collect::<Vec<u8>>())
		.expect("failed to write to Cargo.toml");

	// Build the module using `wasm-pack build --target web`
	env::set_current_dir(module_root.clone()).expect("could not set current directory");
	match Command::new("wasm-pack")
		.arg("build")
		.arg("--target")
		.arg("web")
		.output()
	{
		Ok(o) => {
			println!["{}", String::from_utf8(o.stderr).unwrap()];
		}
		Err(e) => {
			panic!["could not build: {}", e];
		}
	}

	let mut file = match File::open(
		module_root
			.join("pkg")
			.join(format!["{}_bg.wasm", module_name.clone()]),
	) {
		Ok(file) => file,
		Err(e) => panic!["could not open binary: {}", e],
	};

	let mut binary = vec![];

	file.read_to_end(&mut binary)
		.expect("could not read-in binary");

	let binary_len = binary.len();

	let mut file = match File::open(
		module_root
			.join("pkg")
			.join(format!["{}.js", module_name.clone()]),
	) {
		Ok(file) => file,
		Err(e) => panic!["could not open js: {}", e],
	};

	let mut js = String::new();

	file.read_to_string(&mut js).expect("could not read-in js");

	let module_name = Ident::new(module_name.as_str(), Span::call_site());

	quote![
	 mod #module_name {
	   #input
		pub const wasm: [u8; #binary_len] = [#(#binary),*];
		pub const loader: &str = #js;
	}]
	.into()
}
