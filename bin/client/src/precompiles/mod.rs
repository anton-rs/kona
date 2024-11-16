//! Contains accelerated precompiles for the FPVM.

mod bn128_pair;
pub use bn128_pair::{EcPairingAccelerated, EcPairingAcceleratedGranite, ECPAIRING_ADDRESS};

mod ecrecover;
pub use ecrecover::{EcRecoverAccelerated, ECRECOVER_ADDRESS};

mod kzg_point_eval;
pub use kzg_point_eval::{KZGPointEvalAccelerated, POINT_EVAL_ADDRESS};
