use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct TimeState {
    app_start: Instant,
    previous_update: Instant,
    app_time_context: TimeContext,
}

const NANOS_PER_SEC: u32 = 1_000_000_000;

impl TimeState {
    pub fn new() -> TimeState {
        let now = Instant::now();

        TimeState {
            app_start: now,
            previous_update: now,
            app_time_context: TimeContext::new(),
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now - self.previous_update;
        self.previous_update = now;
        self.app_time_context.update(elapsed);
    }

    pub fn current_instant(&self) -> Instant {
        self.app_time_context.current_instant
    }

    pub fn updates_per_second(&self) -> f32 {
        self.app_time_context.updates_per_second
    }

    pub fn update_count(&self) -> u64 {
        self.app_time_context.update_count
    }

    pub fn updates_per_second_smoothed(&self) -> f32 {
        self.app_time_context.updates_per_second_smoothed
    }

    pub fn total_time(&self) -> Duration {
        self.app_time_context.total_time
    }
}

/// Tracks time passing, this is separate from the "global" `TimeState` since it would be
/// possible to track a separate "context" of time, for example "unpaused" time in a game
#[derive(Copy, Clone)]
pub struct TimeContext {
    total_time: Duration,
    current_instant: Instant,
    previous_update_time: Duration,
    previous_update_dt: f32,
    updates_per_second: f32,
    updates_per_second_smoothed: f32,
    update_count: u64,
}

impl TimeContext {
    /// Create a new TimeState. Default is not allowed because the current time affects the object
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let now_instant = Instant::now();
        let zero_duration = Duration::from_secs(0);
        TimeContext {
            total_time: zero_duration,
            current_instant: now_instant,
            previous_update_time: zero_duration,
            previous_update_dt: 0.0,
            updates_per_second: 0.0,
            updates_per_second_smoothed: 0.0,
            update_count: 0,
        }
    }

    /// Call to capture time passing and update values
    pub fn update(&mut self, elapsed: Duration) {
        self.total_time += elapsed;
        self.current_instant += elapsed;
        self.previous_update_time = elapsed;

        // this can eventually be replaced with as_float_secs
        let dt =
            (elapsed.as_secs() as f32) + (elapsed.subsec_nanos() as f32) / (NANOS_PER_SEC as f32);

        self.previous_update_dt = dt;

        let fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };

        //TODO: Replace with a circular buffer
        const SMOOTHING_FACTOR: f32 = 0.95;
        self.updates_per_second = fps;
        self.updates_per_second_smoothed = (self.updates_per_second_smoothed * SMOOTHING_FACTOR)
            + (fps * (1.0 - SMOOTHING_FACTOR));

        self.update_count += 1;
    }

    /// Duration of time passed in this time context
    pub fn total_time(&self) -> Duration {
        self.total_time
    }

    /// `rafx::base::Instant` object captured at the start of the most recent update in this time
    /// context
    pub fn current_instant(&self) -> Instant {
        self.current_instant
    }

    /// duration of time passed during the previous update
    pub fn previous_update_time(&self) -> Duration {
        self.previous_update_time
    }

    /// previous update time in f32 seconds
    pub fn previous_update_dt(&self) -> f32 {
        self.previous_update_dt
    }

    /// estimate of updates per second
    pub fn updates_per_second(&self) -> f32 {
        self.updates_per_second
    }

    /// estimate of updates per second smoothed over time
    pub fn updates_per_second_smoothed(&self) -> f32 {
        self.updates_per_second_smoothed
    }

    /// Total number of update in this time context
    pub fn update_count(&self) -> u64 {
        self.update_count
    }
}

/// Useful for cases where you want to do something once per time interval.
#[derive(Default)]
pub struct PeriodicEvent {
    last_time_triggered: Option<Instant>,
}

impl PeriodicEvent {
    /// Call try_take_event to see if the required time has elapsed. It will return true only once
    /// enough time has passed since it last returned true.
    pub fn try_take_event(&mut self, current_time: Instant, wait_duration: Duration) -> bool {
        match self.last_time_triggered {
            None => {
                self.last_time_triggered = Some(current_time);
                true
            }
            Some(last_time_triggered) => {
                if current_time - last_time_triggered >= wait_duration {
                    self.last_time_triggered = Some(current_time);
                    true
                } else {
                    false
                }
            }
        }
    }
}
