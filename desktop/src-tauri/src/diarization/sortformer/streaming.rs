//! Sortformer Streaming - Cache management, scoring, and streaming update logic

use anyhow::{anyhow, Result};
use ndarray::{s, Array1, Array2, Array3, Axis};

use super::config::{
    EMB_DIM, NUM_SPEAKERS, FIFO_LEN, SPKCACHE_LEN, SPKCACHE_UPDATE_PERIOD,
    SUBSAMPLING, SPKCACHE_SIL_FRAMES_PER_SPK, PRED_SCORE_THRESHOLD,
    STRONG_BOOST_RATE, WEAK_BOOST_RATE, MIN_POS_SCORES_RATE, SIL_THRESHOLD, MAX_INDEX,
};

/// Streaming state for Sortformer
pub struct StreamingState {
    pub spkcache: Array3<f32>,
    pub spkcache_preds: Option<Array3<f32>>,
    pub fifo: Array3<f32>,
    pub fifo_preds: Array3<f32>,
    pub mean_sil_emb: Array2<f32>,
    pub n_sil_frames: usize,
}

impl StreamingState {
    pub fn new() -> Self {
        Self {
            spkcache: Array3::zeros((1, 0, EMB_DIM)),
            spkcache_preds: None,
            fifo: Array3::zeros((1, 0, EMB_DIM)),
            fifo_preds: Array3::zeros((1, 0, NUM_SPEAKERS)),
            mean_sil_emb: Array2::zeros((1, EMB_DIM)),
            n_sil_frames: 0,
        }
    }

    pub fn reset(&mut self) {
        self.spkcache = Array3::zeros((1, 0, EMB_DIM));
        self.spkcache_preds = None;
        self.fifo = Array3::zeros((1, 0, EMB_DIM));
        self.fifo_preds = Array3::zeros((1, 0, NUM_SPEAKERS));
        self.mean_sil_emb = Array2::zeros((1, EMB_DIM));
        self.n_sil_frames = 0;
    }
}

