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
use std::path::PathBuf;
use std::sync::Arc;

use tempfile::tempdir;

use crypto::address::AddressImpl;
use crypto::dsa::DsaImpl;
use node_api::support::DefaultApiSupport;
use node_api::{Api, ApiConfig};
use node_chain::{module, Chain, ChainConfig};
use node_txpool::{TxPool, TxPoolConfig};
use primitives::{codec, Address, Transaction};
use utils_test::test_accounts;

#[tokio::test]
async fn test_api() {
	let config = ApiConfig {
		rpc_addr: "0.0.0.0:3109".to_string(),
		rpc_workers: 1,
		rpc_maxconn: 100,
	};

	let dsa = Arc::new(DsaImpl::Ed25519);
	let address = Arc::new(AddressImpl::Blake2b160);

	let (account1, _account2) = test_accounts(dsa, address);

	let chain = get_chain(&account1.3);

	let txpool_config = TxPoolConfig {
		pool_capacity: 32,
		buffer_capacity: 32,
	};

	let txpool = Arc::new(TxPool::new(txpool_config, chain.clone()).unwrap());

	let support = Arc::new(DefaultApiSupport::new(chain.clone(), txpool));

	let _api = Api::new(config, support);

	for (request, expected_response) in get_cases(&chain) {
		let mut res = surf::post("http://127.0.0.1:3109")
			.body(request)
			.send()
			.await
			.unwrap();
		let response = res.body_string().await.unwrap();
		assert_eq!(response, expected_response);
	}
}

