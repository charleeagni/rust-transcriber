use anyhow::Result;
use tempfile::tempdir;

use transcriber_core::{WatchConfig, spawn_watch_directory};

#[test]
fn test_spawn_watch_directory_bubbles_up_model_errors() -> Result<()> {
    let watch_dir = tempdir()?;
    let output_dir = tempdir()?;

    let config = WatchConfig {
        watch_dir: watch_dir.path().to_path_buf(),
        output_dir: output_dir.path().to_path_buf(),
        runtime: transcriber_core::RuntimeSelection::Whisper,
        // A fake model ID will now make initialization fail synchronously, rather than silently in the background
        model_id: Some("fake/model-id-123".to_string()),
    };

    // Start watcher with a bad model
    let handle_result = spawn_watch_directory(config, Box::new(move |_text, _path| {}));

    // Assert that it returns an error immediately
    assert!(handle_result.is_err());
    if let Err(e) = handle_result {
        let err_msg = e.to_string();
        assert!(err_msg.contains("Failed to load model"));
    }

    Ok(())
}
