#![cfg(test)]

use alloy_primitives::hex;
use anyhow::Result;
use cannon_mipsevm::{
    load_elf, patch_stack, InstrumentedState, PreimageOracle,
};
use preimage_oracle::{Hint, Key, LocalIndexKey};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, io::BufWriter};

#[test]
fn test_simple_revm() {
    let elf_bytes = include_bytes!(
        "../../../examples/simple-revm/target/mips-unknown-none/release/simple-revm"
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
}

impl RevmTestOracle {
    pub fn new() -> Self {
        const INPUT: &[u8] = b"facade facade facade";
        let mut hasher = Sha256::new();
        hasher.update(INPUT);
        let input_hash = hasher.finalize();

        let mut images = HashMap::new();
        images.insert((0 as LocalIndexKey).preimage_key(), INPUT.to_vec());
        images.insert((1 as LocalIndexKey).preimage_key(), input_hash.to_vec());
        images.insert(
            (2 as LocalIndexKey).preimage_key(),
            hex!("365f5f37365ff3").to_vec(),
        );

        Self { images }
    }
}

impl PreimageOracle for RevmTestOracle {
    fn hint(&mut self, _: impl Hint) -> Result<()> {
        // no-op
        Ok(())
    }

    fn get(&mut self, key: [u8; 32]) -> Result<Vec<u8>> {
        dbg!(&key);
        Ok(self
            .images
            .get(&key)
            .ok_or(anyhow::anyhow!("No image for key"))?
            .to_vec())
    }
}