/// Process a streaming update with the ONNX session
pub fn streaming_update(
    state: &mut StreamingState,
    session: &mut ort::session::Session,
    chunk_feat: &Array3<f32>,
    current_len: usize,
) -> Result<Array2<f32>> {
    let spkcache_len = state.spkcache.shape()[1];
    let fifo_len = state.fifo.shape()[1];

    let chunk_lengths = Array1::from_vec(vec![current_len as i64]);
    let spkcache_lengths = Array1::from_vec(vec![spkcache_len as i64]);
    let fifo_lengths = Array1::from_vec(vec![fifo_len as i64]);

    let fifo_input = if fifo_len > 0 {
        state.fifo.clone()
    } else {
        Array3::zeros((1, 0, EMB_DIM))
    };

    let spkcache_input = if spkcache_len > 0 {
        state.spkcache.clone()
    } else {
        Array3::zeros((1, 0, EMB_DIM))
    };

    let chunk_value = ort::value::Value::from_array(chunk_feat.clone())?;
    let chunk_lengths_value = ort::value::Value::from_array(chunk_lengths)?;
    let spkcache_value = ort::value::Value::from_array(spkcache_input)?;
    let spkcache_lengths_value = ort::value::Value::from_array(spkcache_lengths)?;
    let fifo_value = ort::value::Value::from_array(fifo_input)?;
    let fifo_lengths_value = ort::value::Value::from_array(fifo_lengths)?;

    let (preds, new_embs, chunk_len) = {
        let outputs = session.run(ort::inputs!(
            "chunk" => chunk_value,
            "chunk_lengths" => chunk_lengths_value,
            "spkcache" => spkcache_value,
            "spkcache_lengths" => spkcache_lengths_value,
            "fifo" => fifo_value,
            "fifo_lengths" => fifo_lengths_value
        ))?;

        let (preds_shape, preds_data) = outputs["spkcache_fifo_chunk_preds"]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("Failed to extract preds: {e}"))?;
        let (embs_shape, embs_data) = outputs["chunk_pre_encode_embs"]
            .try_extract_tensor::<f32>()
            .map_err(|e| anyhow!("Failed to extract embs: {e}"))?;

        let preds_dims = preds_shape.as_ref();
        let embs_dims = embs_shape.as_ref();

        let preds = Array3::from_shape_vec(
            (preds_dims[0] as usize, preds_dims[1] as usize, preds_dims[2] as usize),
            preds_data.to_vec(),
        )?;

        let new_embs = Array3::from_shape_vec(
            (embs_dims[0] as usize, embs_dims[1] as usize, embs_dims[2] as usize),
            embs_data.to_vec(),
        )?;

        let valid_frames = (current_len + SUBSAMPLING - 1) / SUBSAMPLING;
        (preds, new_embs, valid_frames)
    };

    let fifo_preds = if fifo_len > 0 {
        preds.slice(s![0, spkcache_len..spkcache_len + fifo_len, ..]).to_owned()
    } else {
        Array2::zeros((0, NUM_SPEAKERS))
    };

    let chunk_preds = preds
        .slice(s![0, spkcache_len + fifo_len..spkcache_len + fifo_len + chunk_len, ..])
        .to_owned();
    let chunk_embs = new_embs.slice(s![0, ..chunk_len, ..]).to_owned();

    state.fifo = concat_axis1(&state.fifo, &chunk_embs.insert_axis(Axis(0)));

    if fifo_len > 0 {
        let combined = concat_axis1_2d(&fifo_preds, &chunk_preds);
        state.fifo_preds = combined.insert_axis(Axis(0));
    } else {
        state.fifo_preds = chunk_preds.clone().insert_axis(Axis(0));
    }

    let fifo_len_after = state.fifo.shape()[1];

    if fifo_len_after > FIFO_LEN {
        let mut pop_out_len = SPKCACHE_UPDATE_PERIOD;
        pop_out_len = pop_out_len.max(chunk_len.saturating_sub(FIFO_LEN) + fifo_len);
        pop_out_len = pop_out_len.min(fifo_len_after);

        let pop_out_embs = state.fifo.slice(s![.., ..pop_out_len, ..]).to_owned();
        let pop_out_preds = state.fifo_preds.slice(s![.., ..pop_out_len, ..]).to_owned();

        update_silence_profile(state, &pop_out_embs, &pop_out_preds);

        state.fifo = state.fifo.slice(s![.., pop_out_len.., ..]).to_owned();
        state.fifo_preds = state.fifo_preds.slice(s![.., pop_out_len.., ..]).to_owned();

        state.spkcache = concat_axis1(&state.spkcache, &pop_out_embs);

        if let Some(ref cache_preds) = state.spkcache_preds {
            state.spkcache_preds = Some(concat_axis1(cache_preds, &pop_out_preds));
        }

        if state.spkcache.shape()[1] > SPKCACHE_LEN {
            if state.spkcache_preds.is_none() {
                let initial_cache_preds = preds.slice(s![.., ..spkcache_len, ..]).to_owned();
                let combined = concat_axis1(&initial_cache_preds, &pop_out_preds);
                state.spkcache_preds = Some(combined);
            }
            compress_spkcache(state);
        }
    }

    Ok(chunk_preds)
}

fn update_silence_profile(state: &mut StreamingState, embs: &Array3<f32>, preds: &Array3<f32>) {
    let preds_2d = preds.slice(s![0, .., ..]);

    for t in 0..preds_2d.shape()[0] {
        let sum: f32 = (0..NUM_SPEAKERS).map(|s| preds_2d[[t, s]]).sum();
        if sum < SIL_THRESHOLD {
            let emb = embs.slice(s![0, t, ..]);
            let old_sum: Vec<f32> = state
                .mean_sil_emb
                .slice(s![0, ..])
                .iter()
                .map(|&x| x * state.n_sil_frames as f32)
                .collect();

            state.n_sil_frames += 1;

            for i in 0..EMB_DIM {
                state.mean_sil_emb[[0, i]] = (old_sum[i] + emb[i]) / state.n_sil_frames as f32;
            }
        }
    }
}

