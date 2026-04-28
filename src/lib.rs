//! Micro-benchmarking utilities with statistical analysis, comparison, and regression detection.
//!
//! Provides simple functions for measuring closure performance, comparing implementations,
//! detecting regressions, and calculating throughput — all with zero dependencies.
//!
//! # Quick start
//!
//! ```
//! use philiprehberger_bench_utils::{bench, black_box};
//!
//! let result = bench("sum", 100, || {
//!     let _sum: u64 = black_box((0..1000).sum());
//! });
//!
//! println!("{}", result.summary());
//! ```

use std::time::Instant;

/// Result of a benchmark run, containing timing samples and statistical methods.
#[derive(Debug, Clone)]
pub struct BenchResult {
    /// Name of the benchmark.
    pub name: String,
    /// Number of iterations measured.
    pub iterations: usize,
    /// Total elapsed time in nanoseconds across all iterations.
    pub total_ns: u128,
    /// Individual sample durations in nanoseconds.
    pub samples: Vec<u128>,
}

impl BenchResult {
    /// Returns the arithmetic mean duration in nanoseconds.
    pub fn mean_ns(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.total_ns as f64 / self.samples.len() as f64
    }

    /// Returns the median duration in nanoseconds.
    pub fn median_ns(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_unstable();
        let len = sorted.len();
        if len % 2 == 0 {
            (sorted[len / 2 - 1] as f64 + sorted[len / 2] as f64) / 2.0
        } else {
            sorted[len / 2] as f64
        }
    }

