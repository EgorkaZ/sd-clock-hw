#![feature(hash_raw_entry)]

pub mod events_statistic;
pub use events_statistic::{EventsStatistic, HourlyEventStatistic};

#[cfg(test)]
mod test {
    use std::{sync::Arc, time::Duration, collections::HashMap};

    use approx::assert_abs_diff_eq;
    use quanta::Mock;
    use rand::{rngs::ThreadRng, Rng, SeedableRng};
    use rstest::{fixture, rstest};

    use crate::{EventsStatistic, HourlyEventStatistic};

    struct Helper {
        stats: HourlyEventStatistic,
        clock: Arc<Mock>,
        events: Vec<&'static str>,
    }

    impl Helper {
        fn events(&self) -> impl Iterator<Item = &'static str> + '_ {
            self.events.iter().copied()
        }

        fn each_equals(&self, value: f64) {
            for (event, stat) in self.stats.get_all_event_statistic() {
                assert_abs_diff_eq!(value, stat);
            }
        }
    }

    const MINUTE: Duration = Duration::from_secs(60);
    const HOUR: Duration = Duration::from_secs(60 * 60);

    #[fixture]
    fn helper() -> Helper {
        let (stats, mock) = HourlyEventStatistic::with_mocked_clock();
        mock.increment(HOUR * 24);
        Helper { stats, clock: mock, events: vec!["event 1", "event 2", "event 3", "event 4"] }
    }

    #[rstest]
    fn zero_stats(helper: Helper) {
        helper.each_equals(0.);
    }

    #[rstest]
    fn single_event_per_minute(mut helper: Helper) {
        for _ in 0..60 {
            helper.clock.increment(Duration::from_secs(60));
            for event in helper.events.iter() {
                helper.stats.inc_event(event);
            }
        }

        helper.each_equals(1.);

        helper.clock.increment(Duration::from_nanos(1));

        for (event, stat) in helper.stats.get_all_event_statistic() {
            assert!(stat < 1., "expected rpm < 1. for {event}, actual: {stat}");
        }
    }

    #[rstest]
    fn vanished_after_hour(mut helper: Helper) {
        helper.stats.inc_event("event");
        assert!(helper.stats.get_event_statistic_by_name("event") > 0., "expected to have some rpm");

        helper.clock.increment(HOUR);
        assert_abs_diff_eq!(helper.stats.get_event_statistic_by_name("event"), 0.);
    }

    #[rstest]
    fn random_test(mut helper: Helper) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut events = [("event 0", 0), ("event 1", 1)];

        // Intervals up to a second. Total up to an hour
        let mut total_dur: Duration = Duration::default();
        let total_count = rng.gen_range(0..(60 * 60));
        println!("Total count {total_count}");
        for _ in 0..rng.gen_range(0..(60 * 60)) {
            let micros = Duration::from_micros(rng.gen_range(0..1_000_000));
            total_dur += micros;
            helper.clock.increment(micros);

            let (key, count) = &mut events[rng.gen_range(0..2)];
            helper.stats.inc_event(key);
            *count += 1;
        }

        println!("Total passed: {total_dur:?}");

        let all_stats = helper.stats.get_all_event_statistic();
        assert_eq!(all_stats.len(), 2);
        for (event, count) in events {
            assert!(all_stats.get(event).is_some());
            println!("checking \"{event}\", count: {count}");
            assert_abs_diff_eq!(*all_stats.get(event).unwrap() as f64, count as f64 / 60.);
            assert_abs_diff_eq!(helper.stats.get_event_statistic_by_name(event), count as f64 / 60.);
        }
    }
}
