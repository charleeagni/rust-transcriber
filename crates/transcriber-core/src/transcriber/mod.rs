// Declare the sub-modules that live in sibling files.
// Each one holds the actual transcription logic for a specific AI engine.
pub mod config; // Settings & model selection logic
pub mod moonshine; // Moonshine speech-to-text engine
pub mod parakeet; // Parakeet speech-to-text engine
pub mod whisper; // Whisper speech-to-text engine

// Re-export commonly used types so callers don't have to go through `config::` themselves.
pub use config::{DEFAULT_PARAKEET_MODEL, RuntimeSelection, TranscriptionConfig};

use anyhow::{Result, bail};
use std::path::Path;

// Shorter alias so we can write `RS::Whisper` instead of `config::RuntimeSelection::Whisper`.
use config::RuntimeSelection as RS;

/// A simple label that tells you *which* engine is currently active.
/// This is the public-facing version — callers can read it but can't construct
/// the real inner transcriber themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBackend {
    Whisper,
    Parakeet,
    Moonshine,
}

/// The main transcriber object that the rest of the app talks to.
/// It wraps whichever engine was chosen so callers don't need to care about details.
pub struct Transcriber {
    /// The actual engine doing the work — hidden behind an enum so we only expose one type.
    backend: RuntimeBackendImpl,
}

impl Transcriber {
    /// Create a new `Transcriber` from a config.
    /// This picks the right engine and loads the model — it may download files from the internet.
    pub fn new(config: &TranscriptionConfig) -> Result<Self> {
        // Ask the config to decide which engine to use (Whisper / Parakeet / Moonshine).
        let runtime = config.resolve_runtime();
        // Get the model name/path for that engine (or use the default if none was specified).
        let model_id = config.resolve_model_id(runtime);

        // Build the concrete engine based on the chosen runtime.
        let backend = match runtime {
            RS::Whisper => {
                // Load Whisper — downloads model weights from Hugging Face if needed.
                RuntimeBackendImpl::Whisper(whisper::WhisperTranscriber::new(&model_id)?)
            }
            RS::Parakeet => {
                // Load Parakeet — downloads ONNX model files from Hugging Face if needed.
                RuntimeBackendImpl::Parakeet(parakeet::ParakeetTranscriber::new(&model_id)?)
            }
            RS::Moonshine => {
                // Load Moonshine — downloads model files from Hugging Face if needed.
                RuntimeBackendImpl::Moonshine(moonshine::MoonshineTranscriber::new(&model_id)?)
            }
            // `Auto` should have already been resolved to a concrete variant above; this branch
            // should never be reached, so we panic loudly if it somehow is.
            RS::Auto => unreachable!(),
        };

        Ok(Self { backend })
    }

    /// Returns which engine is currently loaded (Whisper, Parakeet, or Moonshine).
    pub fn backend(&self) -> RuntimeBackend {
        match self.backend {
            RuntimeBackendImpl::Whisper(_) => RuntimeBackend::Whisper,
            RuntimeBackendImpl::Parakeet(_) => RuntimeBackend::Parakeet,
            RuntimeBackendImpl::Moonshine(_) => RuntimeBackend::Moonshine,
        }
    }

    /// Transcribe a chunk of raw audio samples (PCM = numbers representing sound waves).
    /// Only Whisper supports this — it's used for live/streaming audio.
    /// Parakeet and Moonshine need a file on disk instead.
    pub fn transcribe_pcm(&mut self, pcm_data: &[f32]) -> Result<String> {
        match &mut self.backend {
            RuntimeBackendImpl::Whisper(transcriber) => transcriber.transcribe_pcm(pcm_data),
            // These two engines cannot work with raw audio directly — return a helpful error.
            RuntimeBackendImpl::Parakeet(_) => {
                bail!("Parakeet runtime requires a file path input")
            }
            RuntimeBackendImpl::Moonshine(_) => {
                bail!("Moonshine runtime requires a file path input")
            }
        }
    }

    /// Transcribe an audio file given its path on disk.
    /// Parakeet and Moonshine use this path. Whisper works on raw PCM instead.
    pub fn transcribe_path(&mut self, input_path: &Path) -> Result<String> {
        match &mut self.backend {
            // Whisper doesn't support file-path input — return a helpful error.
            RuntimeBackendImpl::Whisper(_) => bail!("Whisper runtime requires decoded PCM input"),
            RuntimeBackendImpl::Parakeet(transcriber) => transcriber.transcribe_path(input_path),
            RuntimeBackendImpl::Moonshine(transcriber) => transcriber.transcribe_path(input_path),
        }
    }
}

/// The private enum that holds the actual engine.
/// Using an enum lets us store any of the three engines in the same `Transcriber` struct
/// without needing dynamic dispatch (no `Box<dyn Trait>` overhead).
enum RuntimeBackendImpl {
    Whisper(whisper::WhisperTranscriber),
    Parakeet(parakeet::ParakeetTranscriber),
    Moonshine(moonshine::MoonshineTranscriber),
}
