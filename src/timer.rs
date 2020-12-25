use std::time::{Duration, SystemTime};

/// Struct to model the behaviour of a timer.
/// Used to calculate the time taken to execute
/// a particular block of code
pub struct Timer {
    start_time: SystemTime
}

impl Timer {
    /// Method to create a new timer with current time as
    /// the start time
    pub fn new() -> Self {
        return Timer { start_time: Timer::now() };
    }

    /// Static method to get the current time
    ///
    /// # Examples
    /// ```no_run
    /// let current_time = Timer::now();
    /// ```
    fn now() -> SystemTime {
        return SystemTime::now();
    }

    /// Returns the time ellapsed from the time that the instance
    /// of the struct has been initialized
    ///
    /// # Panics
    /// This function will panic if the `current_time` is earlier
    /// than the `start_time`
    ///
    /// # Examples
    /// ```no_run
    /// let timer = Timer::new();
    /// println!("{:?}", timer.ellapsed());
    /// ```
    pub fn ellapsed(&self) -> Duration {
        let current_time = Timer::now();
        return current_time.duration_since(self.start_time).unwrap();
    }
}