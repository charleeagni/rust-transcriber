use anyhow::{Error as E, Result};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config, audio};
use hf_hub::{Repo, RepoType, api::sync::Api};
use tokenizers::Tokenizer;

#[allow(dead_code)]
enum WhisperModel {
    Normal(m::model::Whisper),
    Quantized(m::quantized_model::Whisper),
}

impl WhisperModel {
    #[allow(dead_code)]
    fn config(&self) -> &Config {
        match self {
            Self::Normal(m) => &m.config,
            Self::Quantized(m) => &m.config,
        }
    }

    fn encoder_forward(&mut self, x: &Tensor, flush: bool) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.encoder.forward(x, flush),
            Self::Quantized(m) => m.encoder.forward(x, flush),
        }
    }

    fn decoder_forward(
        &mut self,
        x: &Tensor,
        xa: &Tensor,
        flush: bool,
    ) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.decoder.forward(x, xa, flush),
            Self::Quantized(m) => m.decoder.forward(x, xa, flush),
        }
    }

    fn decoder_final_linear(&self, x: &Tensor) -> candle_core::Result<Tensor> {
        match self {
            Self::Normal(m) => m.decoder.final_linear(x),
            Self::Quantized(m) => m.decoder.final_linear(x),
        }
    }
}

pub(crate) struct WhisperTranscriber {
    model: WhisperModel,
    tokenizer: Tokenizer,
    config: Config,
    device: Device,
    mel_filters: Vec<f32>,
    sot_token: u32,
    transcribe_token: u32,
    eot_token: u32,
    no_timestamps_token: u32,
    #[allow(dead_code)]
    no_speech_token: u32,
}

impl WhisperTranscriber {
    pub(crate) fn new(model_id_or_path: &str) -> Result<Self> {
        let device = Device::new_metal(0).unwrap_or(Device::Cpu);

        let (config_filename, tokenizer_filename, weights_filename) = {
            let api = Api::new()?;
            let repo = api.repo(Repo::with_revision(
                model_id_or_path.to_string(),
                RepoType::Model,
                "main".to_string(),
            ));
            let config = repo.get("config.json")?;
            let tokenizer = repo.get("tokenizer.json")?;
            let model = repo.get("model.safetensors")?;
            (config, tokenizer, model)
        };

        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_filename)?)?;
        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(E::msg)?;

        let mel_bytes = match config.num_mel_bins {
            80 => include_bytes!("../melfilters.bytes").as_slice(),
            128 => include_bytes!("../melfilters128.bytes").as_slice(),
            nmel => anyhow::bail!("unexpected num_mel_bins {nmel}"),
        };
        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        <byteorder::LittleEndian as byteorder::ByteOrder>::read_f32_into(
            mel_bytes,
            &mut mel_filters,
        );

        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], m::DTYPE, &device)? };
        let model = WhisperModel::Normal(m::model::Whisper::load(&vb, config.clone())?);

        let sot_token = token_id(&tokenizer, m::SOT_TOKEN)?;
        let transcribe_token = token_id(&tokenizer, m::TRANSCRIBE_TOKEN)?;
        let eot_token = token_id(&tokenizer, m::EOT_TOKEN)?;
        let no_timestamps_token = token_id(&tokenizer, m::NO_TIMESTAMPS_TOKEN)?;

        let no_speech_token = m::NO_SPEECH_TOKENS
            .iter()
            .find_map(|token| token_id(&tokenizer, token).ok())
            .unwrap_or(0);

        Ok(Self {
            model,
            tokenizer,
            config,
            device,
            mel_filters,
            sot_token,
            transcribe_token,
            eot_token,
            no_timestamps_token,
            no_speech_token,
        })
    }

    pub(crate) fn transcribe_pcm(&mut self, pcm_data: &[f32]) -> Result<String> {
        let mel = audio::pcm_to_mel(&self.config, pcm_data, &self.mel_filters);
        let mel_len = mel.len();
        let mel_tensor = Tensor::from_vec(
            mel,
            (
                1,
                self.config.num_mel_bins,
                mel_len / self.config.num_mel_bins,
            ),
            &self.device,
        )?;

        let (_, _, content_frames) = mel_tensor.dims3()?;
        let mut seek = 0;
        let mut full_text = String::new();

        while seek < content_frames {
            let segment_size = usize::min(content_frames - seek, m::N_FRAMES);
            let mel_segment = mel_tensor.narrow(2, seek, segment_size)?;

            let text = self.decode_segment(&mel_segment)?;
            full_text.push_str(&text);

            seek += segment_size;
        }

        Ok(full_text)
    }

    fn decode_segment(&mut self, mel: &Tensor) -> Result<String> {
        let audio_features = self.model.encoder_forward(mel, true)?;
        let sample_len = self.config.max_target_positions / 2;

        let mut tokens = vec![self.sot_token];
        if let Ok(en_token) = token_id(&self.tokenizer, "<|en|>") {
            tokens.push(en_token);
        }
        tokens.push(self.transcribe_token);
        tokens.push(self.no_timestamps_token);

        for i in 0..sample_len {
            let tokens_t = Tensor::new(tokens.as_slice(), mel.device())?.unsqueeze(0)?;
            let ys = self
                .model
                .decoder_forward(&tokens_t, &audio_features, i == 0)?;

            let (_, seq_len, _) = ys.dims3()?;
            let logits = self
                .model
                .decoder_final_linear(&ys.i((..1, seq_len - 1..))?)?
                .i(0)?
                .i(0)?;

            let logits_v: Vec<f32> = logits.to_vec1()?;
            let next_token = logits_v
                .iter()
                .enumerate()
                .max_by(|(_, u), (_, v)| u.total_cmp(v))
                .map(|(i, _)| i as u32)
                .unwrap();

            tokens.push(next_token);

            if next_token == self.eot_token || tokens.len() > self.config.max_target_positions {
                break;
            }
        }

        let text = self.tokenizer.decode(&tokens, true).map_err(E::msg)?;
        Ok(text)
    }
}

fn token_id(tokenizer: &Tokenizer, token: &str) -> candle_core::Result<u32> {
    match tokenizer.token_to_id(token) {
        None => candle_core::bail!("no token-id for {token}"),
        Some(id) => Ok(id),
    }
}
