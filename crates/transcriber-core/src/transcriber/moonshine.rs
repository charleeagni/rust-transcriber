use anyhow::{Context, Result, bail};
use hf_hub::{Repo, RepoType, api::sync::Api};
use std::path::{Path, PathBuf};
use transcribe_rs::TranscriptionEngine;
use transcribe_rs::engines::moonshine::{ModelVariant, MoonshineEngine, MoonshineModelParams};

pub(crate) const DEFAULT_MOONSHINE_MODEL: &str = "moonshine-tiny";
const DEFAULT_MOONSHINE_REPO: &str = "UsefulSensors/moonshine";
const TINY_SUBDIR: &str = "onnx/merged/tiny/float";
const BASE_SUBDIR: &str = "onnx/merged/base/float";

const REQUIRED_FILES: &[&str] = &[
    "encoder_model.onnx",
    "decoder_model_merged.onnx",
    "tokenizer.json",
];

pub(crate) struct MoonshineTranscriber {
    engine: MoonshineEngine,
}

impl MoonshineTranscriber {
    pub(crate) fn new(model_id_or_path: &str) -> Result<Self> {
        let (source, variant) = resolve_moonshine_model_source_and_variant(model_id_or_path)?;
        let model_dir = match source {
            MoonshineModelSource::LocalDir(path) => path,
            MoonshineModelSource::HuggingFaceRepo => download_moonshine_model_dir(variant)?,
        };

        let params = match variant {
            ModelVariant::Tiny => MoonshineModelParams::tiny(),
            ModelVariant::Base => MoonshineModelParams::base(),
            _ => bail!("unsupported Moonshine variant: {:?}", variant),
        };

        let mut engine = MoonshineEngine::new();
        engine
            .load_model_with_params(&model_dir, params)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("moonshine model load failed")?;

        Ok(Self { engine })
    }

    pub(crate) fn transcribe_path(&mut self, input_path: &Path) -> Result<String> {
        let audio_data = crate::audio::load_audio(input_path).context("failed to decode audio")?;
        let result = self
            .engine
            .transcribe_samples(audio_data, None)
            .map_err(|error| anyhow::anyhow!("{error}"))
            .context("moonshine inference failed")?;

        let text = result.text.trim().to_string();
        if text.is_empty() {
            bail!("Moonshine inference returned empty transcript");
        }

        Ok(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MoonshineModelSource {
    LocalDir(PathBuf),
    HuggingFaceRepo,
}

fn resolve_moonshine_model_source_and_variant(
    model_id_or_path: &str,
) -> Result<(MoonshineModelSource, ModelVariant)> {
    let local_path = Path::new(model_id_or_path);
    let is_dir = local_path.is_dir();

    let lowercase_id = model_id_or_path.to_ascii_lowercase();

    let variant = if lowercase_id.contains("tiny") {
        ModelVariant::Tiny
    } else if lowercase_id.contains("base") {
        ModelVariant::Base
    } else {
        bail!(
            "unsupported Moonshine variant '{}': only 'tiny' and 'base' are supported",
            model_id_or_path
        );
    };

    if is_dir {
        Ok((
            MoonshineModelSource::LocalDir(local_path.to_path_buf()),
            variant,
        ))
    } else {
        Ok((MoonshineModelSource::HuggingFaceRepo, variant))
    }
}

fn download_moonshine_model_dir(variant: ModelVariant) -> Result<PathBuf> {
    let subdir = match variant {
        ModelVariant::Tiny => TINY_SUBDIR,
        ModelVariant::Base => BASE_SUBDIR,
        _ => unreachable!(),
    };

    let api = Api::new().context("failed to initialize Hugging Face API")?;
    let repo = api.repo(Repo::with_revision(
        DEFAULT_MOONSHINE_REPO.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));

    let mut model_dir: Option<PathBuf> = None;
    let mut tokenizer_src_path: Option<PathBuf> = None;

    for filename in REQUIRED_FILES {
        let repo_path = if *filename == "tokenizer.json" {
            let variant_name = if subdir.contains("tiny") {
                "tiny"
            } else {
                "base"
            };
            format!("ctranslate2/{}/tokenizer.json", variant_name)
        } else {
            format!("{}/{}", subdir, filename)
        };

        let downloaded = repo.get(&repo_path).with_context(|| {
            format!(
                "failed to fetch '{}' from '{}'",
                repo_path, DEFAULT_MOONSHINE_REPO
            )
        })?;

        if *filename == "tokenizer.json" {
            tokenizer_src_path = Some(downloaded);
        } else {
            let parent = downloaded.parent().ok_or_else(|| {
                anyhow::anyhow!(
                    "could not resolve parent directory for '{}'",
                    downloaded.display()
                )
            })?;

            if let Some(existing_dir) = &model_dir {
                if existing_dir != parent {
                    bail!(
                        "downloaded model files from '{}' did not resolve to one directory",
                        DEFAULT_MOONSHINE_REPO
                    );
                }
            } else {
                model_dir = Some(parent.to_path_buf());
            }
        }
    }

    let resolved_model_dir = model_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "failed to resolve model directory for '{}'",
            DEFAULT_MOONSHINE_REPO
        )
    })?;

    if let Some(tok_src) = tokenizer_src_path {
        let tok_dest = resolved_model_dir.join("tokenizer.json");
        if !tok_dest.exists() {
            std::fs::copy(&tok_src, &tok_dest).with_context(|| {
                format!(
                    "failed to copy tokenizer.json to model directory '{}'",
                    resolved_model_dir.display()
                )
            })?;
        }
    }

    Ok(resolved_model_dir)
}

#[cfg(test)]
mod tests {
    use super::{ModelVariant, MoonshineModelSource, resolve_moonshine_model_source_and_variant};
    use tempfile::tempdir;

    #[test]
    fn variant_tiny_accepted() {
        let (_, variant) = resolve_moonshine_model_source_and_variant("moonshine-tiny").unwrap();
        assert_eq!(variant, ModelVariant::Tiny);
    }

    #[test]
    fn variant_base_accepted() {
        let (_, variant) = resolve_moonshine_model_source_and_variant("moonshine-base").unwrap();
        assert_eq!(variant, ModelVariant::Base);
    }

    #[test]
    fn variant_small_rejected() {
        let err = resolve_moonshine_model_source_and_variant("moonshine-small").unwrap_err();
        assert!(
            err.to_string()
                .contains("only 'tiny' and 'base' are supported")
        );
    }

    #[test]
    fn source_resolves_local_dir() {
        let tmp_dir = tempdir().expect("failed to create temp directory");
        // Must contain "tiny" or "base" to be valid even as a local dir
        let path = tmp_dir.path().join("moonshine-tiny");
        std::fs::create_dir(&path).unwrap();

        let input = path.display().to_string();
        let (source, variant) = resolve_moonshine_model_source_and_variant(&input).unwrap();
        assert_eq!(source, MoonshineModelSource::LocalDir(path));
        assert_eq!(variant, ModelVariant::Tiny);
    }

    #[test]
    fn source_resolves_to_repo() {
        let path = "non-existent-path/moonshine-base";
        let (source, variant) = resolve_moonshine_model_source_and_variant(path).unwrap();
        assert_eq!(source, MoonshineModelSource::HuggingFaceRepo);
        assert_eq!(variant, ModelVariant::Base);
    }
}
