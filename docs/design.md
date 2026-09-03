# Design

## Run record

`BenchmarkRun` contains an exact `CommandSpec`, a bounded `RunPolicy`, host
conditions, warmup samples, measurement samples, a summary, and uncertainty
strings. The timing interval surrounds the child process wait and uses
`Instant`, so it does not depend on wall-clock changes.

## Conditions

The host record includes OS family, architecture, kernel release, CPU model,
logical CPU count, process affinity request and observation, CPU governor,
current frequency, thermal reading, one-minute load, and the operator's
filesystem cache policy. Linux procfs and sysfs values are read only when
available and are never replaced with guessed defaults.

## Warmups and samples

Warmups are executed and recorded separately from measurement samples. Summary
statistics use successful measurement durations only. Each sample also records
the outcome, exit code, signal where available, iteration number, and phase.
Failed or timed-out invocations remain in the raw record and make the run
uncertain.

## Comparability

`compare_runs` checks exact command and policy equality plus recorded OS, CPU,
affinity, governor, frequency, thermal, load, and cache-policy conditions. It
returns timing deltas even when runs are not comparable, but the report keeps
`comparable` false and lists the differing conditions and uncertainty.

## Bounds and failure behavior

Warmups and repetitions are capped at 100. Each child has a bounded timeout.
Host probes read at most 16 KiB each. Reports are capped at 1 MiB. Missing,
unreadable, or unsupported metadata stays `None` and is named in uncertainty.

## Portability boundary

The runner uses portable Rust process and timing APIs. CPU affinity and host
metadata probes are Linux-first. The release evidence covers Linux only;
Windows affinity, macOS host telemetry, thermal interfaces, and equivalent
governor semantics are not claimed.
