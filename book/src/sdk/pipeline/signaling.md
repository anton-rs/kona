# Signals

Understanding signals first require a more in-depth review of the result
returned by stepping on the derivation pipeline.


## The [`StepResult`][step-result]

As briefly outlined in the [intro](./intro.md), stepping on the derivation
pipeline returns a [`StepResult`][step-result]. Step results provide a
an extensible way for pipeline stages to signal different results to the
pipeline driver. The variants of [`StepResult`][step-result] and what they
signal include the following.

- `StepResult::PreparedAttributes` - signals that payload attributes are
   ready to be consumed by the pipeline driver.
- `StepResult::AdvancedOrigin` - signals that the pipeline has derived all
   payload attributes for the given L1 block, and the origin of the pipeline
   was advanced to the next canonical L1 block.
- `StepResult::OriginAdvanceErr(_)` - The driver failed to advance the
   origin of pipeline.
- `StepResult::StepFailed(_)` - The step failed.

No action is needed when the prepared attributes step result is received.
The pipeline driver may chose to consume the payload attributes how it
wishes. Likewise, `StepResult::AdvancedOrigin` simply notifies the driver
that the pipeline advanced its origin - the driver may continue stepping
on the pipeline. Now, it becomes more involved with the remaining two
variants of [`StepResult`][step-result].

When either `StepResult::OriginAdvanceErr(_)` or `StepResult::StepFailed(_)`
are received, the pipeline driver needs to introspect the error within these
variants. Depending on the [`PipelineErrorKind`][error-kind], the driver may
need to send a "signal" down through the pipeline.

The next section goes over pipeline signals by looking at the variants of
the [`PipelineErrorKind`][error-kind] and the driver's response.


## [`PipelineErrorKind`][error-kind]

There are three variants of the [`PipelineErrorKind`][error-kind], each
groups the inner error based on severity (or how they should be handled).

- `PipelineErrorKind::Temporary` - This is an error that's expected, and
   is temporary. For example, not all channel data has been posted to L1
   so the pipeline doesn't have enough data yet to continue deriving
   payload attributes.
- `PipelineErrorKind::Critical` - This is an unexpected error that breaks
   the derivation pipeline. It should cause the driver to error since this
   is behavior that is breaking the derivation of payload attributes.
- `PipelineErrorKind::Reset` - When this is received, it effectively
   requests that the driver perform some action on the pipeline. Kona
   uses message passing so the driver can send a [`Signal`][signal] down
   the pipeline with whatever action that needs to be performed. By
   allowing both the driver and individual pipeline stages to define their
   own behaviour around signals, they become very extensible. More on this
   in [a later section](#extending-the-signal-type).


## The [`Signal`][signal] Type

Continuing from the [`PipelineErrorKind`][error-kind], when the driver
receives a `PipelineErrorKind::Reset`, it needs to send a signal down
through the pipeline.

Prior to the Holocene hardfork, the pipeline only needed to be reset
when the reset pipeline error was received. Holocene activation rules
changed this to require Holocene-specific activation logic internal to
the pipeline stages. The way kona's driver handles this activation is
by sending a new `ActivationSignal` if the `PipelineErrorKind::Reset`
type is a `ResetError::HoloceneActivation`. Otherwise, it will send the
`ResetSignal`.

The last of the three [`Signal`][signal] variants is the `FlushChannel`
signal. Similar to `ActivationSignal`, the flush channel signal is logic
introduced post-Holocene. When the driver fails to execute payload
attributes and Holocene is active, a `FlushChannel` signal needs to
forwards invalidate the associated batch and channel, and the block
is replaced with a deposit-only block.


## Extending the Signal Type

To extend the [`Signal`][signal] type, all that is needed is to introduce
a new variant to the [`Signal`][signal] enum.

Once the variant is added, the segments where signals are handled need to
be updated. Anywhere the [`SignalReceiver`][receiver] trait is
implemented, handling needs to be updated for the new signal variant. Most
notably, this is on the top-level [`DerivationPipeline`][dp] type, as well
as all [the pipeline stages][stages].

#### An Example

Let's create a new [`Signal`][signal] variant that updates the `RollupConfig`
in the [`L1Traversal`][traversal] stage. Let's call it `SetConfig`.
The [`signal`][signal] type would look like the following with this new
variant.

```rust
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
    /// Updates the rollup config in the L1Traversal stage.
    UpdateConfig(ConfigUpdateSignal),
}

/// A signal that updates the `RollupConfig`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ConfigUpdateSignal(Arc<RollupConfig>);
```

Next, all handling of the [`Signal`][signal] type needs to be updated for
the new `UpdateConfig` variant. For the sake of this example, we'll just
focus on updating the [`L1Traversal`][traversal] stage.

```rust
#[async_trait]
impl<F: ChainProvider + Send> SignalReceiver for L1Traversal<F> {
    async fn signal(&mut self, signal: Signal) -> PipelineResult<()> {
        match signal {
            Signal::Reset(ResetSignal { l1_origin, system_config, .. }) |
            Signal::Activation(ActivationSignal { l1_origin, system_config, .. }) => {
                self.block = Some(l1_origin);
                self.done = false;
                self.system_config = system_config.expect("System config must be provided.");
            }
            Signal::UpdateConfig(inner) => {
               self.rollup_config = Arc::clone(&inner.0);
            }
            _ => {}
        }

        Ok(())
    }
}
```


<!-- Links -->

[traversal]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.L1Traversal.html
[dp]: https://docs.rs/kona-derive/latest/kona_derive/pipeline/struct.DerivationPipeline.html
[stages]: https://docs.rs/kona-derive/latest/kona_derive/stages/index.html
[receiver]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.SignalReceiver.html
[signal]: https://docs.rs/kona-derive/latest/kona_derive/traits/enum.Signal.html
[error-kind]: https://docs.rs/kona-derive/latest/kona_derive/errors/enum.PipelineErrorKind.html
[step-result]: https://docs.rs/kona-derive/latest/kona_derive/traits/enum.StepResult.html