    /// Returns the standard deviation of sample durations in nanoseconds.
    pub fn stddev_ns(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let mean = self.mean_ns();
        let variance = self
            .samples
            .iter()
            .map(|&s| {
                let diff = s as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / (self.samples.len() - 1) as f64;
        variance.sqrt()
    }

    /// Returns the minimum sample duration in nanoseconds.
    pub fn min_ns(&self) -> u128 {
        self.samples.iter().copied().min().unwrap_or(0)
    }

    /// Returns the maximum sample duration in nanoseconds.
    pub fn max_ns(&self) -> u128 {
        self.samples.iter().copied().max().unwrap_or(0)
    }

    /// Returns the 99th percentile duration in nanoseconds.
    pub fn p99_ns(&self) -> f64 {
        percentile(&self.samples, 99.0)
    }

    /// Returns the 95th percentile duration in nanoseconds.
    pub fn p95_ns(&self) -> f64 {
        percentile(&self.samples, 95.0)
    }

    /// Returns operations per second based on mean duration.
    pub fn ops_per_sec(&self) -> f64 {
        let mean = self.mean_ns();
        if mean == 0.0 {
            return 0.0;
        }
        1_000_000_000.0 / mean
    }

    /// Returns the coefficient of variation as a percentage (`stddev / mean * 100`).
    ///
    /// A common signal-to-noise indicator: values under 5% are very stable,
    /// values over 20% suggest the benchmark is noisy. Returns `0.0` when
    /// the mean is zero.
    pub fn cv(&self) -> f64 {
        let mean = self.mean_ns();
        if mean == 0.0 {
            return 0.0;
        }
        self.stddev_ns() / mean * 100.0
    }

    /// Returns the 95% confidence interval for the mean as `(low, high)`,
    /// using the normal approximation (`mean ± 1.96 * stddev / sqrt(n)`).
    ///
    /// Returns `(mean, mean)` for samples with fewer than 2 entries.
    pub fn confidence_interval_95(&self) -> (f64, f64) {
        let n = self.samples.len();
        if n < 2 {
            let mean = self.mean_ns();
            return (mean, mean);
        }
        let mean = self.mean_ns();
        let stderr = self.stddev_ns() / (n as f64).sqrt();
        let margin = 1.96 * stderr;
        (mean - margin, mean + margin)
    }

    /// Returns a human-readable string for the mean duration.
    ///
    /// Formats as nanoseconds, microseconds, milliseconds, or seconds
    /// depending on magnitude.
    pub fn mean_human(&self) -> String {
        format_duration_ns(self.mean_ns())
    }

    /// Returns a single-line summary of the benchmark result.
    pub fn summary(&self) -> String {
        format!(
            "{}: mean={} median={} stddev={} min={} max={} p95={} p99={} ({} iterations)",
            self.name,
            format_duration_ns(self.mean_ns()),
            format_duration_ns(self.median_ns()),
            format_duration_ns(self.stddev_ns()),
            format_duration_ns(self.min_ns() as f64),
            format_duration_ns(self.max_ns() as f64),
            format_duration_ns(self.p95_ns()),
            format_duration_ns(self.p99_ns()),
            self.iterations,
        )
    }
}

/// Result of comparing two benchmarks.
#[derive(Debug, Clone)]
pub struct CompareResult {
    /// The baseline benchmark result.
    pub baseline: BenchResult,
    /// The candidate benchmark result.
    pub candidate: BenchResult,
    /// Speedup factor. Values > 1.0 mean the candidate is faster.
    pub speedup: f64,
    /// Percentage difference. Negative means faster, positive means slower.
    pub diff_percent: f64,
}

impl CompareResult {
    /// Returns a human-readable summary of the comparison.
    pub fn summary(&self) -> String {
        let direction = if self.is_faster() { "faster" } else { "slower" };
        format!(
            "{} vs {}: {:.1}x {} ({:+.1}%)",
            self.candidate.name,
            self.baseline.name,
            if self.is_faster() {
                self.speedup
            } else {
                1.0 / self.speedup
            },
            direction,
            self.diff_percent,
        )
    }

    /// Returns true if the candidate is faster than the baseline.
    pub fn is_faster(&self) -> bool {
        self.speedup > 1.0
    }

    /// Returns true if the candidate is slower than the baseline.
    pub fn is_slower(&self) -> bool {
        self.speedup < 1.0
    }
}

/// Result of a regression check between a baseline and current benchmark.
#[derive(Debug, Clone)]
pub struct RegressionCheck {
    /// Name of the benchmark being checked.
    pub name: String,
    /// Mean duration of the baseline in nanoseconds.
    pub baseline_mean_ns: f64,
    /// Mean duration of the current run in nanoseconds.
    pub current_mean_ns: f64,
    /// Regression threshold as a percentage.
    pub threshold_percent: f64,
    /// Whether a regression was detected.
    pub regressed: bool,
    /// Percentage difference from baseline (positive = slower).
    pub diff_percent: f64,
}

/// Throughput metrics for a benchmark.
#[derive(Debug, Clone)]
pub struct Throughput {
    /// Bytes processed per second.
    pub bytes_per_sec: f64,
    /// Operations completed per second.
    pub ops_per_sec: f64,
}

impl Throughput {
    /// Returns a human-readable bytes-per-second string (e.g. "150.5 MB/s").
    pub fn human_bytes(&self) -> String {
        let (val, unit) = humanize_rate(self.bytes_per_sec);
        format!("{:.1} {}/s", val, unit)
    }

    /// Returns a human-readable operations-per-second string (e.g. "1.23 Mops/s").
    pub fn human_ops(&self) -> String {
        if self.ops_per_sec >= 1_000_000_000.0 {
            format!("{:.2} Gops/s", self.ops_per_sec / 1_000_000_000.0)
        } else if self.ops_per_sec >= 1_000_000.0 {
            format!("{:.2} Mops/s", self.ops_per_sec / 1_000_000.0)
        } else if self.ops_per_sec >= 1_000.0 {
            format!("{:.2} Kops/s", self.ops_per_sec / 1_000.0)
        } else {
            format!("{:.2} ops/s", self.ops_per_sec)
        }
    }
}

/// A group of benchmarks that can be run and compared together.
pub struct BenchGroup {
    name: String,
    results: Vec<BenchResult>,
}

impl BenchGroup {
    /// Creates a new benchmark group with the given name.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            results: Vec::new(),
        }
    }

