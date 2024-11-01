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
   ready to be be consumed by the pipeline driver.
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

There are two


## Extending the Signal Type


<!-- Links -->

[signal]: https://docs.rs/kona-derive/latest/kona_derive/traits/enum.Signal.html
[error-kind]: https://docs.rs/kona-derive/latest/kona_derive/errors/enum.PipelineErrorKind.html
[step-result]: https://docs.rs/kona-derive/latest/kona_derive/traits/enum.StepResult.html
