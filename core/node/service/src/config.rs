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
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::sync::Arc;

use crypto::dsa::{Dsa, DsaImpl, KeyPair};
use main_base::config::Config as FileConfig;
use node_api::ApiConfig;
use node_chain::Basic;
use node_consensus_poa::PoaConfig;
use node_coordinator::{
	ed25519, CoordinatorConfig, Keypair, LinkedHashMap, Multiaddr, PeerId, Protocol,
};
use node_txpool::TxPoolConfig;
use primitives::errors::CommonResult;
use primitives::SecretKey;

use crate::errors::ErrorKind;
use crate::{errors, ServiceConfig};

pub struct Config {
	pub txpool: TxPoolConfig,
	pub api: ApiConfig,
	pub poa: PoaConfig,
	pub coordinator: CoordinatorConfig,
}

pub fn get_config(chain_config: &ServiceConfig, basic: Arc<Basic>) -> CommonResult<Config> {
	let home = &chain_config.home;
	let agent_version = &chain_config.agent_version;

	let file_config = get_file_config(home)?;
	let config = Config {
		txpool: get_txpool_config(&file_config)?,
		api: get_api_config(&file_config)?,
		poa: get_poa_config(&file_config, home, basic)?,
		coordinator: get_coordinator_config(&file_config, home, agent_version)?,
	};
	Ok(config)
}

fn get_txpool_config(file_config: &FileConfig) -> CommonResult<TxPoolConfig> {
	let txpool = TxPoolConfig {
		pool_capacity: file_config.txpool.pool_capacity,
		buffer_capacity: file_config.txpool.buffer_capacity,
	};
	Ok(txpool)
}

fn get_api_config(file_config: &FileConfig) -> CommonResult<ApiConfig> {
	let api = ApiConfig {
		rpc_addr: file_config.api.rpc_addr.clone(),
		rpc_workers: file_config.api.rpc_workers,
		rpc_maxconn: file_config.api.rpc_maxconn,
	};
	Ok(api)
}

fn get_poa_config(
	file_config: &FileConfig,
	home: &PathBuf,
	basic: Arc<Basic>,
) -> CommonResult<PoaConfig> {
	let secret_key = {
		let file = &file_config.validator.secret_key_file;
		let secret_key = read_secret_key_file(file, home)?;
		let _key_pair = basic
			.dsa
			.key_pair_from_secret_key(&secret_key)
			.map_err(|_| errors::ErrorKind::Config(format!("Invalid secret key in: {:?}", file)))?;
		SecretKey(secret_key)
	};

	let poa = PoaConfig { secret_key };
	Ok(poa)
}

fn get_coordinator_config(
	file_config: &FileConfig,
	home: &PathBuf,
	agent_version: &str,
) -> CommonResult<CoordinatorConfig> {
	let listen_addresses = parse_from_socket_addresses(&file_config.network.listen_addresses)?;
	let external_addresses = parse_from_socket_addresses(&file_config.network.external_addresses)?;
	let bootnodes = parse_from_multi_addresses(&file_config.network.bootnodes)?;
	let reserved_nodes = parse_from_multi_addresses(&file_config.network.reserved_nodes)?;

	let local_key_pair = {
		let file = &file_config.network.secret_key_file;
		let mut secret_key = read_secret_key_file(file, home)?;
		let dsa = DsaImpl::Ed25519;
		let key_pair = dsa.key_pair_from_secret_key(&secret_key)?;
		let (_, public_key_len, _) = dsa.length().into();
		let mut public_key = vec![0u8; public_key_len];
		key_pair.public_key(&mut public_key);
		secret_key.extend(public_key);
		let key_pair = ed25519::Keypair::decode(&mut secret_key[..])
			.map_err(|_| errors::ErrorKind::Config(format!("Invalid secret key in: {:?}", file)))?;
		Keypair::Ed25519(key_pair)
	};

	let network_config = node_coordinator::NetworkConfig {
		max_in_peers: file_config.network.max_in_peers,
		max_out_peers: file_config.network.max_out_peers,
		listen_addresses,
		external_addresses,
		bootnodes,
		reserved_nodes,
		reserved_only: file_config.network.reserved_only,
		agent_version: agent_version.to_string(),
		local_key_pair,
		handshake: vec![],
	};

	let config = CoordinatorConfig { network_config };
	Ok(config)
}

fn get_file_config(home: &PathBuf) -> CommonResult<FileConfig> {
	let config_path = home.join(main_base::CONFIG).join(main_base::CONFIG_FILE);
	let config = fs::read_to_string(&config_path).map_err(|_| {
		errors::ErrorKind::Config(format!("Failed to read config file: {:?}", config_path))
	})?;

	let config = toml::from_str(&config)
		.map_err(|e| errors::ErrorKind::Config(format!("Failed to parse config file: {:?}", e)))?;

	Ok(config)
}

fn parse_from_socket_addresses(
	addresses: &Vec<String>,
) -> CommonResult<LinkedHashMap<Multiaddr, ()>> {
	let addresses = addresses.iter().map(|x| -> CommonResult<Multiaddr> {
		let addr: SocketAddrV4 = x
			.parse()
			.map_err(|_| ErrorKind::Config(format!("Invalid socket address: {:?}", x)))?;
		let addr = Multiaddr::empty()
			.with(Protocol::Ip4(addr.ip().clone()))
			.with(Protocol::Tcp(addr.port()));
		Ok(addr)
	});
	let mut result = LinkedHashMap::new();
	for address in addresses {
		result.insert(address?, ());
	}
	Ok(result)
}

fn parse_from_multi_addresses(
	addresses: &Vec<String>,
) -> CommonResult<LinkedHashMap<(PeerId, Multiaddr), ()>> {
	let addresses = addresses
		.iter()
		.map(|x| -> CommonResult<(PeerId, Multiaddr)> {
			let mut addr: Multiaddr = x
				.parse()
				.map_err(|_| ErrorKind::Config(format!("Invalid multi address: {:?}", x)))?;
			let peer_id = match addr.pop() {
				Some(Protocol::P2p(key)) => PeerId::from_multihash(key)
					.map_err(|_| ErrorKind::Config(format!("Invalid multi address: {:?}", x)))?,
				_ => {
					return Err(ErrorKind::Config(format!("Invalid multi address: {:?}", x)).into())
				}
			};
			Ok((peer_id, addr))
		});
	let mut result = LinkedHashMap::new();
	for address in addresses {
		result.insert(address?, ());
	}
	Ok(result)
}

fn read_secret_key_file(file: &PathBuf, home: &PathBuf) -> CommonResult<Vec<u8>> {
	let file = {
		let file = if file.starts_with("/") {
			file.clone()
		} else {
			home.join(main_base::CONFIG).join(file)
		};
		file
	};

	let secret_key = {
		let secret_key = fs::read_to_string(&file).map_err(|_| {
			errors::ErrorKind::Config(format!("Failed to read secret key file: {:?}", file))
		})?;
		let secret_key = hex::decode(secret_key.trim())
			.map_err(|_| errors::ErrorKind::Config(format!("Invalid secret key in: {:?}", file)))?;
		secret_key
	};

	Ok(secret_key)
}
