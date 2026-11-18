//! `--tic-stats`: a measurement aid. Keeps the monotonic time of each executed tic in memory
//! and summarises the intervals on exit. Nothing on the simulation path reads it.

/// Executed-tic times (microseconds on the loop's monotonic clock) and the dropped count.
#[derive(Default)]
pub struct TicStats {
    times: Vec<u64>,
    dropped: u64,
}

impl TicStats {
    /// Notes one `App::update` at `now_us`: `ran` tics ran and `dropped` were dropped.
    pub fn record(&mut self, now_us: u64, ran: u64, dropped: u64) {
        self.times.extend((0..ran).map(|_| now_us));
        self.dropped += dropped;
    }

    pub fn summary(&self) -> String {
        summarize(&self.times, self.dropped)
    }
}

/// The summary text: tics, wall time against tics/35, intervals in microseconds.
pub fn summarize(times: &[u64], dropped: u64) -> String {
    let n = times.len();
    if n < 2 {
        return format!("tic-stats: executed {n}, dropped {dropped}, too few tics for intervals");
    }
    let mut d: Vec<u64> = times.windows(2).map(|w| w[1] - w[0]).collect();
    let wall = (times[n - 1] - times[0]) as f64 / 1e6;
    let mean = d.iter().sum::<u64>() as f64 / d.len() as f64;
    let sd = (d.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / d.len() as f64).sqrt();
    d.sort_unstable();
    let at = |p: usize| d[((d.len() - 1) * p).div_ceil(100)];
    let ideal = 1e6 / 35.0;
    let off = |us: f64| d.iter().filter(|&&x| (x as f64 - ideal).abs() > us).count();
    format!(
        "tic-stats: executed {n}, dropped {dropped}, wall {wall:.3} s (first to last tic) against {:.3} s expected; \
         intervals us: mean {mean:.1}, sd {sd:.1}, min {}, median {}, p95 {}, p99 {}, max {}; \
         more than 1 ms from 1/35 s: {}, more than 5 ms: {}",
        (n - 1) as f64 / 35.0,
        d[0],
        at(50),
        at(95),
        at(99),
        d[d.len() - 1],
        off(1000.0),
        off(5000.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_arithmetic() {
        // intervals 28000, 29000, 30000 us; ideal 28571.4
        let s = summarize(&[0, 28_000, 57_000, 87_000], 2);
        for want in [
            "executed 4, dropped 2",
            "wall 0.087 s",
            "against 0.086 s",
            "mean 29000.0, sd 816.5, min 28000, median 29000, p95 30000, p99 30000, max 30000",
            "more than 1 ms from 1/35 s: 1, more than 5 ms: 0",
        ] {
            assert!(s.contains(want), "{want} not in {s}");
        }
        assert!(summarize(&[5], 0).contains("too few"));
        let mut t = TicStats::default();
        t.record(10, 2, 1);
        t.record(20, 0, 3);
        assert_eq!((t.times.clone(), t.dropped), (vec![10, 10], 4));
    }
}
