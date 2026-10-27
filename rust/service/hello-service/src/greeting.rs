//! The hello_world greeting cycle.
//!
//! The greeting advances through a fixed Latin-script list. The interval
//! between changes is randomly chosen from 15 to 60 seconds.

use std::time::{Duration, Instant};

pub(crate) const GREETINGS: &[&str] = &[
    "Hello, world!",
    "Hola, mundo!",
    "Bonjour le monde!",
    "Hallo, Welt!",
    "Ciao, mondo!",
    "Olá, mundo!",
    "Hallo, wereld!",
    "Hej, världen!",
    "Hei, maailma!",
    "Witaj, świecie!",
    "Merhaba dünya!",
    "Salut, lume!",
    "Salve, munde!",
    "toki, ma o!",
    "Halo, wol!",
    "Habari, dunia!",
    "Pẹlẹ o, ayé!",
];

const MIN_CHANGE_SECS: u64 = 15;
const MAX_CHANGE_SECS: u64 = 60;

/// Advances through [`GREETINGS`] at random 15-60 second intervals.
pub(crate) struct GreetingCycle {
    index: usize,
    next_change: Instant,
    rng: SplitMix64,
}

impl GreetingCycle {
    pub(crate) fn new(now: Instant) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Self::with_seed(now, seed)
    }

    fn with_seed(now: Instant, seed: u64) -> Self {
        let mut rng = SplitMix64(seed);
        let wait = random_interval(&mut rng);
        Self {
            index: 0,
            next_change: now + wait,
            rng,
        }
    }

    pub(crate) fn current(&self) -> &'static str {
        GREETINGS[self.index]
    }

    /// Advance as many times as the elapsed time requires. Returns true when
    /// the greeting changed.
    pub(crate) fn advance_if_due(&mut self, now: Instant) -> bool {
        let mut changed = false;
        while now >= self.next_change {
            self.index = (self.index + 1) % GREETINGS.len();
            self.next_change += random_interval(&mut self.rng);
            changed = true;
        }
        changed
    }
}

fn random_interval(rng: &mut SplitMix64) -> Duration {
    let span = MAX_CHANGE_SECS - MIN_CHANGE_SECS + 1;
    Duration::from_secs(MIN_CHANGE_SECS + rng.next_u64() % span)
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_are_within_the_configured_range() {
        let now = Instant::now();
        let mut cycle = GreetingCycle::with_seed(now, 1);
        let mut previous = now;
        for _ in 0..100 {
            let wait = cycle.next_change.saturating_duration_since(previous);
            assert!(
                wait >= Duration::from_secs(MIN_CHANGE_SECS)
                    && wait <= Duration::from_secs(MAX_CHANGE_SECS),
                "wait {wait:?} out of range"
            );
            previous = cycle.next_change;
            let next = cycle.next_change + Duration::from_millis(1);
            cycle.advance_if_due(next);
        }
    }

    #[test]
    fn advances_sequentially_and_wraps() {
        let now = Instant::now();
        let mut cycle = GreetingCycle::with_seed(now, 2);
        assert_eq!(cycle.current(), GREETINGS[0]);
        for offset in 1..=GREETINGS.len() {
            let next = cycle.next_change + Duration::from_millis(1);
            assert!(cycle.advance_if_due(next));
            assert_eq!(cycle.current(), GREETINGS[offset % GREETINGS.len()]);
        }
    }

    #[test]
    fn does_not_advance_early() {
        let now = Instant::now();
        let mut cycle = GreetingCycle::with_seed(now, 3);
        assert!(!cycle.advance_if_due(now));
        assert_eq!(cycle.current(), GREETINGS[0]);
    }
}
