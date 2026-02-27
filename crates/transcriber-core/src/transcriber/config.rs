use crate::transcriber::parakeet::ParakeetTranscriber;

pub const DEFAULT_WHISPER_MODEL: &str = "openai/whisper-tiny";
pub const DEFAULT_PARAKEET_MODEL: &str = "mlx-community/parakeet-tdt-0.6b-v3";
pub const DEFAULT_MOONSHINE_MODEL: &str = crate::transcriber::moonshine::DEFAULT_MOONSHINE_MODEL;
pub(crate) const DEFAULT_PARAKEET_ONNX_REPO: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSelection {
    Whisper,
    Parakeet,
    Moonshine,
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionConfig {
    pub runtime: RuntimeSelection,
    pub model_id: Option<String>,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeSelection::Auto,
            model_id: None,
        }
    }
}

impl TranscriptionConfig {
    pub fn whisper(model_id: impl Into<String>) -> Self {
        Self {
            runtime: RuntimeSelection::Whisper,
            model_id: Some(model_id.into()),
        }
    }

    pub fn parakeet(model_id: Option<String>) -> Self {
        Self {
            runtime: RuntimeSelection::Parakeet,
            model_id,
        }
    }

    pub fn moonshine(model_id: Option<String>) -> Self {
        Self {
            runtime: RuntimeSelection::Moonshine,
            model_id,
        }
    }

    pub(crate) fn resolve_runtime(&self) -> RuntimeSelection {
        match self.runtime {
            RuntimeSelection::Whisper => RuntimeSelection::Whisper,
            RuntimeSelection::Parakeet => RuntimeSelection::Parakeet,
            RuntimeSelection::Moonshine => RuntimeSelection::Moonshine,
            RuntimeSelection::Auto => {
                if let Some(model_id) = self.model_id.as_deref() {
                    let lowercase_id = model_id.to_ascii_lowercase();
                    if lowercase_id.contains("whisper") {
                        return RuntimeSelection::Whisper;
                    }
                    if lowercase_id.contains("moonshine") {
                        return RuntimeSelection::Moonshine;
                    }
                    if lowercase_id.contains("parakeet") {
                        return RuntimeSelection::Parakeet;
                    }
                }
                if ParakeetTranscriber::is_available() {
                    RuntimeSelection::Parakeet
                } else {
                    RuntimeSelection::Whisper
                }
            }
        }
    }

    pub(crate) fn resolve_model_id(&self, runtime: RuntimeSelection) -> String {
        match (&self.model_id, runtime) {
            (Some(model_id), _) => model_id.clone(),
            (None, RuntimeSelection::Moonshine) => DEFAULT_MOONSHINE_MODEL.to_string(),
            (None, RuntimeSelection::Parakeet) => DEFAULT_PARAKEET_MODEL.to_string(),
            _ => DEFAULT_WHISPER_MODEL.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeSelection, TranscriptionConfig};

    #[test]
    fn auto_runtime_uses_model_hint_for_whisper() {
        let config = TranscriptionConfig {
            runtime: RuntimeSelection::Auto,
            model_id: Some("openai/whisper-base".to_string()),
        };
        assert_eq!(config.resolve_runtime(), RuntimeSelection::Whisper);
    }

    #[test]
    fn auto_runtime_uses_model_hint_for_parakeet() {
        let config = TranscriptionConfig {
            runtime: RuntimeSelection::Auto,
            model_id: Some("mlx-community/parakeet-tdt-0.6b-v3".to_string()),
        };
        assert_eq!(config.resolve_runtime(), RuntimeSelection::Parakeet);
    }

    #[test]
    fn auto_runtime_uses_model_hint_for_moonshine() {
        let config = TranscriptionConfig {
            runtime: RuntimeSelection::Auto,
            model_id: Some("moonshine-tiny".to_string()),
        };
        assert_eq!(config.resolve_runtime(), RuntimeSelection::Moonshine);
    }

    #[test]
    fn default_model_id_for_moonshine() {
        let config = TranscriptionConfig {
            runtime: RuntimeSelection::Moonshine,
            model_id: None,
        };
        assert_eq!(
            config.resolve_model_id(RuntimeSelection::Moonshine),
            crate::transcriber::moonshine::DEFAULT_MOONSHINE_MODEL
        );
    }
}
