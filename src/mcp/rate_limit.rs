use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use uuid::Uuid;

use crate::error::AppError;

const WINDOW: Duration = Duration::from_secs(60);
const MAX_REQUESTS_PER_WINDOW: usize = 120;

#[derive(Clone, Default)]
pub struct McpRateLimiter {
    inner: Arc<Mutex<HashMap<Uuid, VecDeque<Instant>>>>,
}

impl McpRateLimiter {
    pub fn check(&self, token_id: Uuid) -> Result<(), AppError> {
        self.check_at(token_id, Instant::now(), MAX_REQUESTS_PER_WINDOW)
    }

    fn check_at(&self, token_id: Uuid, now: Instant, maximum: usize) -> Result<(), AppError> {
        let mut limits = self.inner.lock().expect("MCP rate limit mutex poisoned");
        limits.retain(|_, requests| {
            requests
                .back()
                .is_some_and(|request| now.duration_since(*request) < WINDOW)
        });
        let requests = limits.entry(token_id).or_default();
        while requests
            .front()
            .is_some_and(|request| now.duration_since(*request) >= WINDOW)
        {
            requests.pop_front();
        }
        if requests.len() >= maximum {
            return Err(AppError::RateLimited);
        }
        requests.push_back(now);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use uuid::Uuid;

    use super::McpRateLimiter;

    #[test]
    fn requests_are_bounded_per_token_and_reset_after_the_window() {
        let limiter = McpRateLimiter::default();
        let token = Uuid::new_v4();
        let now = Instant::now();
        assert!(limiter.check_at(token, now, 2).is_ok());
        assert!(limiter.check_at(token, now, 2).is_ok());
        assert!(limiter.check_at(token, now, 2).is_err());
        assert!(
            limiter
                .check_at(token, now + Duration::from_secs(61), 2)
                .is_ok()
        );
    }
}
