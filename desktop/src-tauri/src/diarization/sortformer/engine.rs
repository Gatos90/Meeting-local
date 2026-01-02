//! Sortformer Engine - Main diarization struct and entry point

use anyhow::{anyhow, Result};
use ndarray::{s, Array2, Array3};
use ort::session::Session;
use std::path::Path;

use super::config::{
    DiarizationConfig, SAMPLE_RATE, CHUNK_LEN, SUBSAMPLING, N_MELS,
};
use super::types::SpeakerSegment;
use super::features::{create_mel_filterbank, extract_mel_features};
use super::streaming::{StreamingState, streaming_update, concat_predictions};
use super::postprocess::{median_filter, binarize};

/// Streaming Sortformer v2 speaker diarization engine
pub struct Sortformer {
    session: Session,
    config: DiarizationConfig,
    state: StreamingState,
    mel_basis: Array2<f32>,
}

impl Sortformer {
    /// Create a new Sortformer instance from ONNX model path
    pub fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        Self::with_config(model_path, DiarizationConfig::default())
    }

    /// Create with custom config
    pub fn with_config<P: AsRef<Path>>(model_path: P, config: DiarizationConfig) -> Result<Self> {
        let session = Session::builder()?
            .with_intra_threads(4)?
            .commit_from_file(model_path.as_ref())?;

        let mel_basis = create_mel_filterbank();

        let mut instance = Self {
            session,
            config,
            state: StreamingState::new(),
            mel_basis,
        };
        instance.reset_state();
        Ok(instance)
    }

    /// Reset streaming state
    pub fn reset_state(&mut self) {
        self.state.reset();
    }

    /// Main diarization entry point
    pub fn diarize(
        &mut self,
        mut audio: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Vec<SpeakerSegment>> {
        if sample_rate != SAMPLE_RATE as u32 {
            return Err(anyhow!(
                "Expected {} Hz, got {} Hz",
                SAMPLE_RATE,
                sample_rate
            ));
        }

        if channels > 1 {
            audio = audio
                .chunks(channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                .collect();
        }

        self.reset_state();

        let features = extract_mel_features(&audio, &self.mel_basis);
        let total_frames = features.shape()[1];

        let chunk_stride = CHUNK_LEN * SUBSAMPLING;
        let num_chunks = (total_frames + chunk_stride - 1) / chunk_stride;

        let mut all_chunk_preds = Vec::new();

        for chunk_idx in 0..num_chunks {
            let start = chunk_idx * chunk_stride;
            let end = (start + chunk_stride).min(total_frames);
            let current_len = end - start;

            let mut chunk_feat = features.slice(s![.., start..end, ..]).to_owned();

            if current_len < chunk_stride {
                let mut padded = Array3::zeros((1, chunk_stride, N_MELS));
                padded.slice_mut(s![.., ..current_len, ..]).assign(&chunk_feat);
                chunk_feat = padded;
            }

            let chunk_preds = streaming_update(
                &mut self.state,
                &mut self.session,
                &chunk_feat,
                current_len,
            )?;
            all_chunk_preds.push(chunk_preds);
        }

        let full_preds = concat_predictions(&all_chunk_preds);

        let filtered_preds = if self.config.median_window > 1 {
            median_filter(&full_preds, &self.config)
        } else {
            full_preds
        };

        let segments = binarize(&filtered_preds, &self.config);

        Ok(segments)
    }
}
