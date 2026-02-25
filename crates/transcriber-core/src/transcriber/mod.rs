pub mod config;
pub mod parakeet;
pub mod whisper;

pub use config::{RuntimeSelection, TranscriptionConfig};

use anyhow::{Result, bail};
use std::path::Path;

use config::RuntimeSelection as RS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeBackend {
    Whisper,
    Parakeet,
}

pub struct Transcriber {
    backend: RuntimeBackendImpl,
}

impl Transcriber {
    pub fn new(config: &TranscriptionConfig) -> Result<Self> {
        let runtime = config.resolve_runtime();
        let model_id = config.resolve_model_id(runtime);

        let backend = match runtime {
            RS::Whisper => {
                RuntimeBackendImpl::Whisper(whisper::WhisperTranscriber::new(&model_id)?)
            }
            RS::Parakeet => {
                RuntimeBackendImpl::Parakeet(parakeet::ParakeetTranscriber::new(&model_id)?)
            }
            RS::Auto => unreachable!(),
        };

        Ok(Self { backend })
    }

    pub fn backend(&self) -> RuntimeBackend {
        match self.backend {
            RuntimeBackendImpl::Whisper(_) => RuntimeBackend::Whisper,
            RuntimeBackendImpl::Parakeet(_) => RuntimeBackend::Parakeet,
        }
    }

    pub fn transcribe_pcm(&mut self, pcm_data: &[f32]) -> Result<String> {
        match &mut self.backend {
            RuntimeBackendImpl::Whisper(transcriber) => transcriber.transcribe_pcm(pcm_data),
            RuntimeBackendImpl::Parakeet(_) => {
                bail!("Parakeet runtime requires a file path input")
            }
        }
    }

    pub fn transcribe_path(&mut self, input_path: &Path) -> Result<String> {
        match &mut self.backend {
            RuntimeBackendImpl::Whisper(_) => bail!("Whisper runtime requires decoded PCM input"),
            RuntimeBackendImpl::Parakeet(transcriber) => transcriber.transcribe_path(input_path),
        }
    }
}

enum RuntimeBackendImpl {
    Whisper(whisper::WhisperTranscriber),
    Parakeet(parakeet::ParakeetTranscriber),
}
