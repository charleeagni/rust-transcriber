# Low Level Design: Rust-Only Runtime Migration (CODIN-178)

## 1. Scope

This LLD covers replacing Python Parakeet runtime execution with Rust-native inference for Parakeet models only, while preserving current CLI and watcher workflows.

Included:
- Runtime implementation replacement in `transcriber-core`.
- Dependency and model-loading changes needed for Rust inference.
- CLI/watch compatibility behavior.
- Documentation updates to remove Python runtime setup.
- Explicitly no implementation change to Whisper runtime logic.

Not included:
- New queueing behavior.
- UI changes.
- Worker pool redesign.
- Broad module refactors outside runtime path migration.
- Any change to Whisper model loading, decoding, inference, or output behavior.

## 2. Compatibility Decisions

1. Keep CLI surface stable:
- Keep existing `--runtime whisper|parakeet|auto`.
- Keep existing `--model` argument contract.

2. Remove Python from runtime execution:
- No Python binary discovery.
- No Python subprocess invocation.
- No Python preflight checks.

3. Keep watcher behavior intact:
- Existing watcher entrypoints remain.
- Runtime configuration continues to pass through unchanged.

## 3. Dependency Changes

Workspace dependency additions:
- Add `transcribe-rs` with `parakeet` feature enabled.

Reason:
- Provides Rust-native Parakeet engine and inference API.

## 4. File-Level Design

### 4.1 `crates/transcriber-core/src/transcriber.rs`

Replace Python-backed Parakeet implementation with Rust-backed implementation:
- Remove Python script constants and output marker parsing.
- Remove `std::process::Command` runtime invocation.
- Keep `RuntimeSelection` and `RuntimeBackend` enum values for compatibility.
- Reimplement `ParakeetTranscriber` using `transcribe_rs::engines::parakeet::ParakeetEngine`.
- Keep `WhisperTranscriber` implementation unchanged.

Model resolution behavior:
- If `--model` points to an existing local directory, treat it as Parakeet model directory.
- If `--model` is not a local directory, treat it as a Hugging Face repo id and materialize required files through `hf-hub`.
- If `--model` is not provided, use existing default Parakeet model identifier and resolve through the same path.

Parakeet inference path:
- Decode source audio into mono 16kHz `Vec<f32>` using existing audio loader.
- Call Parakeet engine sample-based transcription API.
- Return transcript text from Rust engine result.

Error mapping:
- Preserve high-level error boundaries (`Error loading model`, `Error transcribing`) so CLI and watcher output format stays stable.

### 4.2 `crates/transcriber-core/src/lib.rs`

No interface changes.

Keep current branching:
- Whisper path uses decoded PCM with `transcribe_pcm`.
- Parakeet path uses `transcribe_path`.

Parakeet `transcribe_path` now uses Rust backend internally.
Whisper path behavior remains unchanged.

### 4.3 `crates/transcriber-core/src/watcher.rs`

No API changes.

Watcher behavior remains:
- Runtime is selected once at startup.
- For Parakeet backend, `transcribe_path` is called per file.
- Output writing and processed-file dedup remain unchanged.

### 4.4 `crates/transcriber-cli/src/cli.rs` and `crates/transcriber-cli/src/main.rs`

No argument contract changes.

Keep runtime value mapping as-is to avoid breaking existing command usage.
No CLI behavior change specific to Whisper.

### 4.5 `README.md` and runtime docs

Remove Python-specific setup steps:
- Remove `.venv311` / `PARAKEET_PYTHON_BIN` runtime guidance.
- Remove Python install commands tied to Parakeet runtime execution.

Add Rust-only prerequisites and model path expectations.

## 5. Runtime Resolution Rules

`RuntimeSelection::Whisper`:
- Existing Whisper implementation remains unchanged.
- No model-loading or inference-path modifications are planned.

`RuntimeSelection::Parakeet`:
- Always uses Rust Parakeet engine.
- Fails fast if model directory/repo cannot be resolved.

`RuntimeSelection::Auto`:
- Keeps current model-hint behavior.
- Uses same runtime resolution rules after model hint evaluation.

## 6. Test Plan

Unit tests in `transcriber-core`:
- Keep runtime hint tests for Whisper/Parakeet selection.
- Remove obsolete Python output parsing test.
- Add model-resolution tests for:
  - existing local directory path
  - non-path repo id fallback behavior

CLI tests:
- Keep existing required-argument failure tests.

Manual validation:
- `transcribe` command with `--runtime parakeet` succeeds without Python installed.
- `watch` mode processes `.m4a` files with parakeet runtime and writes `.txt`.
- Whisper runtime still functions for regression safety.
- Whisper output parity with current baseline is maintained.

## 7. Rollout Steps

1. Land dependency and core runtime replacement.
2. Update docs to remove Python runtime instructions.
3. Run CLI/watch smoke checks for parakeet and whisper runtime modes.

## 8. Acceptance Mapping

1. No Python subprocess required:
- Satisfied by removing Python execution and preflight paths.

2. Rust-only transcription succeeds:
- Satisfied by Rust Parakeet engine integration and end-to-end CLI/watch validation.

3. Existing CLI command remains operational:
- Satisfied by preserving command arguments and runtime mapping.

4. Packaging compatibility improves:
- Satisfied by removing runtime dependency on Python interpreter and Python packages.

5. Whisper implementation does not change:
- Satisfied by retaining existing Whisper code path and validating regression safety.
