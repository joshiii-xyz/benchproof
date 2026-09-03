# Operations

## Safe execution

Run `benchproof run -- ./benchmark` only when the operator intends to execute
that exact program. Arguments are passed directly to the child. The harness
does not interpret shell syntax or create pipelines. Use `--warmup`,
`--repetitions`, `--timeout-ms`, `--cache-policy`, and optional `--affinity`
to record the run setup.

The cache policy is a label such as `cold`, `warm`, or `unspecified`. It does
not change the filesystem. An affinity list such as `0,2-3` is requested for
each child on Linux and the report states whether setup was observed as
requested or failed.

## Reports and retention

JSON is the default output. Save it to a report file, then run
`benchproof inspect run.json` or `benchproof compare run-a.json run-b.json`.
Text output includes summary values and uncertainty. Reports contain command
arguments and host identity, so store them under the local access and
retention policy.

## Troubleshooting

- `failed` greater than zero means one or more measurement commands returned a
  nonzero status, signal, timeout, or spawn error.
- `comparable: false` means command, policy, conditions, or success state
  differed. Read `condition_differences` and `uncertainty`.
- A missing governor, frequency, thermal, or load value means that host probe
  was unavailable or unreadable. It is not a zero reading.
- An affinity spawn error can mean the requested CPU is outside the host's
  allowed set or the platform does not support this Linux-first path.

## Recovery

The harness does not modify filesystem caches or benchmark inputs. It cannot
undo side effects of the benchmark command. If a benchmark changes external
state, restore that state under the operator's normal change policy before
rerunning.
