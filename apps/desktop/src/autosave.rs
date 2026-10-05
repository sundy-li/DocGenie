//! Trailing-edge save deadline. Only document mutations restart this clock;
//! worker completion, UI signals and non-edit actions never postpone it.
use std::time::{Duration, Instant};

pub const DELAY: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct Autosave {
    deadline: Option<Instant>,
}
impl Autosave {
    pub fn changed(&mut self, now: Instant) {
        self.deadline = Some(now + DELAY);
    }
    pub fn ready(&self, now: Instant) -> bool {
        self.deadline.is_some_and(|deadline| now >= deadline)
    }
    pub fn clear(&mut self) {
        self.deadline = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn waits_three_seconds_after_last_edit() {
        let start = Instant::now();
        let mut save = Autosave::default();
        save.changed(start);
        assert!(!save.ready(start + Duration::from_millis(2999)));
        save.changed(start + Duration::from_secs(2));
        assert!(!save.ready(start + Duration::from_secs(3)));
        assert!(!save.ready(start + Duration::from_millis(4999)));
        assert!(save.ready(start + Duration::from_secs(5)));
    }
    #[test]
    fn completion_of_older_save_does_not_bypass_new_edit_deadline() {
        let start = Instant::now();
        let mut save = Autosave::default();
        save.changed(start);
        assert!(save.ready(start + DELAY));
        save.clear(); // dispatch snapshot A
        save.changed(start + Duration::from_millis(3100)); // edit B during IO
        assert!(!save.ready(start + Duration::from_millis(3200))); // A finishes
        assert!(save.ready(start + Duration::from_millis(6100)));
    }
    #[test]
    fn expired_deadline_survives_busy_worker_until_completion() {
        let start = Instant::now();
        let mut save = Autosave::default();
        save.changed(start);
        assert!(save.ready(start + Duration::from_secs(4)));
        assert!(save.ready(start + Duration::from_secs(6)));
        save.clear();
        assert!(!save.ready(start + Duration::from_secs(7)));
    }
}
