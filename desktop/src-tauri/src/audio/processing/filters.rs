// Audio Processing - Filters
use log::info;

/// High-pass filter to remove low-frequency rumble and noise
pub struct HighPassFilter {
    #[allow(dead_code)]
    sample_rate: f32,
    #[allow(dead_code)]
    cutoff_hz: f32,
    alpha: f32,
    prev_input: f32,
    prev_output: f32,
}

impl HighPassFilter {
    pub fn new(sample_rate: u32, cutoff_hz: f32) -> Self {
        let sample_rate_f = sample_rate as f32;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
        let dt = 1.0 / sample_rate_f;
        let alpha = rc / (rc + dt);

        info!("Initializing high-pass filter: cutoff={}Hz @ {}Hz", cutoff_hz, sample_rate);

        Self {
            sample_rate: sample_rate_f,
            cutoff_hz,
            alpha,
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        let mut output = Vec::with_capacity(samples.len());

        for &sample in samples {
            let filtered = self.alpha * (self.prev_output + sample - self.prev_input);

            self.prev_input = sample;
            self.prev_output = filtered;

            output.push(filtered);
        }

        output
    }

    pub fn reset(&mut self) {
        self.prev_input = 0.0;
        self.prev_output = 0.0;
    }
}
