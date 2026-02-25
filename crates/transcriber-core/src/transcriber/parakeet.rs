use anyhow::{Context, Result, bail};
use hf_hub::{Repo, RepoType, api::sync::Api};
use std::path::{Path, PathBuf};
use transcribe_rs::TranscriptionEngine;
use transcribe_rs::engines::parakeet::{ParakeetEngine, ParakeetModelParams};

use crate::transcriber::config::{DEFAULT_PARAKEET_MODEL, DEFAULT_PARAKEET_ONNX_REPO};

pub(crate) struct ParakeetTranscriber {
    engine: ParakeetEngine,
}

impl ParakeetTranscriber {
    pub(crate) fn new(model_id_or_path: &str) -> Result<Self> {
        let model_dir = resolve_parakeet_model_dir(model_id_or_path)?;
        let mut engine = ParakeetEngine::new();

        if let Err(int8_error) =
            engine.load_model_with_params(&model_dir, ParakeetModelParams::int8())
        {
            engine
                .load_model(&model_dir)
                .map_err(|fp32_error| anyhow::anyhow!("{fp32_error}"))
                .with_context(|| {
                    format!(
                        "failed to load parakeet model from '{}' after int8 attempt: {}",
                        model_dir.display(),
                        int8_error
                    )
                })?;
        }

        Ok(Self { engine })
    }

    pub(crate) fn is_available() -> bool {
        true
    }

    pub(crate) fn transcribe_path(&mut self, input_path: &Path) -> Result<String> {
        let audio_data = crate::audio::load_audio(input_path).context("failed to decode audio")?;
        let result = self
            .engine
            .transcribe_samples(audio_data, None)
            .map_err(|error| anyhow::anyhow!("{error}"))
            .context("parakeet inference failed")?;

        let text = result.text.trim().to_string();
        if text.is_empty() {
            bail!("Parakeet inference returned empty transcript");
        }

        Ok(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParakeetModelSource {
    LocalDir(PathBuf),
    HuggingFaceRepo(String),
}

fn resolve_parakeet_model_source(model_id_or_path: &str) -> ParakeetModelSource {
    let local_path = Path::new(model_id_or_path);
    if local_path.is_dir() {
        return ParakeetModelSource::LocalDir(local_path.to_path_buf());
    }

    ParakeetModelSource::HuggingFaceRepo(model_id_or_path.to_string())
}

fn resolve_parakeet_model_dir(model_id_or_path: &str) -> Result<PathBuf> {
    match resolve_parakeet_model_source(model_id_or_path) {
        ParakeetModelSource::LocalDir(path) => Ok(path),
        ParakeetModelSource::HuggingFaceRepo(repo_id) => download_parakeet_model_dir(&repo_id),
    }
}

fn download_parakeet_model_dir(repo_id: &str) -> Result<PathBuf> {
    let resolved_repo_id = resolve_parakeet_repo_id(repo_id);
    let api = Api::new().context("failed to initialize Hugging Face API")?;
    let repo = api.repo(Repo::with_revision(
        resolved_repo_id.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));

    // Prefer int8 ONNX files for faster load and lower disk use.
    let int8_files = [
        "encoder-model.int8.onnx",
        "decoder_joint-model.int8.onnx",
        "nemo128.onnx",
        "vocab.txt",
    ];

    // Fallback to FP32 ONNX files for compatibility.
    let fp32_files = [
        "encoder-model.onnx",
        "encoder-model.onnx.data",
        "decoder_joint-model.onnx",
        "nemo128.onnx",
        "vocab.txt",
    ];

    match download_parakeet_files(&repo, resolved_repo_id, &int8_files) {
        Ok(model_dir) => Ok(model_dir),
        Err(_) => {
            download_parakeet_files(&repo, resolved_repo_id, &fp32_files).with_context(|| {
                format!(
                    "failed to download both int8 and fp32 model files from '{}'",
                    resolved_repo_id
                )
            })
        }
    }
}

fn download_parakeet_files(
    repo: &hf_hub::api::sync::ApiRepo,
    resolved_repo_id: &str,
    required_files: &[&str],
) -> Result<PathBuf> {
    let mut model_dir: Option<PathBuf> = None;

    for filename in required_files {
        let downloaded = repo
            .get(filename)
            .with_context(|| format!("failed to fetch '{filename}' from '{resolved_repo_id}'"))?;

        let parent = downloaded.parent().ok_or_else(|| {
            anyhow::anyhow!(
                "could not resolve parent directory for '{}'",
                downloaded.display()
            )
        })?;

        if let Some(existing_dir) = &model_dir {
            if existing_dir != parent {
                bail!(
                    "downloaded model files from '{resolved_repo_id}' did not resolve to one directory"
                );
            }
        } else {
            model_dir = Some(parent.to_path_buf());
        }
    }

    model_dir.ok_or_else(|| {
        anyhow::anyhow!("failed to resolve model directory for '{resolved_repo_id}'")
    })
}

fn resolve_parakeet_repo_id(repo_id: &str) -> &str {
    if repo_id == DEFAULT_PARAKEET_MODEL {
        return DEFAULT_PARAKEET_ONNX_REPO;
    }

    repo_id
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_PARAKEET_MODEL, DEFAULT_PARAKEET_ONNX_REPO, ParakeetModelSource,
        resolve_parakeet_model_source, resolve_parakeet_repo_id,
    };
    use tempfile::tempdir;

    #[test]
    fn model_resolution_prefers_existing_local_directory() {
        let tmp_dir = tempdir().expect("failed to create temp directory");
        let input = tmp_dir.path().display().to_string();
        assert_eq!(
            resolve_parakeet_model_source(&input),
            ParakeetModelSource::LocalDir(tmp_dir.path().to_path_buf())
        );
    }

    #[test]
    fn model_resolution_falls_back_to_repo_id() {
        let repo_id = "istupakov/parakeet-tdt-0.6b-v3-onnx";
        assert_eq!(
            resolve_parakeet_model_source(repo_id),
            ParakeetModelSource::HuggingFaceRepo(repo_id.to_string())
        );
    }

    #[test]
    fn default_repo_resolves_to_onnx_repo() {
        assert_eq!(
            resolve_parakeet_repo_id(DEFAULT_PARAKEET_MODEL),
            DEFAULT_PARAKEET_ONNX_REPO
        );
    }
}
