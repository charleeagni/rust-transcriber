use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_bin_path() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop(); // remove test binary name
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("transcriber-cli");
    path
}

#[test]
fn test_missing_input() {
    let bin = get_bin_path();
    // the clap commands are 'transcribe' or 'watch'
    let output = Command::new(bin)
        .arg("transcribe")
        .arg("--output")
        .arg("test.txt")
        .output()
        .expect("Failed to execute process");

    // clap exits with code 2 for missing required arguments
    assert!(!output.status.success());
}

#[test]
fn test_missing_output() {
    let bin = get_bin_path();
    let output = Command::new(bin)
        .arg("transcribe")
        .arg("--input")
        .arg("test.wav")
        .output()
        .expect("Failed to execute process");

    assert!(!output.status.success());
}
