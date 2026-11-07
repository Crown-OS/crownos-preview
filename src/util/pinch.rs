use std::time::{Duration, Instant};

/// Updates further apart than this belong to separate pinches.
const GESTURE_GAP: Duration = Duration::from_millis(150);

/// Turns a pinch's scale since it began into the change since its last update.
#[derive(Debug, Default)]
pub struct PinchTracker {
    previous: Option<(f32, Instant)>,
}

impl PinchTracker {
    pub fn step(&mut self, scale: f32) -> f32 {
        self.step_at(scale, Instant::now())
    }

    fn step_at(&mut self, scale: f32, now: Instant) -> f32 {
        let base = self
            .previous
            .filter(|(_, seen)| now.duration_since(*seen) < GESTURE_GAP)
            .map_or(1.0, |(previous, _)| previous);
        self.previous = Some((scale, now));
        if base > 0.0 && scale > 0.0 {
            scale / base
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_relative_to_the_previous_update() {
        let start = Instant::now();
        let mut tracker = PinchTracker::default();
        assert_eq!(tracker.step_at(1.5, start), 1.5);
        assert_eq!(tracker.step_at(3.0, start + Duration::from_millis(10)), 2.0);
    }

    #[test]
    fn a_pause_starts_a_new_pinch() {
        let start = Instant::now();
        let mut tracker = PinchTracker::default();
        tracker.step_at(2.0, start);
        assert_eq!(tracker.step_at(1.2, start + GESTURE_GAP), 1.2);
    }

    #[test]
    fn a_degenerate_scale_leaves_the_zoom_alone() {
        let mut tracker = PinchTracker::default();
        assert_eq!(tracker.step(0.0), 1.0);
        assert_eq!(tracker.step(1.0), 1.0);
    }
}
