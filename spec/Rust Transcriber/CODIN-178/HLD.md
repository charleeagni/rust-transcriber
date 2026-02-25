# High Level Design: Rust-Only Runtime Migration (CODIN-178)

## 1. Objective

Replace the Python Parakeet runtime path with a pure Rust transcription backend for Parakeet models only.

Primary goals:
- Remove Python subprocess dependency from the transcription flow.
- Keep the existing CLI and watcher usage stable.
- Improve Tauri packaging reliability by using a Rust-only runtime path.
- Keep Whisper implementation and behavior unchanged.

## 2. Problem Context

The current Parakeet path depends on Python environment setup, interpreter resolution, and subprocess execution. This introduces packaging complexity and runtime fragility for desktop distribution.

This work item prioritizes deterministic app packaging over hybrid runtime flexibility.

## 3. Scope

In scope:
- Remove Python runtime invocation from `transcriber-core` transcription path.
- Wire transcription execution to `transcribe-rs` in Rust.
- Preserve command-level behavior where possible for existing users.
- Keep output generation flow unchanged.
- Apply these changes only to the Parakeet runtime path.

Out of scope:
- Python worker pool improvements.
- Queueing and concurrency redesign.
- New UI surfaces.
- Broad architecture refactors outside transcription runtime replacement.
- Any functional or behavioral change to Whisper transcription implementation.

## 4. Proposed Solution

Adopt a single runtime implementation in Rust for transcription:
- `transcriber-cli` remains the entrypoint and argument parser.
- `transcriber-core` owns orchestration and delegates inference to Rust backend code.
- `transcribe-rs` becomes the implementation used by runtime transcription calls.
- Existing writer and file output paths remain intact to minimize behavior drift.

Runtime policy:
- No Python runtime fallback path.
- No Python interpreter discovery logic.
- No Python subprocess transport logic.
- Whisper runtime code path remains the existing implementation.

## 5. High-Level Component View

- CLI Layer (`transcriber-cli`)
  - Accepts user input/output/model/runtime arguments.
  - Calls core transcription API.

- Core Layer (`transcriber-core`)
  - Validates request and input path.
  - Coordinates audio decode, inference, and output writing.
  - Returns stable errors for user-facing CLI behavior.

- Inference Layer (Rust backend using `transcribe-rs`)
  - Loads model/runtime resources.
  - Executes transcription.
  - Returns transcript text and backend errors.

## 6. Behavioral Contract

Expected behavior after migration:
- Existing `transcribe` command remains operational.
- Successful runs produce transcript output files as before.
- Failure modes are surfaced through existing non-zero exit handling.
- No runtime requirement on Python binaries or Python packages.
- Whisper runtime behavior remains unchanged.

## 7. Migration Approach

Phase 1:
- Introduce Rust backend call path inside core transcription flow.

Phase 2:
- Remove Python-specific runtime invocation and fallback logic.

Phase 3:
- Update documentation and runtime prerequisites to Rust-only guidance.

## 8. Risks and Mitigations

- Risk: Functional regressions while replacing backend path.
  - Mitigation: Keep CLI/output contracts stable and validate against sample files.

- Risk: Performance uncertainty versus previous Python path.
  - Mitigation: Keep benchmarking task separate and decision-traceable.

- Risk: Hidden coupling with removed runtime logic.
  - Mitigation: Limit changes to runtime execution boundaries and preserve interfaces.

## 9. Success Criteria

- No Python subprocess is used for transcription.
- Transcription succeeds using Rust-only backend path.
- Existing CLI transcribe flow continues to work.
- Runtime path is compatible with Tauri packaging constraints.
- Whisper implementation remains unchanged.