fn get_cases(chain: &Arc<Chain>) -> Vec<(String, String)> {
	let tx = get_tx(chain);

	let tx_hash = chain.hash_transaction(&tx).unwrap();

	let tx_hex = hex::encode(codec::encode(&tx).unwrap());

	let tx_hash_hex = hex::encode(tx_hash.0);
	let tx_public_key_hex = hex::encode(&tx.witness.clone().unwrap().public_key.0);
	let tx_sig_hex = hex::encode(&tx.witness.clone().unwrap().signature.0);
	let nonce_hex = hex::encode(tx.witness.clone().unwrap().nonce.to_be_bytes());
	let until_hex = hex::encode(tx.witness.clone().unwrap().until.to_be_bytes());
	let params_hex = hex::encode(&tx.call.params.0);

	vec![
        (
            r#"{"jsonrpc": "2.0", "method": "chain_getBlockByNumber", "params": ["confirmed"], "id": 1}"#
                .to_string(),
            format!(r#"{{"jsonrpc":"2.0","result":{{"hash":"0x75f3a28816f3eab0079a734b4f275cd9a1c012d523c86203ee8bff08188f1318","header":{{"number":"0x0000000000000000","timestamp":"0x00000171c4eb7136","parent_hash":"0x0000000000000000000000000000000000000000000000000000000000000000","meta_txs_root":"0x41236f2124e3af09facc6908a9b2befbb3f0614b88e5cf580130cbe657043f29","meta_state_root":"0xaeda7d48b84f094ee0fd8e31aaf5b7d19995b613df19efccbb03e7ee5787ce12","meta_receipts_root":"0x46f793ce72de14b7eecae2b4524658a09cb9cbaa313cf9b27a0dd1274e7caf28","payload_txs_root":"0xc73b1740e53645a26e1926f9d910e560a99a601d14680deb6dc31eb26f321edc","payload_execution_gap":"0x00","payload_execution_state_root":"0x0000000000000000000000000000000000000000000000000000000000000000","payload_execution_receipts_root":"0x0000000000000000000000000000000000000000000000000000000000000000"}},"body":{{"meta_txs":["0x91b00eaf36abb6c89954e1ddc7933ed55373859de12bfdf24abc7c0abb327904"],"payload_txs":["0x6745417d545c3e0f7d610cadfd1ee8d450a92e89fa74bb75777950a779f2aa94"]}}}},"id":1}}"#, ),
        ),
        (
            r#"{"jsonrpc": "2.0", "method": "chain_getTransactionByHash", "params": ["0x91b00eaf36abb6c89954e1ddc7933ed55373859de12bfdf24abc7c0abb327904"], "id": 1}"#
                .to_string(),
            r#"{"jsonrpc":"2.0","result":{"hash":"0x91b00eaf36abb6c89954e1ddc7933ed55373859de12bfdf24abc7c0abb327904","witness":null,"call":{"module":"system","method":"init","params":"0x28636861696e2d746573743671ebc471010000140000000000000008"}},"id":1}"#.to_string(),
        ),
        (
            r#"{"jsonrpc": "2.0", "method": "chain_getRawTransactionByHash", "params": ["0x91b00eaf36abb6c89954e1ddc7933ed55373859de12bfdf24abc7c0abb327904"], "id": 1}"#.to_string(),
            r#"{"jsonrpc":"2.0","result":"0x001873797374656d10696e69747028636861696e2d746573743671ebc471010000140000000000000008","id":1}"#.to_string(),
        ),
        (
            r#"{"jsonrpc": "2.0", "method": "chain_getReceiptByHash", "params": ["0x91b00eaf36abb6c89954e1ddc7933ed55373859de12bfdf24abc7c0abb327904"], "id": 1}"#
                .to_string(),
            r#"{"jsonrpc":"2.0","result":{"hash":"0x91b00eaf36abb6c89954e1ddc7933ed55373859de12bfdf24abc7c0abb327904","block_number":"0x0000000000000000","events":[],"result":"0x0000"},"id":1}"#.to_string(),
        ),
        (
            format!(r#"{{"jsonrpc": "2.0", "method": "chain_sendRawTransaction", "params": ["0x{}"], "id": 1}}"#, tx_hex),
            format!(r#"{{"jsonrpc":"2.0","result":"0x{}","id":1}}"#, tx_hash_hex),
        ),
        (
            format!(r#"{{"jsonrpc": "2.0", "method": "chain_getTransactionInTxPool", "params": ["{}"], "id": 1}}"#, tx_hash_hex),
            format!(r#"{{"jsonrpc":"2.0","result":{{"hash":"0x{}","witness":{{"public_key":"0x{}","signature":"0x{}","nonce":"0x{}","until":"0x{}"}},"call":{{"module":"balance","method":"transfer","params":"0x{}"}}}},"id":1}}"#,
                    tx_hash_hex, tx_public_key_hex, tx_sig_hex, nonce_hex, until_hex, params_hex),
        ),
        (
            format!(r#"{{"jsonrpc": "2.0", "method": "chain_executeCall", "params": {{ "block_hash": "0x75f3a28816f3eab0079a734b4f275cd9a1c012d523c86203ee8bff08188f1318", "sender": "0xb4decd5a5f8f2ba708f8ced72eec89f44f3be96a", "call": {{ "module":"balance", "method":"get_balance", "params": "" }} }}, "id": 1}}"#),
            format!(r#"{{"jsonrpc":"2.0","result":"0x0a00000000000000","id":1}}"#),
        )
    ]
}

fn get_tx(chain: &Arc<Chain>) -> Transaction {
	let (account1, account2) = test_accounts(
		chain.get_basic().dsa.clone(),
		chain.get_basic().address.clone(),
	);

	let nonce = 0u32;
	let until = 1u64;
	let tx = chain
		.build_transaction(
			Some((account1.0, nonce, until)),
			"balance".to_string(),
			"transfer".to_string(),
			module::balance::TransferParams {
				recipient: account2.3,
				value: 2,
			},
		)
		.unwrap();
	chain.validate_transaction(&tx, true).unwrap();
	tx
}

fn get_chain(address: &Address) -> Arc<Chain> {
	let path = tempdir().expect("Could not create a temp dir");
	let home = path.into_path();

	init(&home, address);

	let chain_config = ChainConfig { home };

	let chain = Arc::new(Chain::new(chain_config).unwrap());

	chain
}

fn init(home: &PathBuf, address: &Address) {
	let config_path = home.join("config");

	fs::create_dir_all(&config_path).unwrap();

	let spec = format!(
		r#"
[basic]
hash = "blake2b_256"
dsa = "ed25519"
address = "blake2b_160"

[genesis]

[[genesis.txs]]
module = "system"
method = "init"
params = '''
{{
    "chain_id": "chain-test",
    "timestamp": "2020-04-29T15:51:36.502+08:00",
    "max_until_gap": 20,
    "max_execution_gap": 8
}}
'''

[[genesis.txs]]
module = "balance"
method = "init"
params = '''
{{
    "endow": [
    	["{}", 10]
    ]
}}
'''
	"#,
		address
	);

	fs::write(config_path.join("spec.toml"), &spec).unwrap();
}
