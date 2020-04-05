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

use std::fs;

use chrono::DateTime;
use tempfile::tempdir;
use toml::Value;

use base::spec::Spec;
use base::SharedParams;
use base::SystemInitParams;
use main_init::cli::InitOpt;
use main_init::run;

#[test]
fn test_init() {
	let home = tempdir().expect("could not create a temp dir");
	let home = home.into_path();

	// home should not exists
	fs::remove_dir(&home).unwrap();

	let opt = InitOpt {
		shared_params: SharedParams {
			home: Some(home.to_path_buf()),
		},
	};

	let result = run(opt);

	assert!(result.is_ok());

	let spec: Spec =
		toml::from_str(&fs::read_to_string(home.join("config").join("spec.toml")).unwrap())
			.unwrap();

	assert_eq!(spec.basic.hash, "blake2b_256");
	assert_eq!(spec.basic.dsa, "ed25519");
	assert_eq!(spec.basic.address, "blake2b_160");

	let tx = &spec.genesis.txs[0];

	assert_eq!(tx.method, "system.init");

	let param = &tx.params[0];

	let param = match param {
		Value::String(s) => s,
		_ => unreachable!("param should be string"),
	};

	let param: SystemInitParams = serde_json::from_str(param).unwrap();

	assert_eq!(param.chain_id.len(), 14);
	assert!(DateTime::parse_from_rfc3339(&param.time).is_ok());
}

#[cfg(feature = "build-dep-test")]
#[test]
fn test_init_command() {
	use assert_cmd::Command;

	let home = tempdir().expect("could not create a temp dir");
	let home = home.into_path();

	// home should not exists
	fs::remove_dir(&home).unwrap();

	let mut cmd = match Command::cargo_bin("wingchain") {
		Ok(cmd) => cmd,
		Err(e) => panic!(format!("should build first: {}", e)),
	};

	let output = cmd.arg("init").arg("--home").arg(&home).output().unwrap();

	assert_eq!(output.status.success(), true);

	let spec: Spec =
		toml::from_str(&fs::read_to_string(home.join("config").join("spec.toml")).unwrap())
			.unwrap();

	assert_eq!(spec.basic.hash, "blake2b_256");
	assert_eq!(spec.basic.dsa, "ed25519");
	assert_eq!(spec.basic.address, "blake2b_160");

	let tx = &spec.genesis.txs[0];

	assert_eq!(tx.method, "system.init");

	let param = &tx.params[0];

	let param = match param {
		Value::String(s) => s,
		_ => unreachable!("param should be string"),
	};

	let param: SystemInitParams = serde_json::from_str(param).unwrap();

	assert_eq!(param.chain_id.len(), 14);
	assert!(DateTime::parse_from_rfc3339(&param.time).is_ok());
}