    /// Runs a benchmark and adds the result to the group.
    pub fn add<F>(&mut self, name: &str, iterations: usize, f: F)
    where
        F: Fn(),
    {
        let result = bench(name, iterations, f);
        self.results.push(result);
    }

    /// Returns a slice of all benchmark results in the group.
    pub fn results(&self) -> &[BenchResult] {
        &self.results
    }

    /// Returns the fastest benchmark result, or `None` if the group is empty.
    pub fn fastest(&self) -> Option<&BenchResult> {
        self.results
            .iter()
            .min_by(|a, b| a.mean_ns().partial_cmp(&b.mean_ns()).unwrap())
    }

    /// Returns the slowest benchmark result, or `None` if the group is empty.
    pub fn slowest(&self) -> Option<&BenchResult> {
        self.results
            .iter()
            .max_by(|a, b| a.mean_ns().partial_cmp(&b.mean_ns()).unwrap())
    }

    /// Compares two named results already in the group without re-running.
    ///
    /// Returns `None` if either name is missing. The first argument is the
    /// baseline; the second is the candidate.
    pub fn compare(&self, baseline_name: &str, candidate_name: &str) -> Option<CompareResult> {
        let baseline = self.results.iter().find(|r| r.name == baseline_name)?;
        let candidate = self.results.iter().find(|r| r.name == candidate_name)?;
        let baseline_mean = baseline.mean_ns();
        let candidate_mean = candidate.mean_ns();
        let speedup = if candidate_mean > 0.0 {
            baseline_mean / candidate_mean
        } else {
            f64::INFINITY
        };
        let diff_percent = if baseline_mean > 0.0 {
            ((candidate_mean - baseline_mean) / baseline_mean) * 100.0
        } else {
            0.0
        };
        Some(CompareResult {
            baseline: baseline.clone(),
            candidate: candidate.clone(),
            speedup,
            diff_percent,
        })
    }

    /// Returns a summary table of all results sorted by mean duration (fastest first).
    pub fn summary(&self) -> String {
        if self.results.is_empty() {
            return format!("[{}] (no results)", self.name);
        }

        let mut sorted: Vec<&BenchResult> = self.results.iter().collect();
        sorted.sort_by(|a, b| a.mean_ns().partial_cmp(&b.mean_ns()).unwrap());

        let fastest_mean = sorted[0].mean_ns();
        let mut lines = vec![format!("[{}]", self.name)];

        for result in &sorted {
            let relative = if fastest_mean > 0.0 {
                result.mean_ns() / fastest_mean
            } else {
                1.0
            };
            lines.push(format!(
                "  {}: mean={} ({:.2}x)",
                result.name,
                format_duration_ns(result.mean_ns()),
                relative,
            ));
        }

        lines.join("\n")
    }
}

/// Runs a benchmark, measuring each of `iterations` invocations of `f`.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::{bench, black_box};
///
/// let result = bench("add", 100, || {
///     black_box(2 + 2);
/// });
/// assert_eq!(result.iterations, 100);
/// assert_eq!(result.samples.len(), 100);
/// ```
pub fn bench<F>(name: &str, iterations: usize, f: F) -> BenchResult
where
    F: Fn(),
{
    let mut samples = Vec::with_capacity(iterations);
    let overall_start = Instant::now();

    for _ in 0..iterations {
        let start = Instant::now();
        f();
        let elapsed = start.elapsed().as_nanos();
        samples.push(elapsed);
    }

    let total_ns = overall_start.elapsed().as_nanos();

    BenchResult {
        name: name.to_string(),
        iterations,
        total_ns,
        samples,
    }
}

