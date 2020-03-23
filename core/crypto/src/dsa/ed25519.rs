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

use std::convert::TryInto;

use rand::thread_rng;
use rand::Rng;
use ring::signature::{Ed25519KeyPair, KeyPair, VerificationAlgorithm, ED25519};
use untrusted::Input;

use crate::dsa::{Dsa, KeyPair as KeyPairT, Verifier as VerifierT};
use crate::errors;
use crate::errors::ResultExt;

pub struct Ed25519;

pub struct Verifier([u8; 32]);

impl Dsa for Ed25519 {
	type Error = errors::Error;

	type KeyPair = (Ed25519KeyPair, [u8; 32]);

	type Verifier = Verifier;

	fn name(&self) -> String {
		"ed25519".to_string()
	}

	fn generate_key_pair(&self) -> errors::Result<Self::KeyPair> {
		let seed = random_32_bytes(&mut thread_rng());

		let key_pair = Ed25519KeyPair::from_seed_unchecked(&seed)
			.chain_err(|| errors::ErrorKind::InvalidSecretKey)?;
		let key_pair = (key_pair, seed);

		Ok(key_pair)
	}

	fn key_pair_from_secret_key(&self, secret_key: &[u8]) -> errors::Result<Self::KeyPair> {
		let key_pair = Ed25519KeyPair::from_seed_unchecked(&secret_key)
			.chain_err(|| errors::ErrorKind::InvalidSecretKey)?;
		let seed = secret_key.try_into().expect("qed");
		let key_pair = (key_pair, seed);
		Ok(key_pair)
	}

	fn verifier_from_public_key(&self, public_key: &[u8]) -> errors::Result<Self::Verifier> {
		let verifier = Verifier(
			public_key
				.try_into()
				.chain_err(|| errors::ErrorKind::InvalidPublicKey)?,
		);
		Ok(verifier)
	}
}

impl KeyPairT for (Ed25519KeyPair, [u8; 32]) {
	fn public_key(&self) -> Vec<u8> {
		self.0.public_key().as_ref().to_vec()
	}
	fn secret_key(&self) -> Vec<u8> {
		self.1.to_vec()
	}
	fn sign(&self, message: &[u8]) -> Vec<u8> {
		let signature = self.0.sign(&message);

		let signature = signature.as_ref().to_vec();

		signature
	}
}

impl VerifierT for Verifier {
	type Error = errors::Error;

	fn verify(&self, message: &[u8], signature: &[u8]) -> errors::Result<()> {
		let result = ED25519
			.verify(
				Input::from(&self.0[..]),
				Input::from(&message),
				Input::from(&signature),
			)
			.chain_err(|| errors::ErrorKind::VerificationFailed)?;

		Ok(result)
	}
}

fn random_32_bytes<R: Rng + ?Sized>(rng: &mut R) -> [u8; 32] {
	let mut ret = [0u8; 32];
	rng.fill_bytes(&mut ret);
	ret
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_ed25519_generate_key_pair() {
		assert!(Ed25519.generate_key_pair().is_ok());
	}

	#[test]
	fn test_ed25519_key_pair_from_secret_key() {
		let secret: [u8; 32] = [
			184, 80, 22, 77, 31, 238, 200, 105, 138, 204, 163, 41, 148, 124, 152, 133, 189, 29,
			148, 3, 77, 47, 187, 230, 8, 5, 152, 173, 190, 21, 178, 152,
		];
		assert!(Ed25519.key_pair_from_secret_key(&secret).is_ok());
	}

	#[test]
	fn test_ed25519_key_pair_from_secret_key_chain_err() {
		use error_chain::ChainedError;
		let secret: [u8; 31] = [
			184, 80, 22, 77, 31, 238, 200, 105, 138, 204, 163, 41, 148, 124, 152, 133, 189, 29,
			148, 3, 77, 47, 187, 230, 8, 5, 152, 173, 190, 21, 178,
		];
		let err: errors::Error = Ed25519.key_pair_from_secret_key(&secret).unwrap_err();
		assert_eq!(
			err.display_chain().to_string(),
			"Error: Invalid public key\nCaused by: InvalidEncoding\n"
		);
	}

	#[test]
	fn test_ed25519_key_pair() {
		let secret: [u8; 32] = [
			184, 80, 22, 77, 31, 238, 200, 105, 138, 204, 163, 41, 148, 124, 152, 133, 189, 29,
			148, 3, 77, 47, 187, 230, 8, 5, 152, 173, 190, 21, 178, 152,
		];
		let key_pair = Ed25519.key_pair_from_secret_key(&secret).unwrap();

		let public_key = key_pair.public_key();

		assert_eq!(
			public_key,
			vec![
				137, 44, 137, 164, 205, 99, 29, 8, 218, 49, 70, 7, 34, 56, 20, 119, 86, 4, 83, 90,
				5, 245, 14, 149, 157, 33, 32, 157, 1, 116, 14, 186
			]
		);

		let message: Vec<u8> = vec![97, 98, 99];

		let signature = key_pair.sign(&message);

		assert_eq!(
			signature,
			vec![
				82, 19, 26, 105, 235, 178, 54, 112, 61, 224, 195, 88, 150, 137, 32, 46, 235, 209,
				209, 108, 64, 153, 12, 58, 216, 179, 88, 38, 49, 167, 162, 103, 219, 116, 93, 187,
				145, 86, 216, 98, 97, 135, 228, 15, 66, 246, 207, 232, 132, 182, 211, 206, 12, 220,
				4, 96, 58, 254, 237, 8, 151, 3, 172, 14
			]
		);

		let verifier = Ed25519.verifier_from_public_key(&public_key).unwrap();

		let result = verifier.verify(&message, &signature);

		assert!(result.is_ok());
	}
}
