// Audio Processing - Resampling
use anyhow::Result;
use log::debug;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};

/// High-quality audio resampling with adaptive parameters
pub fn resample(input: &[f32], from_sample_rate: u32, to_sample_rate: u32) -> Result<Vec<f32>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    if from_sample_rate == to_sample_rate {
        return Ok(input.to_vec());
    }

    let ratio = to_sample_rate as f64 / from_sample_rate as f64;

    let (sinc_len, interpolation_type, oversampling) = if ratio >= 2.0 {
        debug!("High-quality upsampling: {}Hz → {}Hz (ratio: {:.2}x)",
               from_sample_rate, to_sample_rate, ratio);
        (512, SincInterpolationType::Cubic, 512)
    } else if ratio >= 1.5 {
        debug!("Moderate upsampling: {}Hz → {}Hz (ratio: {:.2}x)",
               from_sample_rate, to_sample_rate, ratio);
        (384, SincInterpolationType::Cubic, 384)
    } else if ratio > 1.0 {
        debug!("Small upsampling: {}Hz → {}Hz (ratio: {:.2}x)",
               from_sample_rate, to_sample_rate, ratio);
        (256, SincInterpolationType::Linear, 256)
    } else if ratio <= 0.5 {
        debug!("Anti-aliased downsampling: {}Hz → {}Hz (ratio: {:.2}x)",
               from_sample_rate, to_sample_rate, ratio);
        (512, SincInterpolationType::Cubic, 512)
    } else {
        debug!("Moderate downsampling: {}Hz → {}Hz (ratio: {:.2}x)",
               from_sample_rate, to_sample_rate, ratio);
        (384, SincInterpolationType::Linear, 384)
    };

    let params = SincInterpolationParameters {
        sinc_len,
        f_cutoff: 0.95,
        interpolation: interpolation_type,
        oversampling_factor: oversampling,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        ratio,
        2.0,
        params,
        input.len(),
        1,
    )?;

    let waves_in = vec![input.to_vec()];
    let waves_out = resampler.process(&waves_in, None)?;

    debug!("Resampling complete: {} samples → {} samples",
           input.len(), waves_out[0].len());

    Ok(waves_out.into_iter().next().unwrap())
}

/// Alias for compatibility with existing code
pub fn resample_audio(input: &[f32], from_sample_rate: u32, to_sample_rate: u32) -> Vec<f32> {
    match resample(input, from_sample_rate, to_sample_rate) {
        Ok(result) => result,
        Err(e) => {
            debug!("Resampling failed: {}, returning original audio", e);
            input.to_vec()
        }
    }
}
