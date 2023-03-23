use std::{collections::{HashMap, VecDeque}, sync::Arc, time::Duration};

use quanta::{Instant, Clock};

pub trait EventsStatistic {
    fn inc_event(&mut self, name: &str);

    fn get_event_statistic_by_name(&self, name: &str) -> f64;
    fn get_all_event_statistic(&self) -> HashMap<&str, f64>;

    fn print_statistic(&self);
}

#[derive(Debug)]
struct HourlyStat {
    curr_hour: VecDeque<Instant>,
    total_count: u64,
    first_timestamp: Instant,
}

impl HourlyStat {
    fn with_timestamp(ts: Instant) -> Self {
        HourlyStat { first_timestamp: ts, curr_hour: Default::default(), total_count: 0 }
    }
}

#[derive(Debug, Default)]
pub struct HourlyEventStatistic {
    timestamps: HashMap<String, HourlyStat>,
    clock: Clock,
}

impl HourlyEventStatistic {
    pub fn new() -> Self {
        Default::default()
    }

    #[allow(dead_code)]
    pub(crate) fn with_mocked_clock() -> (Self, Arc<quanta::Mock>) {
        let (clock, mock) = Clock::mock();
        let stats = HourlyEventStatistic {
            timestamps: Default::default(),
            clock
        };
        (stats, mock)
    }

    const MINUTE: Duration = Duration::from_secs(60);
    const HOUR: Duration = Duration::from_secs(60 * 60);
    const MINUTES_IN_HOUR: u64 = 60;
}

impl EventsStatistic for HourlyEventStatistic {
    fn inc_event(&mut self, name: &str) {
        let now = self.clock.now();
        let (_, stat) = self.timestamps.raw_entry_mut().from_key(name)
            .or_insert_with(|| (name.into(), HourlyStat::with_timestamp(now)));

        let hour_ago = now - Self::HOUR;
        while let Some(ts) = stat.curr_hour.front() {
            if *ts > hour_ago {
                break;
            }
            stat.curr_hour.pop_front();
        }
        stat.total_count += 1;
        stat.curr_hour.push_back(now);
    }

    fn get_event_statistic_by_name(&self, name: &str) -> f64 {
        let now = self.clock.now();
        let hour_ago = now - Self::HOUR;
        self.timestamps.get(name)
            .map(|stat| (stat, stat.curr_hour.iter()
                .take_while(|ts| **ts <= hour_ago)
                .count() as u64)
            )
            .map(|(stat, old_events)| {
                let curr_hour_events = stat.total_count - old_events;
                curr_hour_events as f64 / Self::MINUTES_IN_HOUR as f64
            })
            .unwrap_or(0.)
    }

    fn get_all_event_statistic(&self) -> HashMap<&str, f64> {
        self.timestamps.keys()
            .map(|event_name| (&event_name[..], self.get_event_statistic_by_name(event_name)))
            .collect()
    }

    fn print_statistic(&self) {
        let now = self.clock.now();
        for (event_name, stat) in self.timestamps.iter() {
            let total_lifetime = now - stat.first_timestamp;
            let minutes = total_lifetime.as_nanos() / Self::MINUTE.as_nanos();
            let rpm = stat.total_count as f64 / minutes as f64;
            println!("{event_name}: {rpm}");
        }
    }
}