/// Runs a benchmark with warmup iterations that are not measured.
///
/// The closure is called `warmup` times without recording, then `iterations` times
/// with measurement.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::bench_with_warmup;
///
/// let result = bench_with_warmup("add", 10, 100, || {
///     let _ = 2 + 2;
/// });
/// assert_eq!(result.iterations, 100);
/// ```
pub fn bench_with_warmup<F>(name: &str, warmup: usize, iterations: usize, f: F) -> BenchResult
where
    F: Fn(),
{
    for _ in 0..warmup {
        f();
    }

    bench(name, iterations, f)
}

/// Runs a benchmark where each iteration first calls `setup` to produce a fresh
/// input, then measures only the time spent in `f`.
///
/// `setup` is *not* included in samples. Use this when each iteration mutates
/// or consumes the input — for example, benchmarking `Vec::sort` requires
/// fresh unsorted data per iteration.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::{bench_with_setup, black_box};
///
/// let result = bench_with_setup(
///     "sort_unstable",
///     50,
///     || vec![3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5],
///     |mut v| {
///         v.sort_unstable();
///         black_box(v);
///     },
/// );
/// assert_eq!(result.iterations, 50);
/// assert_eq!(result.samples.len(), 50);
/// ```
pub fn bench_with_setup<S, T, F>(name: &str, iterations: usize, setup: S, f: F) -> BenchResult
where
    S: Fn() -> T,
    F: Fn(T),
{
    let mut samples = Vec::with_capacity(iterations);
    let overall_start = Instant::now();

    for _ in 0..iterations {
        let input = setup();
        let start = Instant::now();
        f(input);
        samples.push(start.elapsed().as_nanos());
    }

    let total_ns = overall_start.elapsed().as_nanos();

    BenchResult {
        name: name.to_string(),
        iterations,
        total_ns,
        samples,
    }
}

/// Compares two closures by benchmarking each for `iterations` runs.
///
/// Returns a [`CompareResult`] with speedup factor and percentage difference.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::bench_compare;
///
/// let cmp = bench_compare(
///     "baseline", || { let _: u64 = (0..100).sum(); },
///     "candidate", || { let _: u64 = (0..50).sum(); },
///     100,
/// );
/// assert!(cmp.speedup > 0.0);
/// ```
pub fn bench_compare<F1, F2>(
    baseline_name: &str,
    baseline: F1,
    candidate_name: &str,
    candidate: F2,
    iterations: usize,
) -> CompareResult
where
    F1: Fn(),
    F2: Fn(),
{
    let baseline_result = bench(baseline_name, iterations, baseline);
    let candidate_result = bench(candidate_name, iterations, candidate);

    let baseline_mean = baseline_result.mean_ns();
    let candidate_mean = candidate_result.mean_ns();

    let speedup = if candidate_mean > 0.0 {
        baseline_mean / candidate_mean
    } else {
        f64::INFINITY
    };

    let diff_percent = if baseline_mean > 0.0 {
        ((candidate_mean - baseline_mean) / baseline_mean) * 100.0
    } else {
        0.0
    };

    CompareResult {
        baseline: baseline_result,
        candidate: candidate_result,
        speedup,
        diff_percent,
    }
}

/// Checks whether the current benchmark shows a regression compared to a baseline.
///
/// A regression is detected when the current mean is more than `threshold_percent`
/// slower than the baseline mean.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::{bench, check_regression};
///
/// let baseline = bench("v1", 100, || { let _ = 2 + 2; });
/// let current = bench("v2", 100, || { let _ = 2 + 2; });
///
/// let check = check_regression(&baseline, &current, 10.0);
/// // check.regressed will be true if current is >10% slower
/// ```
pub fn check_regression(
    baseline: &BenchResult,
    current: &BenchResult,
    threshold_percent: f64,
) -> RegressionCheck {
    let baseline_mean = baseline.mean_ns();
    let current_mean = current.mean_ns();

    let diff_percent = if baseline_mean > 0.0 {
        ((current_mean - baseline_mean) / baseline_mean) * 100.0
    } else {
        0.0
    };

    let regressed = diff_percent > threshold_percent;

    RegressionCheck {
        name: current.name.clone(),
        baseline_mean_ns: baseline_mean,
        current_mean_ns: current_mean,
        threshold_percent,
        regressed,
        diff_percent,
    }
}

