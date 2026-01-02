//! Sortformer Post-processing - Binarization, median filtering, segment merging

use ndarray::Array2;

use super::config::{NUM_SPEAKERS, FRAME_DURATION, DiarizationConfig};
use super::types::SpeakerSegment;

/// Apply median filter to predictions
pub fn median_filter(preds: &Array2<f32>, config: &DiarizationConfig) -> Array2<f32> {
    let window = config.median_window;
    let half = window / 2;
    let mut filtered = preds.clone();

    for spk in 0..NUM_SPEAKERS {
        for t in 0..preds.shape()[0] {
            let start = t.saturating_sub(half);
            let end = (t + half + 1).min(preds.shape()[0]);
            let mut values: Vec<f32> = (start..end).map(|i| preds[[i, spk]]).collect();
            values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            filtered[[t, spk]] = values[values.len() / 2];
        }
    }

    filtered
}

/// Binarize predictions into speaker segments
pub fn binarize(preds: &Array2<f32>, config: &DiarizationConfig) -> Vec<SpeakerSegment> {
    let mut segments = Vec::new();
    let num_frames = preds.shape()[0];

    for spk in 0..NUM_SPEAKERS {
        let mut in_seg = false;
        let mut seg_start = 0;
        let mut temp_segments = Vec::new();

        for t in 0..num_frames {
            let p = preds[[t, spk]];

            if p >= config.onset && !in_seg {
                in_seg = true;
                seg_start = t;
            } else if p < config.offset && in_seg {
                in_seg = false;
                let start_t = (seg_start as f32 * FRAME_DURATION - config.pad_onset).max(0.0);
                let end_t = t as f32 * FRAME_DURATION + config.pad_offset;

                if end_t - start_t >= config.min_duration_on {
                    temp_segments.push(SpeakerSegment {
                        start: start_t,
                        end: end_t,
                        speaker_id: spk,
                    });
                }
            }
        }

        if in_seg {
            let start_t = (seg_start as f32 * FRAME_DURATION - config.pad_onset).max(0.0);
            let end_t = num_frames as f32 * FRAME_DURATION + config.pad_offset;

            if end_t - start_t >= config.min_duration_on {
                temp_segments.push(SpeakerSegment {
                    start: start_t,
                    end: end_t,
                    speaker_id: spk,
                });
            }
        }

        // Merge segments with small gaps
        if temp_segments.len() > 1 {
            let mut filtered = vec![temp_segments[0].clone()];
            for seg in temp_segments.into_iter().skip(1) {
                let last = filtered.last_mut().unwrap();
                let gap = seg.start - last.end;
                if gap < config.min_duration_off {
                    last.end = seg.end;
                } else {
                    filtered.push(seg);
                }
            }
            segments.extend(filtered);
        } else {
            segments.extend(temp_segments);
        }
    }

    segments.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    segments
}
