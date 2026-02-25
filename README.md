# rust-transcriber

A Rust transcriber that converts audio to text.

## Parakeet runtime (Rust-only)

Parakeet transcription now runs fully in Rust.

- No Python runtime is required.
- No `PARAKEET_PYTHON_BIN` setup is required.
- No `parakeet-mlx` Python package setup is required.

### Model input rules

- If `--model` points to an existing local directory, that directory is used.
- Otherwise, `--model` is treated as a Hugging Face model repo ID.
- For Parakeet, the model source must contain ONNX Parakeet files.

### Example command

```bash
cargo run -p transcriber-cli -- transcribe \
  --input "/Users/karthik/Desktop/merge_conflicts/Personal Automation/rust-transcriber/meeting.m4a" \
  --output "/Users/karthik/Desktop/merge_conflicts/Personal Automation/rust-transcriber/output.parakeet.txt" \
  --runtime parakeet \
  --model "istupakov/parakeet-tdt-0.6b-v3-onnx"
```

## Whisper runtime

Whisper runtime behavior is unchanged.
