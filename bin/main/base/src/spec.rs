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

use serde::Deserialize;
use toml::Value;

#[derive(Deserialize, Debug)]
pub struct Spec {
	pub basic: Basic,
	pub genesis: Genesis,
}

#[derive(Deserialize, Debug)]
pub struct Basic {
	pub hash: String,
	pub dsa: String,
	pub address: String,
}

#[derive(Deserialize, Debug)]
pub struct Genesis {
	pub txs: Vec<Tx>,
}

#[derive(Deserialize, Debug)]
pub struct Tx {
	pub method: String,
	pub params: Vec<Value>,
}
