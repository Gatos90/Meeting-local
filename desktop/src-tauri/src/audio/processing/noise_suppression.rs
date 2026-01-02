// Audio Processing - RNNoise Noise Suppression
use anyhow::Result;
use log::info;
use nnnoiseless::DenoiseState;

/// RNNoise-based noise suppression processor
pub struct NoiseSuppressionProcessor {
    denoiser: DenoiseState<'static>,
    frame_buffer: Vec<f32>,
    frame_size: usize,
}

impl NoiseSuppressionProcessor {
    pub fn new(sample_rate: u32) -> Result<Self> {
        if sample_rate != 48000 {
            return Err(anyhow::anyhow!(
                "Noise suppression requires 48kHz sample rate, got {}Hz",
                sample_rate
            ));
        }

        const FRAME_SIZE: usize = DenoiseState::FRAME_SIZE;

        info!("Initializing RNNoise noise suppression (frame size: {} samples, 10ms @ 48kHz)", FRAME_SIZE);

        Ok(Self {
            denoiser: *DenoiseState::new(),
            frame_buffer: Vec::with_capacity(FRAME_SIZE * 2),
            frame_size: FRAME_SIZE,
        })
    }

    pub fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        if samples.is_empty() {
            return Vec::new();
        }

        let input_len = samples.len();
        self.frame_buffer.extend_from_slice(samples);

        let mut output = Vec::with_capacity(input_len);

        while self.frame_buffer.len() >= self.frame_size {
            let frame: Vec<f32> = self.frame_buffer.drain(0..self.frame_size).collect();
            let mut denoised_frame = vec![0.0f32; self.frame_size];
            let _vad_prob = self.denoiser.process_frame(&mut denoised_frame, &frame);
            output.extend_from_slice(&denoised_frame);
        }

        output
    }

    pub fn buffered_samples(&self) -> usize {
        self.frame_buffer.len()
    }

    pub fn flush(&mut self) -> Vec<f32> {
        if self.frame_buffer.is_empty() {
            return Vec::new();
        }

        let remaining = self.frame_buffer.len();
        let mut input_frame = self.frame_buffer.clone();
        if input_frame.len() < self.frame_size {
            input_frame.resize(self.frame_size, 0.0);
        }

        let mut output = vec![0.0f32; self.frame_size];
        self.denoiser.process_frame(&mut output, &input_frame);
        self.frame_buffer.clear();

        output.truncate(remaining);
        output
    }
}