fn compress_spkcache(state: &mut StreamingState) {
    let cache_preds = match &state.spkcache_preds {
        Some(p) => p.clone(),
        None => return,
    };

    let n_frames = state.spkcache.shape()[1];
    let spkcache_len_per_spk = SPKCACHE_LEN / NUM_SPEAKERS - SPKCACHE_SIL_FRAMES_PER_SPK;
    let strong_boost_per_spk = (spkcache_len_per_spk as f32 * STRONG_BOOST_RATE) as usize;
    let weak_boost_per_spk = (spkcache_len_per_spk as f32 * WEAK_BOOST_RATE) as usize;
    let min_pos_scores_per_spk = (spkcache_len_per_spk as f32 * MIN_POS_SCORES_RATE) as usize;

    let preds_2d = cache_preds.slice(s![0, .., ..]).to_owned();
    let mut scores = get_log_pred_scores(&preds_2d);
    scores = disable_low_scores(&preds_2d, scores, min_pos_scores_per_spk);
    scores = boost_topk_scores(scores, strong_boost_per_spk, 2.0);
    scores = boost_topk_scores(scores, weak_boost_per_spk, 1.0);

    if SPKCACHE_SIL_FRAMES_PER_SPK > 0 {
        let mut padded = Array2::from_elem(
            (n_frames + SPKCACHE_SIL_FRAMES_PER_SPK, NUM_SPEAKERS),
            f32::NEG_INFINITY,
        );
        padded.slice_mut(s![..n_frames, ..]).assign(&scores);
        for i in n_frames..n_frames + SPKCACHE_SIL_FRAMES_PER_SPK {
            for j in 0..NUM_SPEAKERS {
                padded[[i, j]] = f32::INFINITY;
            }
        }
        scores = padded;
    }

    let (topk_indices, is_disabled) = get_topk_indices(&scores, n_frames);
    let (new_embs, new_preds) = gather_spkcache(state, &topk_indices, &is_disabled);

    state.spkcache = new_embs;
    state.spkcache_preds = Some(new_preds);
}

fn get_log_pred_scores(preds: &Array2<f32>) -> Array2<f32> {
    let mut scores = Array2::zeros(preds.dim());

    for t in 0..preds.shape()[0] {
        let mut log_1_probs_sum = 0.0f32;
        for s in 0..NUM_SPEAKERS {
            let p = preds[[t, s]].max(PRED_SCORE_THRESHOLD);
            let log_1_p = (1.0 - p).max(PRED_SCORE_THRESHOLD).ln();
            log_1_probs_sum += log_1_p;
        }

        for s in 0..NUM_SPEAKERS {
            let p = preds[[t, s]].max(PRED_SCORE_THRESHOLD);
            let log_p = p.ln();
            let log_1_p = (1.0 - p).max(PRED_SCORE_THRESHOLD).ln();
            scores[[t, s]] = log_p - log_1_p + log_1_probs_sum - 0.5f32.ln();
        }
    }

    scores
}

fn disable_low_scores(
    preds: &Array2<f32>,
    mut scores: Array2<f32>,
    min_pos_scores_per_spk: usize,
) -> Array2<f32> {
    let mut pos_count = vec![0usize; NUM_SPEAKERS];
    for t in 0..scores.shape()[0] {
        for s in 0..NUM_SPEAKERS {
            if scores[[t, s]] > 0.0 {
                pos_count[s] += 1;
            }
        }
    }

    for t in 0..preds.shape()[0] {
        for s in 0..NUM_SPEAKERS {
            let is_speech = preds[[t, s]] > 0.5;

            if !is_speech {
                scores[[t, s]] = f32::NEG_INFINITY;
            } else {
                let is_pos = scores[[t, s]] > 0.0;
                if !is_pos && pos_count[s] >= min_pos_scores_per_spk {
                    scores[[t, s]] = f32::NEG_INFINITY;
                }
            }
        }
    }

    scores
}

