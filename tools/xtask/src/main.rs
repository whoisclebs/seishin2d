use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let Some(task) = std::env::args().nth(1) else {
        eprintln!("usage: cargo run -p xtask -- <check>");
        return ExitCode::FAILURE;
    };

    match task.as_str() {
        "check" => run("cargo", &["test"]),
        _ => {
            eprintln!("unknown xtask command: {task}");
            ExitCode::FAILURE
        }
    }
}

fn run(program: &str, args: &[&str]) -> ExitCode {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("failed to run {program}: {error}");
            ExitCode::FAILURE
        }
    }
}
