use std::time::{Duration, Instant};

enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: CircuitState,
    failures: u32,
    last_fail: Option<Instant>,
    threshold: u32,
    reset_duration: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_seconds: u64) -> Self {
        Self {
            state: CircuitState::Closed,
            failures: 0,
            last_fail: None,
            threshold,
            reset_duration: Duration::from_secs(reset_seconds),
        }
    }

    pub fn is_open(&mut self) -> bool {
        match self.state {
            CircuitState::Open => {
                if let Some(last) = self.last_fail {
                    if last.elapsed() > self.reset_duration {
                        self.state = CircuitState::HalfOpen;
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn record_success(&mut self) {
        self.failures = 0;
        self.state = CircuitState::Closed;
    }

    pub fn record_failure(&mut self) {
        self.failures += 1;
        self.last_fail = Some(Instant::now());
        if self.failures >= self.threshold {
            self.state = CircuitState::Open;
        }
    }

    pub fn failure_count(&self) -> u32 {
        self.failures
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.state, CircuitState::Closed)
    }
}

pub struct CircuitBreakerPool {
    breakers: Vec<(String, CircuitBreaker)>,
}

impl CircuitBreakerPool {
    pub fn new() -> Self {
        Self { breakers: Vec::new() }
    }

    pub fn get(&mut self, key: &str) -> &mut CircuitBreaker {
        let pos = self.breakers.iter().position(|(k, _)| k == key);
        let idx = pos.unwrap_or_else(|| {
            self.breakers.push((key.to_string(), CircuitBreaker::new(3, 30)));
            self.breakers.len() - 1
        });
        &mut self.breakers[idx].1
    }
}
