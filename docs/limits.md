# Limits and non-goals

The 0.1.0 MVP has these limits:

- Linux is the only platform covered by release evidence.
- Timing is local monotonic wall-clock evidence. It is not a proof of perfect
  reproducibility, causality, or statistical significance.
- CPU governor, frequency, thermal, load, and affinity observations depend on
  host permissions and procfs or sysfs availability. Missing values are
  reported as unobserved.
- The filesystem cache policy is a user-selected label. benchproof does not
  flush, warm, or otherwise control filesystem caches.
- A child can spawn descendants or alter process state outside the harness's
  direct observations. The tool does not provide complete process-tree
  control.
- Warmups and repetitions are capped at 100, invocation time at ten minutes,
  and report JSON at 1 MiB.
- Comparability uses exact recorded values. Values that change between runs
  can make a comparison non-comparable even when the benchmark command is
  unchanged.
- The harness does not inspect benchmark correctness, input distribution,
  compiler flags, hardware isolation, scheduler state, or all background
  work.

The project is not hyperfine, a profiler, scheduler, cache controller,
regression service, or hosted benchmark platform.
