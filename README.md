# rs-bench-utils

[![CI](https://github.com/philiprehberger/rs-bench-utils/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-bench-utils/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-bench-utils.svg)](https://crates.io/crates/philiprehberger-bench-utils)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-bench-utils)](https://github.com/philiprehberger/rs-bench-utils/commits/main)

Micro-benchmarking utilities with statistical analysis, comparison, and regression detection

## Installation

```toml
[dependencies]
philiprehberger-bench-utils = "0.2.0"
```

## Usage

```rust
use philiprehberger_bench_utils::{bench, black_box};

let result = bench("sum", 1000, || {
    let _sum: u64 = black_box((0..1000).sum());
});

println!("{}", result.summary());
```

### Compare two implementations

```rust
use philiprehberger_bench_utils::bench_compare;

let cmp = bench_compare(
    "vec_collect", || { let _: Vec<i32> = (0..100).collect(); },
    "vec_push", || { let mut v = Vec::new(); for i in 0..100 { v.push(i); } },
    1000,
);

println!("{}", cmp.summary());
```

### Regression detection

```rust
use philiprehberger_bench_utils::{bench, check_regression};

let baseline = bench("v1", 1000, || { /* old code */ });
let current = bench("v2", 1000, || { /* new code */ });

let check = check_regression(&baseline, &current, 10.0);
if check.regressed {
    eprintln!("Performance regression: {:.1}%", check.diff_percent);
}
```

### Throughput measurement

```rust
use philiprehberger_bench_utils::{bench, throughput};

let result = bench("parse", 1000, || {
    let _: Vec<&str> = "a,b,c,d,e".split(',').collect();
});

let tp = throughput(&result, 9); // 9 bytes per operation
println!("{}", tp.human_bytes()); // e.g. "150.5 MB/s"
```

### Benchmark groups

```rust
use philiprehberger_bench_utils::BenchGroup;

let mut group = BenchGroup::new("sorting");
group.add("sort_unstable", 100, || { let mut v = vec![3,1,2]; v.sort_unstable(); });
group.add("sort_stable", 100, || { let mut v = vec![3,1,2]; v.sort(); });

println!("{}", group.summary());

// Pairwise compare two recorded results without re-running
if let Some(cmp) = group.compare("sort_stable", "sort_unstable") {
    println!("{}", cmp.summary());
}
```

### Per-iteration setup

```rust
use philiprehberger_bench_utils::{bench_with_setup, black_box};

// Each iteration gets a fresh, unsorted Vec — setup time is excluded
let result = bench_with_setup(
    "sort_unstable",
    100,
    || vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5],
    |mut v| {
        v.sort_unstable();
        black_box(v);
    },
);
```

### Confidence intervals

```rust
use philiprehberger_bench_utils::bench;

let result = bench("op", 100, || { let _: u64 = (0..50).sum(); });
let (low, high) = result.confidence_interval_95();
println!("95% CI: [{:.0} ns, {:.0} ns]  cv={:.1}%", low, high, result.cv());
```

## API

| Function / Type | Description |
|---|---|
| `bench(name, iterations, f)` | Run and measure a closure |
| `bench_with_warmup(name, warmup, iterations, f)` | Warmup runs then measure |
| `bench_with_setup(name, iterations, setup, f)` | Per-iteration setup excluded from samples |
| `bench_compare(name1, f1, name2, f2, iterations)` | Compare two closures |
| `check_regression(baseline, current, threshold)` | Detect performance regressions |
| `throughput(result, bytes_per_op)` | Calculate throughput metrics |
| `black_box(value)` | Prevent compiler optimizations |
| `BenchResult` | Statistical results (mean, median, stddev, p95, p99) |
| `BenchResult::cv()` | Coefficient of variation as a percentage |
| `BenchResult::confidence_interval_95()` | 95% CI for the mean as `(low, high)` |
| `CompareResult` | Comparison with speedup and diff percentage |
| `RegressionCheck` | Regression detection result |
| `Throughput` | Bytes/s and ops/s metrics |
| `BenchGroup` | Group and compare multiple benchmarks |
| `BenchGroup::compare(name1, name2)` | Pairwise compare two recorded results |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-bench-utils)

🐛 [Report issues](https://github.com/philiprehberger/rs-bench-utils/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-bench-utils/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
