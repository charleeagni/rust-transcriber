use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Transcribe a single audio file
    Transcribe {
        /// Input audio file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output transcript file path
        #[arg(short, long)]
        output: PathBuf,

        /// Runtime backend selection
        #[arg(long, value_enum, default_value_t = RuntimeArg::Auto)]
        runtime: RuntimeArg,

        /// Optional model ID or path override
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Watch a directory for audio files and transcribe them automatically
    Watch {
        /// Directory to watch for incoming files
        #[arg(
            short,
            long,
            default_value = "/Users/karthik/Library/Group Containers/group.com.apple.VoiceMemos.shared/Recordings"
        )]
        watch_dir: String,

        /// Directory to save transcripts
        #[arg(short, long, default_value = "./watch_out")]
        output_dir: String,

        /// Runtime backend selection
        #[arg(long, value_enum, default_value_t = RuntimeArg::Auto)]
        runtime: RuntimeArg,

        /// Optional model ID or path override
        #[arg(short, long)]
        model: Option<String>,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum RuntimeArg {
    Whisper,
    Parakeet,
    Auto,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