/// Calculates throughput metrics given a benchmark result and bytes per operation.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::{bench, throughput, black_box};
///
/// let result = bench("parse", 100, || {
///     black_box("hello".len());
/// });
/// let tp = throughput(&result, 5);
/// assert!(tp.bytes_per_sec > 0.0);
/// assert!(tp.ops_per_sec > 0.0);
/// ```
pub fn throughput(result: &BenchResult, bytes_per_op: usize) -> Throughput {
    let ops = result.ops_per_sec();
    Throughput {
        bytes_per_sec: ops * bytes_per_op as f64,
        ops_per_sec: ops,
    }
}

/// Prevents the compiler from optimizing away a value.
///
/// Wraps [`std::hint::black_box`], which is stable since Rust 1.66.
///
/// # Examples
///
/// ```
/// use philiprehberger_bench_utils::black_box;
///
/// let x = black_box(42);
/// assert_eq!(x, 42);
/// ```
#[inline]
pub fn black_box<T>(value: T) -> T {
    std::hint::black_box(value)
}

// --- Internal helpers ---

fn percentile(samples: &[u128], pct: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();

    if len == 1 {
        return sorted[0] as f64;
    }

    let rank = (pct / 100.0) * (len - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;

    if lower == upper {
        sorted[lower] as f64
    } else {
        sorted[lower] as f64 * (1.0 - frac) + sorted[upper] as f64 * frac
    }
}

fn format_duration_ns(ns: f64) -> String {
    if ns >= 1_000_000_000.0 {
        format!("{:.2} s", ns / 1_000_000_000.0)
    } else if ns >= 1_000_000.0 {
        format!("{:.2} ms", ns / 1_000_000.0)
    } else if ns >= 1_000.0 {
        format!("{:.2} us", ns / 1_000.0)
    } else {
        format!("{:.2} ns", ns)
    }
}

fn humanize_rate(bytes_per_sec: f64) -> (f64, &'static str) {
    if bytes_per_sec >= 1_000_000_000.0 {
        (bytes_per_sec / 1_000_000_000.0, "GB")
    } else if bytes_per_sec >= 1_000_000.0 {
        (bytes_per_sec / 1_000_000.0, "MB")
    } else if bytes_per_sec >= 1_000.0 {
        (bytes_per_sec / 1_000.0, "KB")
    } else {
        (bytes_per_sec, "B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bench_simple_operation() {
        let result = bench("sum", 100, || {
            let _sum: u64 = black_box((0..1000).sum());
        });
        assert_eq!(result.name, "sum");
        assert_eq!(result.iterations, 100);
        assert_eq!(result.samples.len(), 100);
        assert!(result.total_ns > 0);
    }

    #[test]
    fn test_mean_with_known_data() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 5,
            total_ns: 150,
            samples: vec![10, 20, 30, 40, 50],
        };
        assert!((result.mean_ns() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_median_with_known_data_odd() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 5,
            total_ns: 150,
            samples: vec![50, 10, 30, 20, 40],
        };
        assert!((result.median_ns() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_median_with_known_data_even() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 4,
            total_ns: 100,
            samples: vec![10, 20, 30, 40],
        };
        assert!((result.median_ns() - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_stddev_with_known_data() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 4,
            total_ns: 40,
            samples: vec![10, 10, 10, 10],
        };
        assert!((result.stddev_ns() - 0.0).abs() < 0.01);

        let result2 = BenchResult {
            name: "test".to_string(),
            iterations: 2,
            total_ns: 30,
            samples: vec![10, 20],
        };
        // stddev of [10, 20] with sample correction = sqrt((25+25)/1) = sqrt(50) ≈ 7.07
        assert!((result2.stddev_ns() - 7.071).abs() < 0.1);
    }

    #[test]
    fn test_min_max() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 5,
            total_ns: 150,
            samples: vec![50, 10, 30, 20, 40],
        };
        assert_eq!(result.min_ns(), 10);
        assert_eq!(result.max_ns(), 50);
    }

    #[test]
    fn test_p95_p99_with_known_data() {
        // 100 samples: 1, 2, 3, ..., 100
        let samples: Vec<u128> = (1..=100).collect();
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 100,
            total_ns: 5050,
            samples,
        };
        // p95 at index 94.05 → interpolation between 95 and 96
        let p95 = result.p95_ns();
        assert!(p95 > 94.0 && p95 < 97.0, "p95={}", p95);

        let p99 = result.p99_ns();
        assert!(p99 > 98.0 && p99 <= 100.0, "p99={}", p99);
    }

    #[test]
    fn test_ops_per_sec() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 1,
            total_ns: 1_000_000,
            samples: vec![1_000_000],
        };
        // mean = 1_000_000 ns = 1 ms → 1000 ops/sec
        let ops = result.ops_per_sec();
        assert!((ops - 1000.0).abs() < 0.1, "ops={}", ops);
    }

    #[test]
    fn test_mean_human_nanoseconds() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 1,
            total_ns: 456,
            samples: vec![456],
        };
        assert_eq!(result.mean_human(), "456.00 ns");
    }

    #[test]
    fn test_mean_human_microseconds() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 1,
            total_ns: 5_500,
            samples: vec![5_500],
        };
        assert_eq!(result.mean_human(), "5.50 us");
    }

    #[test]
    fn test_mean_human_milliseconds() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 1,
            total_ns: 1_230_000,
            samples: vec![1_230_000],
        };
        assert_eq!(result.mean_human(), "1.23 ms");
    }

    #[test]
    fn test_mean_human_seconds() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 1,
            total_ns: 2_340_000_000,
            samples: vec![2_340_000_000],
        };
        assert_eq!(result.mean_human(), "2.34 s");
    }

    #[test]
    fn test_bench_compare() {
        let cmp = bench_compare(
            "slow",
            || {
                let _: u64 = black_box((0..10000).sum());
            },
            "fast",
            || {
                let _: u64 = black_box((0..100).sum());
            },
            100,
        );
        assert_eq!(cmp.baseline.name, "slow");
        assert_eq!(cmp.candidate.name, "fast");
        assert!(cmp.speedup > 0.0);
    }

    #[test]
    fn test_compare_result_is_faster_slower() {
        let fast = BenchResult {
            name: "fast".to_string(),
            iterations: 1,
            total_ns: 100,
            samples: vec![100],
        };
        let slow = BenchResult {
            name: "slow".to_string(),
            iterations: 1,
            total_ns: 200,
            samples: vec![200],
        };

        let cmp = CompareResult {
            baseline: slow.clone(),
            candidate: fast.clone(),
            speedup: 2.0,
            diff_percent: -50.0,
        };
        assert!(cmp.is_faster());
        assert!(!cmp.is_slower());

        let cmp2 = CompareResult {
            baseline: fast,
            candidate: slow,
            speedup: 0.5,
            diff_percent: 100.0,
        };
        assert!(!cmp2.is_faster());
        assert!(cmp2.is_slower());
    }

    #[test]
    fn test_compare_result_summary() {
        let baseline = BenchResult {
            name: "old".to_string(),
            iterations: 1,
            total_ns: 200,
            samples: vec![200],
        };
        let candidate = BenchResult {
            name: "new".to_string(),
            iterations: 1,
            total_ns: 100,
            samples: vec![100],
        };
        let cmp = CompareResult {
            baseline,
            candidate,
            speedup: 2.0,
            diff_percent: -50.0,
        };
        let summary = cmp.summary();
        assert!(summary.contains("new vs old"), "summary={}", summary);
        assert!(summary.contains("2.0x faster"), "summary={}", summary);
        assert!(summary.contains("-50.0%"), "summary={}", summary);
    }

    #[test]
    fn test_check_regression_detected() {
        let baseline = BenchResult {
            name: "v1".to_string(),
            iterations: 1,
            total_ns: 100,
            samples: vec![100],
        };
        let current = BenchResult {
            name: "v2".to_string(),
            iterations: 1,
            total_ns: 120,
            samples: vec![120],
        };
        let check = check_regression(&baseline, &current, 10.0);
        assert!(check.regressed);
        assert!((check.diff_percent - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_check_regression_not_detected() {
        let baseline = BenchResult {
            name: "v1".to_string(),
            iterations: 1,
            total_ns: 100,
            samples: vec![100],
        };
        let current = BenchResult {
            name: "v2".to_string(),
            iterations: 1,
            total_ns: 105,
            samples: vec![105],
        };
        let check = check_regression(&baseline, &current, 10.0);
        assert!(!check.regressed);
    }

    #[test]
    fn test_throughput_calculation() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 1,
            total_ns: 1_000_000,
            samples: vec![1_000_000], // 1ms per op
        };
        let tp = throughput(&result, 1024);
        // ops_per_sec = 1_000_000_000 / 1_000_000 = 1000
        assert!((tp.ops_per_sec - 1000.0).abs() < 0.1);
        // bytes_per_sec = 1000 * 1024 = 1_024_000
        assert!((tp.bytes_per_sec - 1_024_000.0).abs() < 1.0);
    }

    #[test]
    fn test_throughput_human_bytes() {
        let tp = Throughput {
            bytes_per_sec: 150_500_000.0,
            ops_per_sec: 1_000.0,
        };
        assert_eq!(tp.human_bytes(), "150.5 MB/s");
    }

    #[test]
    fn test_throughput_human_ops() {
        let tp = Throughput {
            bytes_per_sec: 1_000.0,
            ops_per_sec: 1_230_000.0,
        };
        assert_eq!(tp.human_ops(), "1.23 Mops/s");
    }

    #[test]
    fn test_bench_group() {
        let mut group = BenchGroup::new("test_group");
        group.add("fast", 50, || {
            let _: u64 = black_box((0..10).sum());
        });
        group.add("slow", 50, || {
            let _: u64 = black_box((0..10000).sum());
        });

        assert_eq!(group.results().len(), 2);
        assert!(group.fastest().is_some());
        assert!(group.slowest().is_some());

        let summary = group.summary();
        assert!(summary.contains("[test_group]"), "summary={}", summary);
        assert!(summary.contains("fast"), "summary={}", summary);
        assert!(summary.contains("slow"), "summary={}", summary);
    }

    #[test]
    fn test_black_box_preserves_value() {
        assert_eq!(black_box(42), 42);
        assert_eq!(black_box("hello"), "hello");
        assert_eq!(black_box(vec![1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn test_bench_result_summary_format() {
        let result = BenchResult {
            name: "mytest".to_string(),
            iterations: 5,
            total_ns: 500,
            samples: vec![80, 90, 100, 110, 120],
        };
        let summary = result.summary();
        assert!(summary.starts_with("mytest:"), "summary={}", summary);
        assert!(summary.contains("mean="), "summary={}", summary);
        assert!(summary.contains("median="), "summary={}", summary);
        assert!(summary.contains("stddev="), "summary={}", summary);
        assert!(summary.contains("min="), "summary={}", summary);
        assert!(summary.contains("max="), "summary={}", summary);
        assert!(summary.contains("p95="), "summary={}", summary);
        assert!(summary.contains("p99="), "summary={}", summary);
        assert!(summary.contains("(5 iterations)"), "summary={}", summary);
    }

    #[test]
    fn test_bench_with_warmup() {
        let result = bench_with_warmup("warmup_test", 10, 50, || {
            let _: u64 = black_box((0..100).sum());
        });
        assert_eq!(result.iterations, 50);
        assert_eq!(result.samples.len(), 50);
    }

    #[test]
    fn test_cv_calculation() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 2,
            total_ns: 30,
            samples: vec![10, 20],
        };
        // mean=15, stddev≈7.071, cv = 7.071/15*100 ≈ 47.14
        let cv = result.cv();
        assert!((cv - 47.14).abs() < 0.5, "cv={}", cv);
    }

    #[test]
    fn test_cv_zero_mean() {
        let result = BenchResult {
            name: "empty".to_string(),
            iterations: 0,
            total_ns: 0,
            samples: vec![],
        };
        assert_eq!(result.cv(), 0.0);
    }

    #[test]
    fn test_confidence_interval_95() {
        let result = BenchResult {
            name: "test".to_string(),
            iterations: 4,
            total_ns: 100,
            samples: vec![20, 25, 25, 30],
        };
        // mean=25, stddev≈4.082, stderr=4.082/2=2.041, margin≈4.0
        let (low, high) = result.confidence_interval_95();
        assert!((low - 21.0).abs() < 0.5, "low={}", low);
        assert!((high - 29.0).abs() < 0.5, "high={}", high);
    }

    #[test]
    fn test_confidence_interval_95_single_sample() {
        let result = BenchResult {
            name: "single".to_string(),
            iterations: 1,
            total_ns: 100,
            samples: vec![100],
        };
        let (low, high) = result.confidence_interval_95();
        assert_eq!(low, 100.0);
        assert_eq!(high, 100.0);
    }

    #[test]
    fn test_bench_with_setup_runs_setup_each_iteration() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let setup_calls = AtomicUsize::new(0);
        let f_calls = AtomicUsize::new(0);

        let result = bench_with_setup(
            "with_setup",
            25,
            || {
                setup_calls.fetch_add(1, Ordering::SeqCst);
                vec![3u32, 1, 2]
            },
            |mut v| {
                v.sort_unstable();
                black_box(v);
                f_calls.fetch_add(1, Ordering::SeqCst);
            },
        );

        assert_eq!(result.samples.len(), 25);
        assert_eq!(setup_calls.load(Ordering::SeqCst), 25);
        assert_eq!(f_calls.load(Ordering::SeqCst), 25);
    }

    #[test]
    fn test_bench_group_compare() {
        let mut group = BenchGroup::new("g");
        group.add("fast", 30, || {
            let _: u64 = black_box((0..10).sum());
        });
        group.add("slow", 30, || {
            let _: u64 = black_box((0..1000).sum());
        });

        let cmp = group.compare("slow", "fast").expect("both names present");
        assert_eq!(cmp.baseline.name, "slow");
        assert_eq!(cmp.candidate.name, "fast");

        assert!(group.compare("slow", "missing").is_none());
        assert!(group.compare("missing", "fast").is_none());
    }

    #[test]
    fn test_empty_samples() {
        let result = BenchResult {
            name: "empty".to_string(),
            iterations: 0,
            total_ns: 0,
            samples: vec![],
        };
        assert_eq!(result.mean_ns(), 0.0);
        assert_eq!(result.median_ns(), 0.0);
        assert_eq!(result.stddev_ns(), 0.0);
        assert_eq!(result.min_ns(), 0);
        assert_eq!(result.max_ns(), 0);
        assert_eq!(result.p95_ns(), 0.0);
        assert_eq!(result.p99_ns(), 0.0);
        assert_eq!(result.ops_per_sec(), 0.0);
    }
}
