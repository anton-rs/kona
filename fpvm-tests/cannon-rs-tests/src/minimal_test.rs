#![cfg(test)]

use cannon_mipsevm::test_utils::StaticOracle;
use cannon_mipsevm::InstrumentedState;
use cannon_mipsevm::{load_elf, patch_stack};
use std::io::BufWriter;

#[test]
fn test_minimal() {
    let elf_bytes =
        include_bytes!("../../bin/cannon/minimal");
    let mut state = load_elf(elf_bytes).unwrap();
    patch_stack(&mut state).unwrap();

    let out = BufWriter::new(Vec::default());
    let err = BufWriter::new(Vec::default());
    let mut ins = InstrumentedState::new(state, StaticOracle::default(), out, err);

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
        "Hello, world!\n"
    );
    assert_eq!(String::from_utf8(ins.std_err().to_vec()).unwrap(), "");
}
