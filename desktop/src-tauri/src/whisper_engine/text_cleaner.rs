// Whisper Engine - Text Cleaning and Repetition Removal
use std::collections::{HashMap, HashSet};
use crate::{perf_debug};

/// Clean repetitive text patterns and meaningless outputs
pub fn clean_repetitive_text(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Check for obviously meaningless patterns first
    if is_meaningless_output(text) {
        // Performance optimization: reduce meaningless output logging to debug level
        perf_debug!("Detected meaningless output, returning empty: '{}'", text);
        return String::new();
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 3 {
        return text.to_string();
    }

    // Enhanced repetition detection with sliding window
    let cleaned_words = remove_word_repetitions(&words);

    // Remove phrase repetitions with more sophisticated detection
    let cleaned_words = remove_phrase_repetitions(&cleaned_words);

    // Check for overall repetition ratio
    let final_text = cleaned_words.join(" ");
    if calculate_repetition_ratio(&final_text) > 0.7 {
        // Performance optimization: reduce repetition ratio logging to debug level
        perf_debug!("High repetition ratio detected, filtering out: '{}'", final_text);
        return String::new();
    }

    final_text
}

/// Check for obviously meaningless patterns
pub fn is_meaningless_output(text: &str) -> bool {
    let text_lower = text.to_lowercase();

    // Check for common meaningless patterns
    let meaningless_patterns = [
        "thank you for watching",
        "thanks for watching",
        "like and subscribe",
        "music playing",
        "applause",
        "laughter",
        "um um um",
        "uh uh uh",
        "ah ah ah",
    ];

    for pattern in &meaningless_patterns {
        if text_lower.contains(pattern) {
            return true;
        }
    }

    // Check if text is mostly the same character or very short repetitive patterns
    let unique_chars: HashSet<char> = text.chars().collect();
    if unique_chars.len() <= 3 && text.len() > 10 {
        return true;
    }

    false
}

/// Enhanced word repetition removal
pub fn remove_word_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    let mut cleaned_words = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let current_word = words[i];
        let mut repeat_count = 1;

        // Count consecutive repetitions of the same word
        while i + repeat_count < words.len() && words[i + repeat_count] == current_word {
            repeat_count += 1;
        }

        // Be more aggressive: if word is repeated 2+ times, only keep one instance
        if repeat_count >= 2 {
            cleaned_words.push(current_word);
            i += repeat_count;
        } else {
            cleaned_words.push(current_word);
            i += 1;
        }
    }

    cleaned_words
}

/// Enhanced phrase repetition removal with variable length detection
pub fn remove_phrase_repetitions<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
    if words.len() < 4 {
        return words.to_vec();
    }

    let mut final_words = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut phrase_found = false;

        // Check for 2-word to 5-word phrase repetitions
        for phrase_len in 2..=std::cmp::min(5, (words.len() - i) / 2) {
            if i + phrase_len * 2 <= words.len() {
                let phrase1 = &words[i..i + phrase_len];
                let phrase2 = &words[i + phrase_len..i + phrase_len * 2];

                if phrase1 == phrase2 {
                    // Add the phrase once and skip the repetition
                    final_words.extend_from_slice(phrase1);
                    i += phrase_len * 2;
                    phrase_found = true;
                    break;
                }
            }
        }

        if !phrase_found {
            final_words.push(words[i]);
            i += 1;
        }
    }

    final_words
}

/// Calculate repetition ratio in text
pub fn calculate_repetition_ratio(text: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() < 4 {
        return 0.0;
    }

    let mut word_counts = HashMap::new();
    for word in &words {
        *word_counts.entry(word.to_lowercase()).or_insert(0) += 1;
    }

    let total_words = words.len() as f32;
    let repeated_words: usize = word_counts.values().map(|&count| if count > 1 { count - 1 } else { 0 }).sum();

    repeated_words as f32 / total_words
}
