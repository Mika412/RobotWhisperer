use chrono::{DateTime, Utc};

#[cfg(any(test, feature = "test-support"))]
use std::sync::atomic::{AtomicU64, Ordering};

pub trait Clock: Send + Sync + 'static {
    fn now(&self) -> DateTime<Utc>;
    fn now_monotonic_ns(&self) -> u64;
}

pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn now_monotonic_ns(&self) -> u64 {
        static ANCHOR: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        let anchor = ANCHOR.get_or_init(std::time::Instant::now);
        anchor.elapsed().as_nanos() as u64
    }
}

#[cfg(any(test, feature = "test-support"))]
pub struct MockClock {
    wall: std::sync::Mutex<DateTime<Utc>>,
    monotonic: AtomicU64,
}

#[cfg(any(test, feature = "test-support"))]
impl MockClock {
    pub fn new(initial: DateTime<Utc>) -> Self {
        Self {
            wall: std::sync::Mutex::new(initial),
            monotonic: AtomicU64::new(0),
        }
    }

    pub fn set(&self, time: DateTime<Utc>) {
        *self.wall.lock().expect("clock mutex poisoned") = time;
    }

    pub fn advance(&self, duration: chrono::Duration) {
        let mut wall = self.wall.lock().expect("clock mutex poisoned");
        *wall += duration;
        let nanos = duration.num_nanoseconds().unwrap_or_default().max(0) as u64;
        self.monotonic.fetch_add(nanos, Ordering::Relaxed);
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        *self.wall.lock().expect("clock mutex poisoned")
    }

    fn now_monotonic_ns(&self) -> u64 {
        self.monotonic.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed(year: i32, month: u32, day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap()
    }

    #[test]
    fn mock_clock_returns_initial_time() {
        let clock = MockClock::new(fixed(2026, 1, 1));
        assert_eq!(clock.now(), fixed(2026, 1, 1));
    }

    #[test]
    fn mock_clock_set_overrides_wall_time() {
        let clock = MockClock::new(fixed(2026, 1, 1));
        clock.set(fixed(2030, 6, 15));
        assert_eq!(clock.now(), fixed(2030, 6, 15));
    }

    #[test]
    fn mock_clock_advance_moves_wall_and_monotonic() {
        let clock = MockClock::new(fixed(2026, 1, 1));
        clock.advance(chrono::Duration::seconds(5));
        assert_eq!(
            clock.now(),
            fixed(2026, 1, 1) + chrono::Duration::seconds(5)
        );
        assert!(clock.now_monotonic_ns() > 0);
    }

    #[test]
    fn mock_clock_monotonic_is_strictly_increasing() {
        let clock = MockClock::new(fixed(2026, 1, 1));
        let first = clock.now_monotonic_ns();
        let second = clock.now_monotonic_ns();
        assert!(second > first);
    }

    #[test]
    fn system_clock_monotonic_does_not_decrease() {
        let clock = SystemClock::new();
        let first = clock.now_monotonic_ns();
        let second = clock.now_monotonic_ns();
        assert!(second >= first);
    }
}
