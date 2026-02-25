# High Level Design: Phase 0 Native Rust MLX Transcriber (CODIN-162)

## 1. Objective

Deliver a minimal native transcriber that runs outside Docker and uses MLX on macOS.

Scope is intentionally narrow:
- Input: one audio file path via CLI argument.
- Processing: MLX Whisper transcription.
- Output: one transcript text file.

Out of scope:
- Watchers.
- Django/Temporal integration.
- Background services.
- UI wiring.
- Multi-file batching.

## 2. Problem Context

Current transcription logic relies on Python in the backend stack. Running this inside Docker prevents direct use of Apple Metal acceleration for MLX workloads.

Phase 0 creates a standalone Rust transcriber executable so transcription can run natively on host macOS and avoid Docker GPU limitations.

## 3. Proposed Solution

Build a Rust CLI executable with a single responsibility:
- Read an audio file path from CLI.
- Run transcription with MLX Whisper.
- Write transcript text to an output file path.

This executable becomes the base unit we can later compose into watchers, app orchestration, and backend callbacks.

## 4. Interface Contract

CLI contract for Phase 0:
- Required input: `--input <audio_file_path>`.
- Required output: `--output <transcript_file_path>`.
- Optional model: `--model <mlx_model_id_or_path>`.

Behavior:
- Exit code `0` on success.
- Non-zero exit code on validation, decoding, model, or inference errors.
- Writes UTF-8 transcript text to output path.

## 5. Design Principles

- Atomic: single file in, single file out.
- Deterministic: no side effects outside explicit output file.
- Minimal dependencies: only what is needed for decode + infer + write.
- Reusable core: keep transcription logic separable from CLI argument parsing.

## 6. Risks and Mitigations

- MLX Rust maturity risk:
  - Mitigation: keep Phase 0 contract stable and implementation modular to allow swapping inference backend if needed.
- Audio decoding format variance:
  - Mitigation: define supported formats for Phase 0 and fail fast with clear errors.
- First-run model acquisition latency:
  - Mitigation: support explicit model argument and document default model behavior.

## 7. Success Criteria

- Running one CLI command with a valid audio path produces a transcript file.
- No Docker dependency for transcription path.
- Runs locally on macOS and is ready to be integrated in later phases.
