// Copyright 2019, 2020 Wingchain
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
#![feature(test)]

extern crate test;

use std::str::FromStr;
use test::{black_box, Bencher};

use assert_cmd::cargo::cargo_bin;

use crypto::hash::{Hash, HashImpl};

#[bench]
fn bench_hash_native(b: &mut Bencher) {
	let hash = HashImpl::Blake2b256;

	let data = (0..32u8).collect::<Vec<_>>();
	let mut out = [0u8; 32];

	b.iter(|| black_box(hash.hash(&mut out, &data)));
}

/// to run with dylib, should `cargo +nightly build --release` first.
#[bench]
fn bench_hash_dylib(b: &mut Bencher) {
	let path = cargo_bin(get_dylib("crypto_dylib_samples_hash"));

	assert!(path.exists());

	let path = path.to_string_lossy();
	let hasher = HashImpl::from_str(&path).unwrap();

	let data = (0..32u8).collect::<Vec<_>>();
	let mut out = [0u8; 32];

	b.iter(|| black_box(hasher.hash(&mut out, &data)));
}

#[cfg(target_os = "macos")]
fn get_dylib(package_name: &str) -> String {
	format!("lib{}.dylib", package_name)
}

#[cfg(target_os = "linux")]
fn get_dylib(package_name: &str) -> String {
	format!("lib{}.so", package_name)
}

#[cfg(target_os = "windows")]
fn get_dylib(package_name: &str) -> String {
	format!("{}.dll", package_name)
}
