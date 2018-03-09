use futures::{Future, Stream, Async, Poll, task};

use {Sleep, TimerError};

use std::time::Duration;

/// A stream representing notifications at fixed interval
///
/// Intervals are created through `Timer::interval`.
#[derive(Debug)]
pub struct Interval {
    sleep: Sleep,
    duration: Duration,
}

/// Create a new interval
pub fn new(sleep: Sleep, dur: Duration) -> Interval {
    Interval {
        sleep: sleep,
        duration: dur,
    }
}

impl Stream for Interval {
    type Item = ();
    type Error = TimerError;

    fn poll_next(&mut self, cx: &mut task::Context) -> Poll<Option<()>, TimerError> {
        let _ = try_ready!(self.sleep.poll(cx));

        // Reset the timeout
        self.sleep = self.sleep.timer().sleep(self.duration);

        Ok(Async::Ready(Some(())))
    }
}
