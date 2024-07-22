//! A simple program that takes a number `n` as input, and writes the `n-1`th and `n`th fibonacci
//! number as an output.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::{sol, SolType};
use kona_client::scenario::Scenario;
use kona_preimage::PreimageKey;
use revm::primitives::HashMap;

extern crate alloc;
use alloc::vec::Vec;

/// The public values encoded as a tuple that can be easily deserialized inside Solidity.
type PublicValuesTuple = sol! {
    tuple(uint32, uint32, uint32)
};

pub fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a custom system call which handles reading inputs
    // from the prover.
    let prebuilt_preimage = sp1_zkvm::io::read::<HashMap<PreimageKey, Vec<u8>>>();

    kona_common::block_on(async move {
    let mut client = Scenario::new(Some(prebuilt_preimage)).await.unwrap();
    let (attributes, l2_safe_head_header) = client.derive().await.unwrap();
    let number = client.execute_block(attributes, l2_safe_head_header).await.unwrap();
    let output_root = client.compute_output_root().await.unwrap();

    assert_eq!(number, client.boot.l2_claim_block);
    assert_eq!(output_root, client.boot.l2_claim);
    });
    // // Encocde the public values of the program.
    // let bytes = PublicValuesTuple::abi_encode(&(n, a, b));

    // // Commit to the public values of the program.
    // sp1_zkvm::io::commit_slice(&bytes);
}
