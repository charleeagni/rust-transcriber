use anyhow::{Context, Result, anyhow};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

struct AudioStream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: usize,
}

pub fn load_audio<P: AsRef<Path>>(input_path: P) -> Result<Vec<f32>> {
    let stream = open_audio_stream(input_path.as_ref())?;
    let sample_rate = stream.sample_rate;

    let samples = decode_audio_stream(stream)?;
    resample_audio(samples, sample_rate)
}

fn open_audio_stream(input_path: &Path) -> Result<AudioStream> {
    let src = File::open(input_path).context("failed to open input file")?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("m4a"); // Optional but helps probe

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)
        .context("unsupported format")?;

    let (sample_rate, channels, track_id, codec_params) = {
        let track = probed
            .format
            .default_track()
            .ok_or_else(|| anyhow!("no default track"))?;
        let sr = track.codec_params.sample_rate.unwrap_or(44100);
        let c = track
            .codec_params
            .channels
            .unwrap_or(symphonia::core::audio::Channels::FRONT_CENTRE)
            .count();
        (sr, c, track.id, track.codec_params.clone())
    };

    let dec_opts: DecoderOptions = Default::default();
    let decoder = symphonia::default::get_codecs()
        .make(&codec_params, &dec_opts)
        .context("unsupported codec")?;

    let format = probed.format;

    Ok(AudioStream {
        format,
        decoder,
        track_id,
        sample_rate,
        channels,
    })
}

fn decode_audio_stream(mut stream: AudioStream) -> Result<Vec<f32>> {
    let mut samples = Vec::new();
    let mut sample_buf = None;

    loop {
        let packet = match stream.format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(err)) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof
                    || err.kind() == std::io::ErrorKind::ConnectionReset
                {
                    break;
                }
                return Err(anyhow::anyhow!(err));
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(err) => {
                let s = err.to_string();
                if s.contains("end of stream") || s.contains("EOF") || s.contains("exhausted") {
                    break;
                }
                return Err(anyhow::anyhow!(s));
            }
        };

        if packet.track_id() != stream.track_id {
            continue;
        }

        match stream.decoder.decode(&packet) {
            Ok(audio_buf) => {
                if sample_buf.is_none() {
                    let spec = *audio_buf.spec();
                    let duration = audio_buf.capacity() as u64;
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);

                    let frames = buf.samples();
                    downmix_to_mono(&mut samples, frames, stream.channels);
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => {}
            Err(_) => break, // EOF or other error
        }
    }

    Ok(samples)
}

fn downmix_to_mono(target: &mut Vec<f32>, frames: &[f32], channels: usize) {
    if channels > 1 {
        for chunk in frames.chunks(channels) {
            let sum: f32 = chunk.iter().sum();
            target.push(sum / channels as f32);
        }
    } else {
        target.extend_from_slice(frames);
    }
}

fn resample_audio(samples: Vec<f32>, sample_rate: u32) -> Result<Vec<f32>> {
    if sample_rate == 16000 {
        return Ok(samples);
    }

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f32>::new(
        16000_f64 / sample_rate as f64,
        2.0,
        params,
        samples.len(),
        1,
    )
    .context("failed to create resampler")?;

    let waves_in = vec![samples];
    let mut waves_out = resampler
        .process(&waves_in, None)
        .context("failed to process audio resampling")?;

    Ok(waves_out.remove(0))
}
