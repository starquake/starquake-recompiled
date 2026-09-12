//! Beeper sound: turning the game's speaker toggles into samples, and
//! playing them.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const CPU_HZ: f64 = 3_500_000.0;
const VOLUME: f32 = 0.25;

/// Converts speaker levels over time (in T-states) into samples.
pub struct Beeper {
    rate: f64,
    level: bool,
    /// T-states into the current sample, and the level integrated over it.
    sample_t: f64,
    acc: f64,
    samples: Vec<f32>,
    /// Simple DC blocker, so a speaker left high does not sit off-centre.
    dc_in: f32,
    dc_out: f32,
}

impl Beeper {
    pub fn new(rate: u32) -> Beeper {
        Beeper {
            rate: rate as f64,
            level: false,
            sample_t: 0.0,
            acc: 0.0,
            samples: Vec::new(),
            dc_in: 0.0,
            dc_out: 0.0,
        }
    }

    /// Holds the current level for `t` T-states.
    fn advance(&mut self, mut t: f64) {
        let per_sample = CPU_HZ / self.rate;
        let level = if self.level { 1.0 } else { -1.0 };
        while t > 0.0 {
            let step = t.min(per_sample - self.sample_t);
            self.acc += level * step;
            self.sample_t += step;
            t -= step;
            if self.sample_t >= per_sample {
                let x = (self.acc / per_sample) as f32 * VOLUME;
                let y = x - self.dc_in + 0.995 * self.dc_out;
                self.dc_in = x;
                self.dc_out = y;
                self.samples.push(y);
                self.sample_t = 0.0;
                self.acc = 0.0;
            }
        }
    }

    /// Plays blocking sound effects; returns how many T-states they took.
    pub fn effects(&mut self, ram: &[u8], ids: &[u8]) -> u32 {
        let mut total = 0;
        for &id in ids {
            let (edges, duration) = starquake::sound::beep(ram, id);
            let mut now = 0;
            for (t, level) in edges {
                self.advance((t - now) as f64);
                now = t;
                self.level = level;
            }
            self.advance((duration - now) as f64);
            self.level = false;
            total += duration;
        }
        total
    }

    /// Plays a tune's speaker changes over `t` T-states.
    pub fn music(&mut self, edges: &[(u32, bool)], t: u32) {
        let mut now = 0;
        for &(at, level) in edges {
            let at = at.min(t);
            self.advance((at - now) as f64);
            now = at;
            self.level = level;
        }
        self.advance((t - now) as f64);
    }

    /// Plays the frame tone (or silence) for `t` T-states.
    pub fn tone(&mut self, half_period: Option<u8>, t: u32) {
        match half_period {
            None => self.advance(t as f64),
            Some(hp) => {
                let half = starquake::sound::tone_half_period(hp);
                let mut left = t;
                while left > 0 {
                    self.level = !self.level;
                    let step = half.min(left);
                    self.advance(step as f64);
                    left -= step;
                }
            }
        }
    }

    pub fn take_samples(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.samples)
    }
}

/// The sound card, fed through a queue of mono samples.
pub struct Output {
    queue: Arc<Mutex<VecDeque<f32>>>,
    rate: u32,
}

impl Output {
    pub fn start() -> Result<(Output, cpal::Stream), String> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or("no output device")?;
        let supported = device.default_output_config().map_err(|e| e.to_string())?;
        let config: cpal::StreamConfig = supported.config();
        let channels = config.channels as usize;
        let rate = config.sample_rate;
        let queue = Arc::new(Mutex::new(VecDeque::<f32>::new()));
        let q = queue.clone();
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| {
                    let mut q = q.lock().unwrap();
                    for frame in data.chunks_mut(channels) {
                        let v = q.pop_front().unwrap_or(0.0);
                        frame.fill(v);
                    }
                },
                |e| eprintln!("sound error: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok((Output { queue, rate }, stream))
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn push(&self, samples: &[f32]) {
        let mut q = self.queue.lock().unwrap();
        q.extend(samples);
        // If the game ever runs ahead, drop the oldest rather than let the
        // sound fall behind the picture for good. The cap has to clear the
        // longest thing the game can hand over at once, or it would eat the
        // start of it: the death explosion is 1.44s, and a core piece going
        // in queues 25 effects together, near 2.9s.
        let cap = self.rate as usize * 4;
        if q.len() > cap {
            let excess = q.len() - cap;
            q.drain(..excess);
        }
    }

    pub fn queued(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
}
