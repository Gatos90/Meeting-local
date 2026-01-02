// audio/transcription/diarization_integration.rs
//
// Live diarization support for transcription worker.

use crate::diarization::{DIARIZATION_ENGINE, SpeakerSegment};
use log::debug;

use super::globals::is_live_diarization_enabled;

/// Run diarization on audio samples and return speaker info for the given time range
#[allow(dead_code)]
pub async fn get_speaker_for_segment(
    samples: &[f32],
    sample_rate: u32,
    start_time: f64,
    end_time: f64,
) -> Option<(String, String, bool)> {
    if !is_live_diarization_enabled() {
        return None;
    }

    // Try to get diarization engine
    let mut guard = DIARIZATION_ENGINE.write().await;
    let engine = match guard.as_mut() {
        Some(e) => e,
        None => return None,
    };

    // Run diarization on this segment
    match engine.diarize(samples, sample_rate) {
        Ok(segments) => {
            // Find the best matching speaker segment by time overlap
            let best_segment = find_best_speaker_segment(&segments, start_time, end_time);
            best_segment.map(|seg| (
                seg.speaker_id.clone(),
                seg.speaker_label.clone(),
                seg.is_registered,
            ))
        }
        Err(e) => {
            debug!("Diarization failed for segment: {}", e);
            None
        }
    }
}

/// Find the speaker segment with the most overlap with the given time range
pub fn find_best_speaker_segment(
    segments: &[SpeakerSegment],
    start_time: f64,
    end_time: f64,
) -> Option<&SpeakerSegment> {
    if segments.is_empty() {
        return None;
    }

    // Find segment with maximum overlap
    segments.iter()
        .filter_map(|seg| {
            let overlap_start = start_time.max(seg.start_time);
            let overlap_end = end_time.min(seg.end_time);
            let overlap = (overlap_end - overlap_start).max(0.0);
            if overlap > 0.0 {
                Some((seg, overlap))
            } else {
                None
            }
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(seg, _)| seg)
}
