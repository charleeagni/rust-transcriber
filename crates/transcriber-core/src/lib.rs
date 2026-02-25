pub mod audio;
pub mod transcriber;
pub mod watcher;
pub mod writer;

use anyhow::{Context, Result};
use std::path::Path;

pub use transcriber::{RuntimeSelection, TranscriptionConfig};
pub use watcher::{
    WatchCallback, WatchConfig, WatchHandle, spawn_watch_directory, start_m4a_watcher,
    start_m4a_watcher_with_config, watch_directory,
};

pub fn transcribe_file_with_config(
    input_path: &Path,
    output_path: &Path,
    config: &TranscriptionConfig,
) -> Result<String> {
    let mut transcriber_instance =
        transcriber::Transcriber::new(config).context("Error loading model")?;

    let text = match transcriber_instance.backend() {
        transcriber::RuntimeBackend::Whisper => {
            let audio_data = audio::load_audio(input_path).context("Error decoding audio")?;
            transcriber_instance
                .transcribe_pcm(&audio_data)
                .context("Error transcribing")?
        }
        transcriber::RuntimeBackend::Parakeet => transcriber_instance
            .transcribe_path(input_path)
            .context("Error transcribing")?,
    };

    writer::write_transcript(output_path, &text).context("Error writing transcript")?;

    Ok(text)
}

pub fn transcribe_file(input_path: &Path, output_path: &Path, model_id: &str) -> Result<String> {
    let config = TranscriptionConfig::whisper(model_id.to_string());
    transcribe_file_with_config(input_path, output_path, &config)
}
