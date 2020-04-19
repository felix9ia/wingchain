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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use node_db::DBTransaction;
use node_executor_macro::dispatcher;
use node_executor_primitives::{Context as ContextT, Module as ModuleT};
use node_statedb::{StateDB, StateDBGetter, StateDBStmt, TrieRoot};
use primitives::{codec, errors::CommonResult};
use primitives::{BlockNumber, Call, DBKey, DBValue, Hash, Params, Transaction};
use serde::Serialize;

pub mod errors;

const META_TXS_SIZE: usize = 64;
const PAYLOAD_TXS_SIZE: usize = 512;

#[derive(Clone)]
pub struct Context {
	inner: Rc<ContextInner>,
}

struct ContextInner {
	#[allow(dead_code)]
	number: BlockNumber,
	#[allow(dead_code)]
	timestamp: u32,
	trie_root: Arc<TrieRoot>,
	meta_statedb: Arc<StateDB>,
	meta_state_root: Hash,
	meta_state: ContextState,
	meta_txs: RefCell<Vec<Arc<Transaction>>>,
	payload_statedb: Arc<StateDB>,
	payload_state_root: Hash,
	payload_state: ContextState,
	payload_txs: RefCell<Vec<Arc<Transaction>>>,
	// to mark the context has already started to executed payload txs
	payload_phase: Cell<bool>,
}

impl Context {
	pub fn new(
		number: BlockNumber,
		timestamp: u32,
		trie_root: Arc<TrieRoot>,
		meta_statedb: Arc<StateDB>,
		meta_state_root: Hash,
		payload_statedb: Arc<StateDB>,
		payload_state_root: Hash,
	) -> CommonResult<Self> {
		let meta_state = ContextState::new(meta_statedb.clone(), &meta_state_root.0)?;
		let payload_state = ContextState::new(payload_statedb.clone(), &payload_state_root.0)?;

		let inner = Rc::new(ContextInner {
			number,
			timestamp,
			trie_root,
			meta_statedb,
			meta_state_root,
			meta_state,
			meta_txs: RefCell::new(Vec::with_capacity(META_TXS_SIZE)),
			payload_statedb,
			payload_state_root,
			payload_state,
			payload_txs: RefCell::new(Vec::with_capacity(PAYLOAD_TXS_SIZE)),
			payload_phase: Cell::new(false),
		});

		Ok(Self { inner })
	}

	pub fn get_meta_update(&self) -> CommonResult<(Hash, DBTransaction)> {
		let buffer = self.inner.meta_state.buffer.borrow();
		let (root, transaction) = self
			.inner
			.meta_statedb
			.prepare_update(&self.inner.meta_state_root.0, buffer.iter())?;
		Ok((Hash(root), transaction))
	}

	pub fn get_meta_txs(&self) -> CommonResult<(Hash, Vec<Arc<Transaction>>)> {
		let txs = self.inner.meta_txs.borrow().clone();

		let input = txs
			.iter()
			.map(|x| codec::encode(&**x))
			.collect::<Result<Vec<Vec<u8>>, _>>()?;
		let txs_root = self.inner.trie_root.calc_ordered_trie_root(input);

		Ok((Hash(txs_root), txs))
	}

	pub fn get_payload_update(&self) -> CommonResult<(Hash, DBTransaction)> {
		let buffer = self.inner.payload_state.buffer.borrow();
		let (root, transaction) = self
			.inner
			.payload_statedb
			.prepare_update(&self.inner.payload_state_root.0, buffer.iter())?;
		Ok((Hash(root), transaction))
	}

	pub fn get_payload_txs(&self) -> CommonResult<(Hash, Vec<Arc<Transaction>>)> {
		let txs = self.inner.payload_txs.borrow().clone();

		let input = txs
			.iter()
			.map(|x| codec::encode(&**x))
			.collect::<Result<Vec<Vec<u8>>, _>>()?;
		let txs_root = self.inner.trie_root.calc_ordered_trie_root(input);

		Ok((Hash(txs_root), txs))
	}
}

struct ContextState {
	#[allow(dead_code)]
	/// statedb_stmt is referred by statedb_getter, should be kept
	statedb_stmt: StateDBStmt,
	/// unsafe, should never out live lib
	statedb_getter: StateDBGetter<'static>,
	buffer: RefCell<HashMap<DBKey, Option<DBValue>>>,
}

