#![cfg(test)]

use alloy_primitives::hex;
use anyhow::Result;
use cannon_mipsevm::{load_elf, patch_stack, InstrumentedState, PreimageOracle};
use preimage_oracle::{Hint, Key, LocalIndexKey};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io::BufWriter};

#[test]
fn test_simple_revm() {
    let elf_bytes = include_bytes!(
        "../../bin/cannon/simple-revm"
    );
    let mut state = load_elf(elf_bytes).unwrap();
    patch_stack(&mut state).unwrap();

    let out = BufWriter::new(Vec::default());
    let err = BufWriter::new(Vec::default());
    let mut ins = InstrumentedState::new(state, RevmTestOracle::new(), out, err);

    for _ in 0..2_000_000 {
        if ins.state.exited {
            break;
        }
        ins.step(false).unwrap();
    }

    assert!(ins.state.exited, "must exit");
    assert_eq!(ins.state.exit_code, 0, "must exit with 0");

    assert_eq!(
        String::from_utf8(ins.std_out().to_vec()).unwrap(),
        "Booting EVM and checking hash...\nSuccess, hashes matched!\n"
    );
    assert_eq!(String::from_utf8(ins.std_err().to_vec()).unwrap(), "");
}

pub struct RevmTestOracle {
    images: HashMap<[u8; 32], Vec<u8>>,
    sha2_preimages: HashMap<[u8; 32], Vec<u8>>,
}

impl RevmTestOracle {
    pub fn new() -> Self {
        const INPUT: &[u8] = b"facade facade facade";
        let mut hasher = Sha256::new();
        hasher.update(INPUT);
        let input_hash = hasher.finalize();

        let mut images = HashMap::new();
        images.insert((1 as LocalIndexKey).preimage_key(), input_hash.to_vec());
        images.insert(
            (2 as LocalIndexKey).preimage_key(),
            hex!("365f5f37365ff3").to_vec(),
        );

        let mut sha2_preimages = HashMap::new();
        sha2_preimages.insert(input_hash.try_into().unwrap(), INPUT.to_vec());

        Self {
            images,
            sha2_preimages,
        }
    }
}

impl PreimageOracle for RevmTestOracle {
    fn hint(&mut self, hint: impl Hint) -> Result<()> {
        let hint_str = std::str::from_utf8(hint.hint())?;
        let hint_parts = hint_str.split_whitespace().collect::<Vec<_>>();

        match hint_parts[0] {
            "sha2-preimage" => {
                let hash: [u8; 32] = hex::decode(hint_parts[1])?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Failed to parse hash"))?;
                self.images.insert(
                    (0 as LocalIndexKey).preimage_key(),
                    self.sha2_preimages
                        .get(&hash)
                        .ok_or(anyhow::anyhow!("No preimage for hash"))?
                        .to_vec(),
                );
                Ok(())
            }
            _ => anyhow::bail!("Unknown hint: {}", hint_str),
        }
    }

    fn get(&mut self, key: [u8; 32]) -> Result<Vec<u8>> {
        Ok(self
            .images
            .get(&key)
            .ok_or(anyhow::anyhow!("No image for key"))?
            .to_vec())
    }
}