fn boost_topk_scores(
    mut scores: Array2<f32>,
    n_boost_per_spk: usize,
    scale_factor: f32,
) -> Array2<f32> {
    for s in 0..NUM_SPEAKERS {
        let col: Vec<(usize, f32)> = (0..scores.shape()[0])
            .map(|t| (t, scores[[t, s]]))
            .collect();

        let mut sorted = col.clone();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        for i in 0..n_boost_per_spk.min(sorted.len()) {
            let t = sorted[i].0;
            if scores[[t, s]] != f32::NEG_INFINITY {
                scores[[t, s]] -= scale_factor * 0.5f32.ln();
            }
        }
    }

    scores
}

fn get_topk_indices(scores: &Array2<f32>, n_frames_no_sil: usize) -> (Vec<usize>, Vec<bool>) {
    let n_frames = scores.shape()[0];

    let mut flat_scores: Vec<(usize, f32)> = Vec::with_capacity(n_frames * NUM_SPEAKERS);
    for s in 0..NUM_SPEAKERS {
        for t in 0..n_frames {
            let flat_idx = s * n_frames + t;
            flat_scores.push((flat_idx, scores[[t, s]]));
        }
    }

    flat_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut topk_flat: Vec<usize> = flat_scores
        .iter()
        .take(SPKCACHE_LEN)
        .map(|(idx, score)| {
            if *score == f32::NEG_INFINITY { MAX_INDEX } else { *idx }
        })
        .collect();

    topk_flat.sort();

    let mut is_disabled = vec![false; SPKCACHE_LEN];
    let mut frame_indices = vec![0usize; SPKCACHE_LEN];

    for (i, &flat_idx) in topk_flat.iter().enumerate() {
        if flat_idx == MAX_INDEX {
            is_disabled[i] = true;
            frame_indices[i] = 0;
        } else {
            let frame_idx = flat_idx % n_frames;
            if frame_idx >= n_frames_no_sil {
                is_disabled[i] = true;
                frame_indices[i] = 0;
            } else {
                frame_indices[i] = frame_idx;
            }
        }
    }

    (frame_indices, is_disabled)
}

fn gather_spkcache(state: &StreamingState, indices: &[usize], is_disabled: &[bool]) -> (Array3<f32>, Array3<f32>) {
    let mut new_embs = Array3::zeros((1, SPKCACHE_LEN, EMB_DIM));
    let mut new_preds = Array3::zeros((1, SPKCACHE_LEN, NUM_SPEAKERS));

    let cache_preds = state.spkcache_preds.as_ref().unwrap();

    for (i, (&idx, &disabled)) in indices.iter().zip(is_disabled.iter()).enumerate() {
        if i >= SPKCACHE_LEN {
            break;
        }

        if disabled {
            new_embs.slice_mut(s![0, i, ..]).assign(&state.mean_sil_emb.slice(s![0, ..]));
        } else if idx < state.spkcache.shape()[1] {
            new_embs.slice_mut(s![0, i, ..]).assign(&state.spkcache.slice(s![0, idx, ..]));
            new_preds.slice_mut(s![0, i, ..]).assign(&cache_preds.slice(s![0, idx, ..]));
        }
    }

    (new_embs, new_preds)
}

// Array concatenation helpers
pub fn concat_axis1(a: &Array3<f32>, b: &Array3<f32>) -> Array3<f32> {
    if a.shape()[1] == 0 { return b.clone(); }
    if b.shape()[1] == 0 { return a.clone(); }
    ndarray::concatenate(Axis(1), &[a.view(), b.view()]).unwrap()
}

pub fn concat_axis1_2d(a: &Array2<f32>, b: &Array2<f32>) -> Array2<f32> {
    if a.shape()[0] == 0 { return b.clone(); }
    if b.shape()[0] == 0 { return a.clone(); }
    ndarray::concatenate(Axis(0), &[a.view(), b.view()]).unwrap()
}

pub fn concat_predictions(preds: &[Array2<f32>]) -> Array2<f32> {
    if preds.is_empty() { return Array2::zeros((0, NUM_SPEAKERS)); }
    if preds.len() == 1 { return preds[0].clone(); }
    let views: Vec<_> = preds.iter().map(|p| p.view()).collect();
    ndarray::concatenate(Axis(0), &views).unwrap()
}
