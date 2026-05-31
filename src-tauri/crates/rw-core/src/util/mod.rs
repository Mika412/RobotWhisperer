pub mod clock;

pub use clock::{Clock, SystemClock};

#[cfg(any(test, feature = "test-support"))]
pub use clock::MockClock;
