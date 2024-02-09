#![no_std]
#![no_main]

use alloc::vec::Vec;
use anyhow::Result;
use kona_common::io::{self, FileDescriptor};
use kona_preimage::{oracle_reader, PreimageKey, PreimageKeyType};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{address, ExecutionResult, Output, TransactTo},
    Evm,
};

extern crate alloc;

const HEAP_SIZE: usize = 0xFFFFFFF;

#[no_mangle]
pub extern "C" fn _start() {
    kona_common::alloc_heap!(HEAP_SIZE);

    let (input, digest) = boot().expect("Failed to boot");
    run_evm(input, digest).expect("EVM execution failed");

    io::exit(0)
}

/// Boot the program and load bootstrap information.
fn boot() -> Result<(Vec<u8>, [u8; 32])> {
    let mut oracle = oracle_reader();
    let input = oracle.get(PreimageKey::new([1u8; 32], PreimageKeyType::Local))?;
    // let mut digest_key = [0u8; 32];
    // digest_key[31] = 1;
    // let digest = oracle
    //     .get(PreimageKey::new(digest_key, PreimageKeyType::Local))
    //     .unwrap()
    //     .try_into()
    //     .unwrap();
    io::write(
        FileDescriptor::StdOut,
        alloc::format!("Input (len = {}): {:x?}", input.len(), input).as_bytes(),
    )?;

    Ok((input, [0u8; 32]))
}

/// Call the SHA-256 precompile and assert that the input and output match the expected values
fn run_evm(input: alloc::vec::Vec<u8>, digest: [u8; 32]) -> Result<()> {
    let cache_db = CacheDB::new(EmptyDB::default());
    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.caller = address!("0000000000000000000000000000000000000000");
            tx.transact_to = TransactTo::Call(address!("0000000000000000000000000000000000000002"));
            tx.data = input.into();
        })
        .build();

    // execute transaction without writing to the DB
    let ref_tx = evm
        .transact()
        .map_err(|_| anyhow::anyhow!("Failed state transition"))?;
    // select ExecutionResult struct
    let result = ref_tx.result;

    // unpack output call enum into raw bytes
    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        _ => anyhow::bail!("EVM Execution failed"),
    };

    if value.as_ref() != digest.as_ref() {
        let _ = io::write(
            FileDescriptor::StdErr,
            alloc::format!("Expected: {:x?} | Got: {:x?}\n", digest, value).as_bytes(),
        );
        io::exit(1);
    }

    Ok(())
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    let _ = io::write(FileDescriptor::StdErr, msg.as_bytes());
    io::exit(2)
}
