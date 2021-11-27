use std::time::Instant;

#[derive(Clone)]
pub struct TimeState {
    app_start: Instant,
    previous_update: Instant,
}

impl TimeState {
    pub fn new() -> TimeState {
        let now = Instant::now();

        TimeState {
            app_start: now,
            previous_update: now,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        self.previous_update = now;
    }
}
