// Speaker database for voice registration and persistent storage
// Stores voice embeddings for known speakers

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use log::{info, debug};
use chrono::{DateTime, Utc};

/// A registered speaker with their voice embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredSpeaker {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Voice embedding vector
    #[serde(skip_serializing)]  // Don't expose raw embedding to frontend
    pub embedding: Vec<f32>,
    /// When the speaker was registered
    pub created_at: DateTime<Utc>,
    /// Number of audio samples used to create this profile
    pub sample_count: u32,
    /// Last recording where this speaker was detected
    pub last_seen: Option<DateTime<Utc>>,
}

/// Database for storing registered speakers
pub struct SpeakerDatabase {
    /// In-memory speaker storage (will be persisted to SQLite in Phase 2)
    speakers: HashMap<String, RegisteredSpeaker>,
    /// Counter for generating unique IDs
    next_id: u32,
}

impl SpeakerDatabase {
    /// Create a new speaker database
    pub fn new() -> Result<Self> {
        info!("Initializing speaker database");

        // TODO: In Phase 2, load existing speakers from SQLite
        Ok(Self {
            speakers: HashMap::new(),
            next_id: 1,
        })
    }

    /// Register a new speaker with their voice embedding
    pub fn register_speaker(&mut self, name: &str, embedding: &[f32]) -> Result<String> {
        let id = format!("spk_{:04}", self.next_id);
        self.next_id += 1;

        let speaker = RegisteredSpeaker {
            id: id.clone(),
            name: name.to_string(),
            embedding: embedding.to_vec(),
            created_at: Utc::now(),
            sample_count: 1,
            last_seen: None,
        };

        self.speakers.insert(id.clone(), speaker);
        info!("Registered speaker '{}' with ID {}", name, id);

        // TODO: In Phase 2, persist to SQLite

        Ok(id)
    }

    /// Update an existing speaker's embedding (add more samples)
    pub fn update_speaker_embedding(&mut self, speaker_id: &str, new_embedding: &[f32]) -> Result<()> {
        let speaker = self.speakers.get_mut(speaker_id)
            .ok_or_else(|| anyhow!("Speaker not found: {}", speaker_id))?;

        // Average the embeddings for better accuracy
        // This creates a more robust voice profile from multiple samples
        let sample_count = speaker.sample_count as f32;
        let new_count = sample_count + 1.0;

        speaker.embedding = speaker.embedding
            .iter()
            .zip(new_embedding.iter())
            .map(|(old, new)| (old * sample_count + new) / new_count)
            .collect();

        speaker.sample_count += 1;
        speaker.last_seen = Some(Utc::now());

        debug!("Updated speaker '{}' embedding (now {} samples)", speaker.name, speaker.sample_count);

        // TODO: In Phase 2, update in SQLite

        Ok(())
    }

    /// Remove a registered speaker
    pub fn unregister_speaker(&mut self, speaker_id: &str) -> Result<()> {
        if self.speakers.remove(speaker_id).is_some() {
            info!("Unregistered speaker: {}", speaker_id);
            // TODO: In Phase 2, delete from SQLite
            Ok(())
        } else {
            Err(anyhow!("Speaker not found: {}", speaker_id))
        }
    }

    /// Find a matching registered speaker for an embedding
    /// Returns (speaker_id, speaker_name, similarity) if found above threshold
    pub fn find_matching_speaker(
        &self,
        embedding: &[f32],
        threshold: f32,
    ) -> Result<Option<(String, String, f32)>> {
        let mut best_match: Option<(String, String, f32)> = None;

        for (id, speaker) in &self.speakers {
            let similarity = cosine_similarity(embedding, &speaker.embedding);

            if similarity >= threshold {
                if best_match.as_ref().map_or(true, |(_, _, s)| similarity > *s) {
                    best_match = Some((id.clone(), speaker.name.clone(), similarity));
                }
            }
        }

        if let Some((ref id, ref name, sim)) = best_match {
            debug!("Found matching speaker: {} ({}) with similarity {:.2}", name, id, sim);
        }

        Ok(best_match)
    }

    /// Get all registered speakers (without embeddings)
    pub fn get_all_speakers(&self) -> Result<Vec<RegisteredSpeaker>> {
        Ok(self.speakers.values().cloned().collect())
    }

    /// Get a specific speaker by ID
    pub fn get_speaker(&self, speaker_id: &str) -> Option<&RegisteredSpeaker> {
        self.speakers.get(speaker_id)
    }

    /// Rename a registered speaker
    pub fn rename_speaker(&mut self, speaker_id: &str, new_name: &str) -> Result<()> {
        let speaker = self.speakers.get_mut(speaker_id)
            .ok_or_else(|| anyhow!("Speaker not found: {}", speaker_id))?;

        speaker.name = new_name.to_string();
        info!("Renamed speaker {} to '{}'", speaker_id, new_name);

        // TODO: In Phase 2, update in SQLite

        Ok(())
    }

    /// Mark a speaker as seen in a recording
    pub fn mark_speaker_seen(&mut self, speaker_id: &str) -> Result<()> {
        if let Some(speaker) = self.speakers.get_mut(speaker_id) {
            speaker.last_seen = Some(Utc::now());
            // TODO: In Phase 2, update in SQLite
        }
        Ok(())
    }

    /// Get the count of registered speakers
    pub fn speaker_count(&self) -> usize {
        self.speakers.len()
    }
}

/// Calculate cosine similarity between two embeddings
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        // Same vector should have similarity 1.0
        let a = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &a) - 1.0).abs() < 0.001);

        // Orthogonal vectors should have similarity 0.0
        let b = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &b).abs() < 0.001);

        // Opposite vectors should have similarity -1.0
        let c = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &c) + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_register_speaker() {
        let mut db = SpeakerDatabase::new().unwrap();
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];

        let id = db.register_speaker("Test User", &embedding).unwrap();
        assert!(id.starts_with("spk_"));

        let speakers = db.get_all_speakers().unwrap();
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].name, "Test User");
    }

    #[test]
    fn test_find_matching_speaker() {
        let mut db = SpeakerDatabase::new().unwrap();
        let embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        db.register_speaker("Test User", &embedding).unwrap();

        // Same embedding should match
        let result = db.find_matching_speaker(&embedding, 0.9).unwrap();
        assert!(result.is_some());
        let (_, name, sim) = result.unwrap();
        assert_eq!(name, "Test User");
        assert!(sim > 0.99);

        // Different embedding should not match
        let different = vec![0.5, 0.4, 0.3, 0.2, 0.1];
        let result = db.find_matching_speaker(&different, 0.9).unwrap();
        assert!(result.is_none());
    }
}
