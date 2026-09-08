# benchproof

benchproof records enough local execution conditions to judge whether two
benchmark runs are meaningfully comparable. It stores the exact command,
warmup and repetition policy, raw timing samples, summary statistics, host
identity, affinity request, readable CPU and thermal signals, selected
filesystem cache policy, and explicit uncertainty.

Status: 0.1.0 implementation pending release evidence.

CI: https://github.com/yoshiii-xyz/benchproof/actions

## Install

```text
cargo install benchproof
```

## Quick start

```text
benchproof run -- ./benchmark
benchproof run --cache-policy warm --warmup 2 --repetitions 10 -- ./benchmark
benchproof compare run-a.json run-b.json
benchproof inspect run.json
```

Save JSON stdout from `run` as a report file. `compare` checks the command,
policy, host observations, and success state before presenting timing deltas.

## What it solves

A timing number is hard to interpret when CPU identity, affinity, frequency,
thermal state, background load, or cache policy is unknown. benchproof keeps
those conditions beside raw samples and says when they differ or were not
observable.

## How it works

The harness passes the program and argument vector directly to
`std::process::Command`. It runs bounded warmups followed by bounded
measurements, uses a monotonic interval for each child invocation, discards
child output, and records exit or timeout state. On Linux, an optional CPU
list is applied to each child before execution when the host accepts it.

Host probes are best-effort and bounded. Missing files or permission errors
become uncertainty rather than fabricated values. The selected filesystem
cache policy is operator input; benchproof does not flush or change caches.

## Commands and library API

The CLI provides `run`, `compare`, and `inspect`. The library exposes
`run_benchmark`, `compare_runs`, `report_json`, `decode_run`, and explanation
helpers. Use `benchproof --help` for the complete option list.

## Output and exit codes

- Exit code 0 means all measurement invocations succeeded, or an inspect or
  compare operation completed with a comparable result.
- Exit code 1 means a measurement failed or the compared runs are not
  comparable.
- Exit code 2 means an input, affinity, or report operation failed.

Warmups and repetitions are each capped at 100. A single invocation is capped
at ten minutes by the CLI. JSON reports are capped at 1 MiB. Summary values
are calculated from successful measurement samples only.

## Safety and data handling

The harness does not interpret shell syntax, construct pipelines, flush
filesystem caches, or upload results. A caller can explicitly choose a shell
as the benchmark program, but its semantics then belong to that supplied
program. Reports contain command arguments and host identity, so store them
with suitable permissions.

## Limits and non-goals

See [`docs/limits.md`](docs/limits.md). Linux is the only release-tested
platform. The tool does not claim perfectly reproducible timing, full thermal
or frequency telemetry, race-free host state, statistical significance, or
complete process-tree control.

This is not hyperfine, a scheduler, a profiler, a cache controller, or a
performance regression service.

## Testing and development

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and [`docs/release.md`](docs/release.md)
for the verified command set.

## Research

See [`docs/research.md`](docs/research.md) for the Rust timing and process API
source trail and the distinction between documented behavior and design
inference.

## Release and support status

The 0.1.0 release is pending local and hosted evidence. The release record
will be updated only after the exact package, checksum, docs.rs, CI, security,
CodeQL, tag package, and fresh-install checks pass.
