# Low Level Design: Phase 0 Native Rust MLX Transcriber (CODIN-162)

## 1. Scope

This LLD only covers the atomic transcriber binary:
- One audio file input path from CLI.
- One MLX Whisper inference run.
- One transcript file output path.

Not included:
- Watchers.
- Tauri triggers.
- Django/Temporal API calls.
- Queueing or retries.

## 2. CLI Contract

### 2.1 Command

`meml-transcriber --input <audio_path> --output <text_path> [--model <model_id_or_path>]`

### 2.2 Argument Rules

- `--input` is required and must point to an existing readable file.
- `--output` is required and parent directory must exist or be creatable.
- `--model` is optional.
  - Default: `mlx-community/whisper-small-mlx`.

### 2.3 Exit Codes

- `0`: Success.
- `1`: Argument validation failure.
- `2`: Audio decode failure.
- `3`: Model load failure.
- `4`: Inference failure.
- `5`: Output write failure.

## 3. Internal Components

### 3.1 `cli`

Responsibilities:
- Parse arguments.
- Validate input/output paths.
- Build runtime config struct.
- Drive end-to-end execution flow.

### 3.2 `audio`

Responsibilities:
- Decode input media to PCM.
- Normalize to mono 16kHz `f32` buffer expected by Whisper path.
- Return explicit typed decode errors.

### 3.3 `transcriber`

Responsibilities:
- Load MLX Whisper model from `--model` or default.
- Run inference on normalized audio samples.
- Return plain transcript text.

### 3.4 `writer`

Responsibilities:
- Write transcript as UTF-8 text file.
- Ensure deterministic output with trailing newline policy.

## 4. Processing Sequence

1. Parse and validate CLI arguments.
2. Decode input audio into normalized PCM samples.
3. Load MLX Whisper model.
4. Run transcription and get transcript text.
5. Write transcript text to output file.
6. Return success exit code.

If any step fails:
- Print concise error to stderr.
- Return mapped non-zero exit code.

## 5. Output Format

- File contents: transcript plain text in UTF-8.
- No JSON or metadata in Phase 0.
- Single file output only.

## 6. Logging and Errors

- Human-readable stderr errors for failures.
- Optional info logs for start/finish and model used.
- No structured telemetry in Phase 0.

## 7. Compatibility Notes

- Target platform for Phase 0: macOS Apple Silicon.
- Objective is native host execution to enable Metal-backed MLX path.
- Docker integration is intentionally excluded from this phase.

## 8. Test Plan (Phase 0)

- Valid `.m4a` input produces non-empty `.txt` output.
- Missing input path returns exit code `1`.
- Corrupt/unsupported audio returns exit code `2`.
- Invalid model identifier returns exit code `3`.
- Read-only output location returns exit code `5`.

## 9. Deferred Work

- Directory watcher.
- Service mode and long-lived model lifecycle.
- API callbacks to MeML backend.
- UI integration and progress reporting.
