mod cli;

use std::path::Path;
use std::process;
use transcriber_core::{
    RuntimeSelection, TranscriptionConfig, start_m4a_watcher_with_config,
    transcribe_file_with_config,
};

fn map_runtime(runtime: cli::RuntimeArg) -> RuntimeSelection {
    match runtime {
        cli::RuntimeArg::Whisper => RuntimeSelection::Whisper,
        cli::RuntimeArg::Parakeet => RuntimeSelection::Parakeet,
        cli::RuntimeArg::Auto => RuntimeSelection::Auto,
    }
}

fn main() {
    let args = cli::parse_args();

    match args.command {
        cli::Commands::Transcribe {
            input,
            output,
            runtime,
            model,
        } => {
            println!("Transcribing {:?} to {:?}", input, output);

            let config = TranscriptionConfig {
                runtime: map_runtime(runtime),
                model_id: model,
            };

            if let Err(e) =
                transcribe_file_with_config(Path::new(&input), Path::new(&output), &config)
            {
                eprintln!("Error during transcription: {:?}", e);
                process::exit(1);
            }

            println!("Transcription finished.");
        }
        cli::Commands::Watch {
            watch_dir,
            output_dir,
            runtime,
            model,
        } => {
            println!("Starting auto-transcribe watcher mode...");
            start_m4a_watcher_with_config(&watch_dir, &output_dir, map_runtime(runtime), model);
        }
    }
}
