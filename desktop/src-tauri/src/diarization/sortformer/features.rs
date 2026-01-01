//! Sortformer Feature Extraction - Mel spectrogram, STFT, preemphasis

use ndarray::{Array2, Array3};
use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;

use super::config::{
    N_FFT, WIN_LENGTH, HOP_LENGTH, N_MELS, PREEMPH, LOG_ZERO_GUARD,
    SAMPLE_RATE, FMIN, FMAX,
};

/// Apply pre-emphasis filter to audio
pub fn apply_preemphasis(audio: &[f32]) -> Vec<f32> {
    let mut result = Vec::with_capacity(audio.len());
    result.push(audio[0]);
    for i in 1..audio.len() {
        result.push(audio[i] - PREEMPH * audio[i - 1]);
    }
    result
}

/// Generate Hann window
pub fn hann_window(window_length: usize) -> Vec<f32> {
    (0..window_length)
        .map(|i| 0.5 - 0.5 * ((2.0 * PI * i as f32) / window_length as f32).cos())
        .collect()
}

/// Short-time Fourier transform
pub fn stft(audio: &[f32]) -> Array2<f32> {
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(N_FFT);

    let hann = hann_window(WIN_LENGTH);
    let win_offset = (N_FFT - WIN_LENGTH) / 2;
    let mut fft_window = vec![0.0f32; N_FFT];
    for i in 0..WIN_LENGTH {
        fft_window[win_offset + i] = hann[i];
    }

    let pad_amount = N_FFT / 2;
    let mut padded_audio = vec![0.0; pad_amount];
    padded_audio.extend_from_slice(audio);
    padded_audio.extend(vec![0.0; pad_amount]);

    let num_frames = (padded_audio.len() - N_FFT) / HOP_LENGTH + 1;
    let freq_bins = N_FFT / 2 + 1;
    let mut spectrogram = Array2::<f32>::zeros((freq_bins, num_frames));

    for frame_idx in 0..num_frames {
        let start = frame_idx * HOP_LENGTH;
        let mut frame: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); N_FFT];

        for i in 0..N_FFT {
            if start + i < padded_audio.len() {
                frame[i] = Complex::new(padded_audio[start + i] * fft_window[i], 0.0);
            }
        }

        fft.process(&mut frame);
        for k in 0..freq_bins {
            let magnitude = frame[k].norm();
            spectrogram[[k, frame_idx]] = magnitude * magnitude;
        }
    }

    spectrogram
}

/// Convert Hz to Mel scale (Slaney formula)
pub fn hz_to_mel_slaney(hz: f64) -> f64 {
    let f_min = 0.0;
    let f_sp = 200.0 / 3.0;
    let min_log_hz = 1000.0;
    let min_log_mel = (min_log_hz - f_min) / f_sp;
    let logstep = (6.4f64).ln() / 27.0;

    if hz >= min_log_hz {
        min_log_mel + (hz / min_log_hz).ln() / logstep
    } else {
        (hz - f_min) / f_sp
    }
}

/// Convert Mel to Hz scale (Slaney formula)
pub fn mel_to_hz_slaney(mel: f64) -> f64 {
    let f_min = 0.0;
    let f_sp = 200.0 / 3.0;
    let min_log_hz = 1000.0;
    let min_log_mel = (min_log_hz - f_min) / f_sp;
    let logstep = (6.4f64).ln() / 27.0;

    if mel >= min_log_mel {
        min_log_hz * (logstep * (mel - min_log_mel)).exp()
    } else {
        f_min + f_sp * mel
    }
}

/// Create mel filterbank matrix
pub fn create_mel_filterbank() -> Array2<f32> {
    let freq_bins = N_FFT / 2 + 1;
    let mut filterbank = Array2::<f32>::zeros((N_MELS, freq_bins));

    let fftfreqs: Vec<f64> = (0..freq_bins)
        .map(|k| k as f64 * SAMPLE_RATE as f64 / N_FFT as f64)
        .collect();

    let fmin_mel = hz_to_mel_slaney(FMIN as f64);
    let fmax_mel = hz_to_mel_slaney(FMAX as f64);
    let mel_f: Vec<f64> = (0..=N_MELS + 1)
        .map(|i| {
            let mel = fmin_mel + (fmax_mel - fmin_mel) * i as f64 / (N_MELS + 1) as f64;
            mel_to_hz_slaney(mel)
        })
        .collect();

    let fdiff: Vec<f64> = mel_f.windows(2).map(|w| w[1] - w[0]).collect();

    for i in 0..N_MELS {
        for k in 0..freq_bins {
            let lower = (fftfreqs[k] - mel_f[i]) / fdiff[i];
            let upper = (mel_f[i + 2] - fftfreqs[k]) / fdiff[i + 1];
            filterbank[[i, k]] = 0.0f64.max(lower.min(upper)) as f32;
        }
    }

    for i in 0..N_MELS {
        let enorm = 2.0 / (mel_f[i + 2] - mel_f[i]);
        for k in 0..freq_bins {
            filterbank[[i, k]] *= enorm as f32;
        }
    }

    filterbank
}

/// Extract mel features from audio
pub fn extract_mel_features(audio: &[f32], mel_basis: &Array2<f32>) -> Array3<f32> {
    let preemphasized = apply_preemphasis(audio);
    let spectrogram = stft(&preemphasized);
    let mel_spec = mel_basis.dot(&spectrogram);
    let log_mel_spec = mel_spec.mapv(|x| (x + LOG_ZERO_GUARD).ln());

    let num_frames = log_mel_spec.shape()[1];
    let mut features = Array3::<f32>::zeros((1, num_frames, N_MELS));

    for t in 0..num_frames {
        for m in 0..N_MELS {
            features[[0, t, m]] = log_mel_spec[[m, t]];
        }
    }

    features
}
