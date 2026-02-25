use anyhow::{Context, Result};
use notify::event::CreateKind;
use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use crate::audio;
use crate::transcriber;
use crate::writer;

pub struct WatchConfig {
    pub watch_dir: PathBuf,
    pub output_dir: PathBuf,
    pub runtime: transcriber::RuntimeSelection,
    pub model_id: Option<String>,
}

pub struct WatchHandle {
    stop_tx: mpsc::Sender<()>,
    is_running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl WatchHandle {
    pub fn stop(&mut self) -> Result<()> {
        let _ = self.stop_tx.send(());
        self.is_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn join(mut self) -> Result<()> {
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }
}

impl Drop for WatchHandle {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub type WatchCallback = Box<dyn Fn(String, &Path) + Send + 'static>;

fn init_watcher_state(
    config: &WatchConfig,
) -> Result<(
    transcriber::Transcriber,
    notify::RecommendedWatcher,
    mpsc::Receiver<notify::Result<notify::Event>>,
)> {
    std::fs::create_dir_all(&config.watch_dir)?;
    std::fs::create_dir_all(&config.output_dir)?;

    println!("[Watcher] Loading runtime: {:?}", config.runtime);
    let transcription_config = transcriber::TranscriptionConfig {
        runtime: config.runtime,
        model_id: config.model_id.clone(),
    };
    let transcriber_inst = transcriber::Transcriber::new(&transcription_config)
        .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;

    let (tx, rx) = mpsc::channel();
    let mut watcher = recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;

    watcher.watch(&config.watch_dir, RecursiveMode::NonRecursive)?;

    println!(
        "[Watcher] Watching for .m4a files in: {:?}",
        config.watch_dir
    );

    Ok((transcriber_inst, watcher, rx))
}

pub fn spawn_watch_directory(
    config: WatchConfig,
    on_transcribed: WatchCallback,
) -> Result<WatchHandle> {
    let (transcriber_inst, watcher, rx) = init_watcher_state(&config)?;

    let (stop_tx, stop_rx) = mpsc::channel();
    let is_running = Arc::new(AtomicBool::new(true));

    let is_running_clone = is_running.clone();

    struct WorkerGuard(Arc<AtomicBool>);
    impl Drop for WorkerGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    let join_handle = thread::spawn(move || {
        let _guard = WorkerGuard(is_running_clone.clone());
        if let Err(e) = run_watch_loop_core(
            config,
            on_transcribed,
            stop_rx,
            is_running_clone,
            transcriber_inst,
            watcher,
            rx,
        ) {
            eprintln!("[Watcher] Loop exited with error: {:?}", e);
        }
    });

    Ok(WatchHandle {
        stop_tx,
        is_running,
        join_handle: Some(join_handle),
    })
}

pub fn watch_directory(config: WatchConfig, on_transcribed: WatchCallback) -> Result<()> {
    let (transcriber_inst, watcher, rx) = init_watcher_state(&config)?;
    let (_stop_tx, stop_rx) = mpsc::channel();
    let is_running = Arc::new(AtomicBool::new(true));
    run_watch_loop_core(
        config,
        on_transcribed,
        stop_rx,
        is_running,
        transcriber_inst,
        watcher,
        rx,
    )
}

// Keeping the older entrypoint for CLI compatibility if desired,
// though we usually prefer the new API.
pub fn start_m4a_watcher(watch_dir: &str, output_dir: &str, model_id: &str) {
    start_m4a_watcher_with_config(
        watch_dir,
        output_dir,
        transcriber::RuntimeSelection::Whisper,
        Some(model_id.to_string()),
    );
}

pub fn start_m4a_watcher_with_config(
    watch_dir: &str,
    output_dir: &str,
    runtime: transcriber::RuntimeSelection,
    model_id: Option<String>,
) {
    let config = WatchConfig {
        watch_dir: PathBuf::from(watch_dir),
        output_dir: PathBuf::from(output_dir),
        runtime,
        model_id,
    };

    let _ = watch_directory(
        config,
        Box::new(|text, path| {
            println!(
                "[PostExecute] Finished processing: {:?}",
                path.file_name().unwrap()
            );
            println!("[PostExecute] Transcript length: {} chars", text.len());
        }),
    );
}

fn run_watch_loop_core(
    config: WatchConfig,
    on_transcribed: WatchCallback,
    stop_rx: mpsc::Receiver<()>,
    is_running: Arc<AtomicBool>,
    mut transcriber_inst: transcriber::Transcriber,
    _watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<notify::Event>>,
) -> Result<()> {
    let processed_file = config.output_dir.join(".processed_files.json");
    let mut processed: std::collections::HashSet<String> = load_processed_files(&processed_file);

    while is_running.load(Ordering::SeqCst) {
        if stop_rx.try_recv().is_ok() {
            println!("[Watcher] Stop signal received.");
            break;
        }

        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(event)) => {
                if matches!(event.kind, EventKind::Create(CreateKind::File)) {
                    handle_file_create_event(
                        event,
                        &config,
                        &mut transcriber_inst,
                        &mut processed,
                        &processed_file,
                        &on_transcribed,
                    );
                }
            }
            Ok(Err(e)) => eprintln!("[Watcher] Watch error: {:?}", e),
            Err(mpsc::RecvTimeoutError::Timeout) => {} // Heartbeat
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("[Watcher] Channel disconnected. Stopping.");
                break;
            }
        }
    }

    Ok(())
}

fn load_processed_files(path: &Path) -> std::collections::HashSet<String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_processed_files(path: &Path, processed: &std::collections::HashSet<String>) {
    if let Ok(json) = serde_json::to_string(processed) {
        let _ = std::fs::write(path, json);
    }
}

fn handle_file_create_event(
    event: notify::Event,
    config: &WatchConfig,
    transcriber_inst: &mut transcriber::Transcriber,
    processed: &mut std::collections::HashSet<String>,
    processed_file: &Path,
    on_transcribed: &WatchCallback,
) {
    for path in event.paths {
        if path.extension().and_then(|e| e.to_str()) != Some("m4a") {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let dedup_key = format!("{}_{}", file_name, file_size);

        if processed.contains(&dedup_key) {
            println!("[Watcher] Skipping already processed file: {}", file_name);
            continue;
        }

        println!("[Watcher] New m4a detected: {}", file_name);
        println!("[Watcher] Transcribing...");

        match transcribe_single_file(&path, transcriber_inst) {
            Ok(text) => {
                let output_file_name = file_name.replace(".m4a", ".txt");
                let output_path = config.output_dir.join(&output_file_name);

                if let Err(e) = writer::write_transcript(&output_path, &text) {
                    eprintln!("[Watcher] Failed to write transcript: {}", e);
                    continue;
                }

                processed.insert(dedup_key);
                save_processed_files(processed_file, processed);

                on_transcribed(text, &path);
            }
            Err(e) => {
                eprintln!("[Watcher] Error processing {}: {}", file_name, e);
            }
        }
    }
}

fn transcribe_single_file(
    path: &Path,
    transcriber_inst: &mut transcriber::Transcriber,
) -> Result<String> {
    match transcriber_inst.backend() {
        transcriber::RuntimeBackend::Whisper => {
            let audio_data = audio::load_audio(path).context("Error decoding audio")?;
            transcriber_inst
                .transcribe_pcm(&audio_data)
                .context("Error transcribing with Whisper")
        }
        transcriber::RuntimeBackend::Parakeet => transcriber_inst
            .transcribe_path(path)
            .context("Error transcribing with Parakeet"),
    }
}
