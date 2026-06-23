use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for the server-side retry budget.
///
/// The budget limits the total fraction of requests that can be retried,
/// preventing retry storms from amplifying load on downstream services.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum fraction of total requests that can be retries (e.g., 0.02 = 2%).
    pub budget_ratio: f64,
    /// Sliding window duration for tracking request/retry counts.
    pub window: Duration,
    /// Maximum number of retry attempts per individual request.
    pub max_retries_per_request: u32,
    /// Minimum number of requests in the window before retries are allowed.
    /// Prevents retrying when traffic is too low to be statistically meaningful.
    pub min_requests_per_window: u64,
    /// HTTP status codes that trigger a retry attempt.
    pub retry_on_status: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            budget_ratio: 0.02,
            window: Duration::from_secs(10),
            max_retries_per_request: 1,
            min_requests_per_window: 100,
            retry_on_status: vec![500, 502, 503],
        }
    }
}

/// A token-bucket style retry budget using a sliding window.
///
/// Tracks total requests and retries over a configurable time window.
/// Allows a retry only if the retry ratio remains under the configured budget.
#[derive(Debug, Clone)]
pub struct RetryBudget {
    config: RetryConfig,
    state: Arc<BudgetState>,
}

#[derive(Debug)]
struct BudgetState {
    /// Total requests in current window
    requests: AtomicU64,
    /// Total retries in current window
    retries: AtomicU64,
    /// Start of current window
    window_start: std::sync::Mutex<Instant>,
}

impl RetryBudget {
    /// Create a new retry budget with the given configuration.
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            state: Arc::new(BudgetState {
                requests: AtomicU64::new(0),
                retries: AtomicU64::new(0),
                window_start: std::sync::Mutex::new(Instant::now()),
            }),
        }
    }

    /// Record an incoming request.
    pub fn record_request(&self) {
        self.maybe_reset_window();
        self.state.requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Check if a retry is allowed under the current budget, and if so, record it.
    ///
    /// Returns `true` if the retry is permitted.
    pub fn try_acquire_retry(&self) -> bool {
        self.maybe_reset_window();

        let requests = self.state.requests.load(Ordering::Relaxed);
        let retries = self.state.retries.load(Ordering::Relaxed);

        // Don't retry if we haven't seen enough traffic
        if requests < self.config.min_requests_per_window {
            return false;
        }

        // Check if retry ratio would exceed budget
        let total = requests + retries;
        if total == 0 {
            return false;
        }

        let current_ratio = (retries + 1) as f64 / (total + 1) as f64;
        if current_ratio > self.config.budget_ratio {
            return false;
        }

        self.state.retries.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Returns whether the given status code should trigger a retry.
    pub fn should_retry_status(&self, status: u16) -> bool {
        self.config.retry_on_status.contains(&status)
    }

    /// Maximum retries per individual request.
    pub fn max_retries_per_request(&self) -> u32 {
        self.config.max_retries_per_request
    }

    fn maybe_reset_window(&self) {
        let mut window_start = self.state.window_start.lock().unwrap();
        if window_start.elapsed() >= self.config.window {
            *window_start = Instant::now();
            self.state.requests.store(0, Ordering::Relaxed);
            self.state.retries.store(0, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_allows_retry_when_under_ratio() {
        let budget = RetryBudget::new(RetryConfig {
            budget_ratio: 0.10,
            window: Duration::from_secs(10),
            max_retries_per_request: 1,
            min_requests_per_window: 10,
            retry_on_status: vec![500],
        });

        // Record enough requests to meet the minimum
        for _ in 0..100 {
            budget.record_request();
        }

        // Should allow retry (1/101 = ~1% < 10%)
        assert!(budget.try_acquire_retry());
    }

    #[test]
    fn test_budget_denies_retry_below_min_requests() {
        let budget = RetryBudget::new(RetryConfig {
            budget_ratio: 0.10,
            window: Duration::from_secs(10),
            max_retries_per_request: 1,
            min_requests_per_window: 100,
            retry_on_status: vec![500],
        });

        // Only 5 requests — below minimum
        for _ in 0..5 {
            budget.record_request();
        }

        assert!(!budget.try_acquire_retry());
    }

    #[test]
    fn test_budget_denies_when_over_ratio() {
        let budget = RetryBudget::new(RetryConfig {
            budget_ratio: 0.02,
            window: Duration::from_secs(10),
            max_retries_per_request: 1,
            min_requests_per_window: 10,
            retry_on_status: vec![500],
        });

        // 100 requests
        for _ in 0..100 {
            budget.record_request();
        }

        // First retry: 1/101 ≈ 0.99% < 2% → allowed
        assert!(budget.try_acquire_retry());

        // Second retry: 2/102 ≈ 1.96% < 2% → allowed
        assert!(budget.try_acquire_retry());

        // Third retry: 3/103 ≈ 2.9% > 2% → denied
        assert!(!budget.try_acquire_retry());
    }

    #[test]
    fn test_should_retry_status() {
        let budget = RetryBudget::new(RetryConfig {
            retry_on_status: vec![500, 502, 503],
            ..Default::default()
        });

        assert!(budget.should_retry_status(500));
        assert!(budget.should_retry_status(502));
        assert!(!budget.should_retry_status(400));
        assert!(!budget.should_retry_status(404));
    }
}
