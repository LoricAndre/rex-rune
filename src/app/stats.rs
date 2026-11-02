//! `REX_FRAME_STATS=1`: per-frame CPU (update + encode) percentiles on stderr every 120 drawn
//! frames or two seconds, whichever comes first.
use rune::{FrameEncoded, FrameOutcome};
use std::time::Instant;

#[derive(Default)]
pub struct FrameStats {
    samples: Vec<f64>,
    update: Vec<f64>,
    since: Option<Instant>,
}

impl FrameStats {
    pub fn enabled() -> bool {
        std::env::var_os("REX_FRAME_STATS").is_some_and(|v| v != "0")
    }

    pub fn record(&mut self, f: &FrameEncoded) {
        if f.outcome() != FrameOutcome::Drawn {
            return;
        }
        self.samples
            .push(f.update_ms().unwrap_or(0.) + f.encode_ms().unwrap_or(0.));
        self.update.push(f.update_ms().unwrap_or(0.));
        let since = *self.since.get_or_insert_with(Instant::now);
        if self.samples.len() < 120
            && (self.samples.len() < 5 || since.elapsed().as_secs_f64() < 2.)
        {
            return;
        }
        self.since = None;
        let mut s = std::mem::take(&mut self.samples);
        s.sort_by(f64::total_cmp);
        let mut u = std::mem::take(&mut self.update);
        u.sort_by(f64::total_cmp);
        let update_p95 = u[((u.len() - 1) as f64 * 0.95).round() as usize];
        let at = |q: f64| s[((s.len() - 1) as f64 * q).round() as usize];
        eprintln!(
            "rex frame-stats n={} p50={:.2}ms p95={:.2}ms max={:.2}ms update-p95={update_p95:.2}ms",
            s.len(),
            at(0.5),
            at(0.95),
            s[s.len() - 1]
        );
    }
}
