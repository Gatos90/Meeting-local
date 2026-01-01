//! Sortformer Types

/// Speaker segment output
#[derive(Debug, Clone)]
pub struct SpeakerSegment {
    pub start: f32,
    pub end: f32,
    pub speaker_id: usize,
}
