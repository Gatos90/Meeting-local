// Audio Processing - Spectral Operations
use anyhow::Result;
use log::warn;
use realfft::num_complex::{Complex32, ComplexFloat};
use realfft::RealFftPlanner;

pub fn spectral_subtraction(audio: &[f32], d: f32) -> Result<Vec<f32>> {
    let mut real_planner = RealFftPlanner::<f32>::new();
    let window_size = 1600;

    if audio.is_empty() {
        return Ok(Vec::new());
    }

    let processed_audio = if audio.len() > window_size {
        warn!("Audio length {} exceeds window size {}, truncating", audio.len(), window_size);
        &audio[..window_size]
    } else {
        audio
    };

    let r2c = real_planner.plan_fft_forward(window_size);
    let mut y = r2c.make_output_vec();

    let mut padded_audio = processed_audio.to_vec();
    if processed_audio.len() < window_size {
        let padding_needed = window_size - processed_audio.len();
        padded_audio.extend(vec![0.0f32; padding_needed]);
    }

    let mut indata = padded_audio;
    r2c.process(&mut indata, &mut y)?;

    let mut processed_audio = y
        .iter()
        .map(|&x| {
            let magnitude_y = x.abs().powf(2.0);
            let div = 1.0 - (d / magnitude_y);
            let gain = if div > 0.0 { f32::sqrt(div) } else { 0.0f32 };
            x * gain
        })
        .collect::<Vec<Complex32>>();

    let c2r = real_planner.plan_fft_inverse(window_size);
    let mut outdata = c2r.make_output_vec();
    c2r.process(&mut processed_audio, &mut outdata)?;

    Ok(outdata)
}

pub fn average_noise_spectrum(audio: &[f32]) -> f32 {
    let mut total_sum = 0.0f32;

    for sample in audio {
        let magnitude = sample.abs();
        total_sum += magnitude.powf(2.0);
    }

    total_sum / audio.len() as f32
}

pub fn audio_to_mono(audio: &[f32], channels: u16) -> Vec<f32> {
    let mut mono_samples = Vec::with_capacity(audio.len() / channels as usize);

    for chunk in audio.chunks(channels as usize) {
        let sum: f32 = chunk.iter().sum();
        let mono_sample = sum / channels as f32;
        mono_samples.push(mono_sample);
    }

    mono_samples
}
