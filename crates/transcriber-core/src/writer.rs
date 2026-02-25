use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_transcript<P: AsRef<Path>>(path: P, text: &str) -> Result<()> {
    let mut file = File::create(path).context("Failed to create output file")?;
    writeln!(file, "{}", text).context("Failed to write to file")?;
    Ok(())
}
