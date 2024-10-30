//! Defines the interface for the core derivation pipeline.

use super::OriginProvider;
use crate::errors::PipelineErrorKind;
use alloc::boxed::Box;
use async_trait::async_trait;
use core::iter::Iterator;
use op_alloy_genesis::SystemConfig;
use op_alloy_protocol::{BlockInfo, L2BlockInfo};
use op_alloy_rpc_types_engine::OpAttributesWithParent;

/// A pipeline error.
#[derive(Debug, PartialEq, Eq)]
pub enum StepResult {
    /// Attributes were successfully prepared.
    PreparedAttributes,
    /// Origin was advanced.
    AdvancedOrigin,
    /// Origin advance failed.
    OriginAdvanceErr(PipelineErrorKind),
    /// Step failed.
    StepFailed(PipelineErrorKind),
}

/// A signal to send to the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum Signal {
    /// Reset the pipeline.
    Reset(ResetSignal),
    /// Hardfork Activation.
    Activation(ActivationSignal),
    /// Flush the currently active channel.
    FlushChannel,
}

impl Signal {
    /// Sets the [SystemConfig] for the signal.
    pub const fn with_system_config(self, system_config: SystemConfig) -> Self {
        match self {
            Self::Reset(reset) => reset.with_system_config(system_config).signal(),
            Self::Activation(activation) => activation.with_system_config(system_config).signal(),
            Self::FlushChannel => Self::FlushChannel,
        }
    }
}

/// A pipeline reset signal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ResetSignal {
    /// The L2 safe head to reset to.
    pub l2_safe_head: L2BlockInfo,
    /// The L1 origin to reset to.
    pub l1_origin: BlockInfo,
    /// The optional [SystemConfig] to reset with.
    pub system_config: Option<SystemConfig>,
}

impl ResetSignal {
    /// Creates a new [Signal::Reset] from the [ResetSignal].
    pub const fn signal(self) -> Signal {
        Signal::Reset(self)
    }

    /// Sets the [SystemConfig] for the signal.
    pub const fn with_system_config(self, system_config: SystemConfig) -> Self {
        Self { system_config: Some(system_config), ..self }
    }
}

/// A pipeline hardfork activation signal.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ActivationSignal {
    /// The L2 safe head to reset to.
    pub l2_safe_head: L2BlockInfo,
    /// The L1 origin to reset to.
    pub l1_origin: BlockInfo,
    /// The optional [SystemConfig] to reset with.
    pub system_config: Option<SystemConfig>,
}

impl ActivationSignal {
    /// Creates a new [Signal::Activation] from the [ActivationSignal].
    pub const fn signal(self) -> Signal {
        Signal::Activation(self)
    }

    /// Sets the [SystemConfig] for the signal.
    pub const fn with_system_config(self, system_config: SystemConfig) -> Self {
        Self { system_config: Some(system_config), ..self }
    }
}

/// This trait defines the interface for interacting with the derivation pipeline.
#[async_trait]
pub trait Pipeline: OriginProvider + Iterator<Item = OpAttributesWithParent> {
    /// Peeks at the next [OpAttributesWithParent] from the pipeline.
    fn peek(&self) -> Option<&OpAttributesWithParent>;

    /// Attempts to progress the pipeline.
    async fn step(&mut self, cursor: L2BlockInfo) -> StepResult;
}

/// Metrics trait for `DerivationPipeline`.
pub trait DerivationPipelineMetrics {
    /// Records the result of a step in the pipeline.
    fn record_step_result(&self, result: &StepResult);

    /// Increments the count of reset signals received.
    fn inc_reset_signals(&self);

    /// Increments the count of flush channel signals received.
    fn inc_flush_channel_signals(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_signal() {
        let signal = ResetSignal::default();
        assert_eq!(signal.signal(), Signal::Reset(signal));
    }

    #[test]
    fn test_activation_signal() {
        let signal = ActivationSignal::default();
        assert_eq!(signal.signal(), Signal::Activation(signal));
    }

    #[test]
    fn test_signal_with_system_config() {
        let signal = ResetSignal::default();
        let system_config = SystemConfig::default();
        assert_eq!(
            signal.with_system_config(system_config).signal(),
            Signal::Reset(ResetSignal { system_config: Some(system_config), ..signal })
        );

        let signal = ActivationSignal::default();
        let system_config = SystemConfig::default();
        assert_eq!(
            signal.with_system_config(system_config).signal(),
            Signal::Activation(ActivationSignal { system_config: Some(system_config), ..signal })
        );

        assert_eq!(Signal::FlushChannel.with_system_config(system_config), Signal::FlushChannel);
    }
}
