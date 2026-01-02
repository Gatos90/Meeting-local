// Speaker diarization module
// Provides speaker identification for both live recording and re-transcription
//
// Supports two backends:
// - pyannote-rs: segmentation + speaker embeddings (default)
// - Sortformer v2: streaming 4-speaker diarization (NVIDIA)

pub mod engine;
pub mod speaker_db;
pub mod model_manager;
pub mod sortformer;
pub mod sortformer_provider;

// Re-export pyannote-rs based engine (default)
pub use engine::{
    DiarizationEngine, SpeakerSegment, DiarizationConfig,
    init_diarization_engine, get_diarization_engine,
    DIARIZATION_ENGINE,
};

// Re-export speaker database
pub use speaker_db::{
    RegisteredSpeaker, SpeakerDatabase,
};

// Re-export model manager for pyannote models
pub use model_manager::{
    are_models_available, get_model_paths, get_models_info,
    ensure_models_downloaded, DiarizationModelInfo,
    SEGMENTATION_MODEL_NAME, EMBEDDING_MODEL_NAME,
};

// Re-export Sortformer provider
pub use sortformer_provider::{
    SortformerEngine, SORTFORMER_ENGINE, SORTFORMER_MODEL_NAME, SORTFORMER_MODEL_URL,
    init_sortformer_engine,
};