impl ContextState {
	fn new(statedb: Arc<StateDB>, state_root: &[u8]) -> CommonResult<Self> {
		let statedb_stmt = statedb.prepare_stmt(state_root)?;
		let statedb_getter = StateDB::prepare_get(&statedb_stmt)?;
		let buffer = Default::default();

		let statedb_getter = unsafe {
			std::mem::transmute::<StateDBGetter<'_>, StateDBGetter<'static>>(statedb_getter)
		};

		Ok(ContextState {
			statedb_stmt,
			statedb_getter,
			buffer,
		})
	}
}

impl ContextT for Context {
	fn meta_get(&self, key: &[u8]) -> CommonResult<Option<DBValue>> {
		let buffer = self.inner.meta_state.buffer.borrow();
		match buffer.get(&DBKey::from_slice(key)) {
			Some(value) => Ok(value.clone()),
			None => self.inner.meta_state.statedb_getter.get(key),
		}
	}
	fn meta_set(&self, key: &[u8], value: Option<DBValue>) -> CommonResult<()> {
		let mut buffer = self.inner.meta_state.buffer.borrow_mut();
		buffer.insert(DBKey::from_slice(key), value);
		Ok(())
	}
	fn payload_get(&self, key: &[u8]) -> CommonResult<Option<DBValue>> {
		let buffer = self.inner.payload_state.buffer.borrow();
		match buffer.get(&DBKey::from_slice(key)) {
			Some(value) => Ok(value.clone()),
			None => self.inner.payload_state.statedb_getter.get(key),
		}
	}
	fn payload_set(&self, key: &[u8], value: Option<DBValue>) -> CommonResult<()> {
		let mut buffer = self.inner.payload_state.buffer.borrow_mut();
		buffer.insert(DBKey::from_slice(key), value);
		Ok(())
	}
}

pub struct Executor;

impl Executor {
	pub fn new() -> Self {
		Self
	}

	pub fn build_tx<P: Serialize>(
		&self,
		module: String,
		method: String,
		params: P,
	) -> CommonResult<Transaction> {
		let params = Params(codec::encode(&params)?);

		let call = Call {
			module: module.clone(),
			method: method,
			params,
		};

		let valid = Dispatcher::is_valid_call::<Context>(&module, &call)?;

		if !valid {
			return Err(errors::ErrorKind::InvalidTxCall.into());
		}

		Ok(Transaction {
			witness: None,
			call,
		})
	}

	pub fn validate_tx(&self, tx: &Transaction) -> CommonResult<()> {
		// TODO validate witness

		let module = &tx.call.module;
		let call = &tx.call;

		let valid = Dispatcher::is_valid_call::<Context>(module, &call)?;

		let write = Dispatcher::is_write_call::<Context>(module, &call)?;

		if !(valid && write == Some(true)) {
			return Err(errors::ErrorKind::InvalidTxCall.into());
		}

		Ok(())
	}

	pub fn execute_txs(&self, context: &Context, txs: Vec<Arc<Transaction>>) -> CommonResult<()> {
		let mut txs_is_meta: Option<bool> = None;
		for tx in &txs {
			let call = &tx.call;
			let module = &call.module;

			let is_meta = Dispatcher::is_meta::<Context>(module)?;
			match txs_is_meta {
				None => {
					txs_is_meta = Some(is_meta);
				}
				Some(txs_is_meta) => {
					if txs_is_meta != is_meta {
						return Err(errors::ErrorKind::InvalidTxs(
							"mixed meta and payload in one txs batch".to_string(),
						)
						.into());
					}
				}
			}

			let _result = Dispatcher::execute_call::<Context>(module, context, &call)?;
		}

		if context.inner.payload_phase.get() && txs_is_meta == Some(true) {
			return Err(errors::ErrorKind::InvalidTxs(
				"meta after payload not allowed".to_string(),
			)
			.into());
		}

		let mut txs = txs;
		match txs_is_meta {
			Some(true) => context.inner.meta_txs.borrow_mut().append(&mut txs),
			Some(false) => context.inner.payload_txs.borrow_mut().append(&mut txs),
			_ => (),
		}

		Ok(())
	}
}

#[allow(non_camel_case_types)]
#[dispatcher]
enum Dispatcher {
	system,
}

/// re-import modules
pub mod module {
	pub use module_system as system;
}
