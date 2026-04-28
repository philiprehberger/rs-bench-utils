# Changelog

## 0.2.0 (2026-04-27)

- Add `bench_with_setup` for benchmarks where each iteration consumes a fresh input
- Add `BenchResult::cv()` returning the coefficient of variation as a percentage
- Add `BenchResult::confidence_interval_95()` returning the 95% CI for the mean
- Add `BenchGroup::compare(name1, name2)` for pairwise comparison of recorded results

## 0.1.5 (2026-03-31)

- Standardize README to 3-badge format with emoji Support section
- Update CI checkout action to v5 for Node.js 24 compatibility

## 0.1.4 (2026-03-27)

- Add GitHub issue templates, PR template, and dependabot configuration
- Update README badges and add Support section

## 0.1.3 (2026-03-22)

- Fix README, CHANGELOG, and CI compliance

## 0.1.2 (2026-03-20)

- Fix CI workflow, re-publish after rate limit

## 0.1.1 (2026-03-20)

- Re-release with registry token configured

## 0.1.0 (2026-03-20)

- `BenchResult` struct with statistical methods (mean, median, stddev, min, max, p95, p99)
- `bench` function for measuring closure performance
- `bench_with_warmup` for warmup + measurement runs
- `bench_compare` for comparing two implementations with `CompareResult`
- `check_regression` for regression detection against a threshold
- `Throughput` calculation with human-readable formatting
- `black_box` to prevent compiler optimizations
- `BenchGroup` for organizing and comparing multiple benchmarks
- Human-readable duration formatting (ns, us, ms, s)
