#![no_std]
#![no_main]

use alloc::vec::Vec;
use anyhow::{anyhow, bail, Result};
use kona_common::io;
use kona_preimage::{client_preimage_pipe_handle, PreimageKey, PreimageKeyType, OracleReader};
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{address, b256, hex, keccak256, AccountInfo, Address, Bytecode, ExecutionResult, Output, TransactTo, B256},
    Evm,
};

extern crate alloc;

const HEAP_SIZE: usize = 0xFFFFFFF;

const EVM_ID_ADDRESS: Address = address!("dead00000000000000000000000000000000beef");
const SHA2_PRECOMPILE: Address = address!("0000000000000000000000000000000000000002");

const INPUT_KEY: B256 = b256!("0000000000000000000000000000000000000000000000000000000000000000");
const DIGEST_KEY: B256 = b256!("0000000000000000000000000000000000000000000000000000000000000001");
const CODE_KEY: B256 = b256!("0000000000000000000000000000000000000000000000000000000000000002");

#[no_mangle]
pub extern "C" fn _start() {
    kona_common::alloc_heap!(HEAP_SIZE);

    io::print("Booting EVM and checking hash...\n");
    let (input, digest, code) = boot().expect("Failed to boot");

    match run_evm(input, digest, code) {
        Ok(_) => io::print("Success, hashes matched!\n"),
        Err(e) => {
            let _ = io::print_err(alloc::format!("Error: {}\n", e).as_ref());
            io::exit(1);
        }
    }

    io::exit(0)
}

/// Boot the program and load bootstrap information.
fn boot() -> Result<(Vec<u8>, [u8; 32], Vec<u8>)> {
    let mut oracle = OracleReader::new(client_preimage_pipe_handle());
    let input = oracle.get(PreimageKey::new(*INPUT_KEY, PreimageKeyType::Local))?;
    let digest = oracle
        .get(PreimageKey::new(*DIGEST_KEY, PreimageKeyType::Local))?
        .try_into()
        .map_err(|_| anyhow!("Failed to convert digest to [u8; 32]"))?;
    let code = oracle.get(PreimageKey::new(*CODE_KEY, PreimageKeyType::Local))?;

    Ok((input, digest, code))
}

/// Call the SHA-256 precompile and assert that the input and output match the expected values
fn run_evm(input: Vec<u8>, digest: [u8; 32], code: Vec<u8>) -> Result<()> {
    let mut cache_db = CacheDB::new(EmptyDB::default());

    // Insert EVM identity contract into database.
    let id_account = AccountInfo {
        code_hash: keccak256(code.as_slice()),
        code: Some(Bytecode::new_raw(code.into())),
        ..Default::default()
    };
    cache_db.insert_account_info(EVM_ID_ADDRESS, id_account);

    // Create the EVM instance
    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.transact_to = TransactTo::Call(EVM_ID_ADDRESS);
            tx.data = input.into();
        })
        .build();

    // Call EVM identity contract.
    let ref_tx = evm.transact().map_err(|e| anyhow!("Failed state transition: {}", e))?;
    let value = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        e => bail!("EVM Execution failed: {:?}", e),
    };
    if value.as_ref() != evm.context.evm.env.tx.data.as_ref() {
        bail!(alloc::format!(
            "Expected: {} | Got: {}\n",
            hex::encode(digest),
            hex::encode(value)
        ));
    }

    // Set up SHA2 precompile call
    evm.context.evm.env.tx.transact_to = TransactTo::Call(SHA2_PRECOMPILE);

    // Call SHA2 precompile.
    let ref_tx = evm
        .transact()
        .map_err(|e| anyhow!("Failed state transition: {}", e))?;
    let value = match ref_tx.result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        e => bail!("EVM Execution failed: {:?}", e),
    };
    if value.as_ref() != digest.as_ref() {
        bail!(alloc::format!(
            "Expected: {} | Got: {}\n",
            hex::encode(digest),
            hex::encode(value)
        ));
    }

    Ok(())
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("Panic: {}", info);
    let _ = io::print_err(msg.as_ref());
    io::exit(2)
}
