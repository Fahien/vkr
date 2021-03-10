// Copyright © 2020-2021
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use std::time::{Duration, Instant};

/// Useful timer to get delta time, and previous time
pub struct Timer {
    prev: Instant,
    curr: Instant,
}

impl Timer {
    pub fn new() -> Self {
        let prev = Instant::now();
        let curr = Instant::now();
        Self { prev, curr }
    }

    /// Returns delta time in seconds
    pub fn get_delta(&mut self) -> Duration {
        self.curr = Instant::now();
        let delta = self.curr - self.prev;
        self.prev = self.curr;
        delta
    }

    /// Returns the time of last `get_delta()`
    pub fn _get_prev(&self) -> Instant {
        self.prev
    }
}
