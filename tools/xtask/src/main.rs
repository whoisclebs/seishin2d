use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(task) = args.first() else {
        eprintln!("usage: cargo run -p xtask -- <check|web-build|web-serve>");
        return ExitCode::FAILURE;
    };

    match task.as_str() {
        "check" => run("cargo", &["test"]),
        "web-build" => web_build(&args[1..]),
        "web-serve" => web_serve(&args[1..]),
        _ => {
            eprintln!("unknown xtask command: {task}");
            ExitCode::FAILURE
        }
    }
}

fn web_build(args: &[String]) -> ExitCode {
    let Some(example) = parse_example(args) else {
        eprintln!("usage: cargo run -p xtask -- web-build --example <name> [--release]");
        return ExitCode::FAILURE;
    };
    let release = args.iter().any(|arg| arg == "--release");

    match build_web_example(&example, release) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("web build failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn web_serve(args: &[String]) -> ExitCode {
    let Some(example) = parse_example(args) else {
        eprintln!("usage: cargo run -p xtask -- web-serve --example <name> [--release]");
        return ExitCode::FAILURE;
    };
    let release = args.iter().any(|arg| arg == "--release");

    if let Err(error) = build_web_example(&example, release) {
        eprintln!("web build failed: {error}");
        return ExitCode::FAILURE;
    }

    let output_dir = PathBuf::from("target").join("web").join(&example);
    println!("serving {} at http://127.0.0.1:8000", output_dir.display());
    run_in_dir("python", &["-m", "http.server", "8000"], &output_dir)
}

fn parse_example(args: &[String]) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == "--example")
        .map(|pair| pair[1].clone())
}

fn build_web_example(example: &str, release: bool) -> Result<(), String> {
    let example_dir = PathBuf::from("examples").join(example);
    if !example_dir.join("Cargo.toml").is_file() {
        return Err(format!("example '{}' not found", example));
    }

    let package = example_package_name(&example_dir)?;
    let profile = if release { "release" } else { "debug" };
    let mut cargo_args = vec![
        "build".to_string(),
        "--target".to_string(),
        "wasm32-unknown-unknown".to_string(),
        "-p".to_string(),
        package.clone(),
    ];
    if release {
        cargo_args.push("--release".to_string());
    }
    run_checked("cargo", &cargo_args)?;

    let output_dir = PathBuf::from("target").join("web").join(example);
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

    let wasm_path = PathBuf::from("target")
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{}.wasm", package.replace('-', "_")));
    let out_name = package.replace('-', "_");
    run_checked(
        "wasm-bindgen",
        &[
            wasm_path.to_string_lossy().into_owned(),
            "--target".to_string(),
            "web".to_string(),
            "--out-dir".to_string(),
            output_dir.to_string_lossy().into_owned(),
            "--out-name".to_string(),
            out_name.clone(),
        ],
    )
    .map_err(|error| format!("{error}. Install with `cargo install wasm-bindgen-cli`"))?;

    copy_if_exists(&example_dir.join("assets"), &output_dir.join("assets"))?;
    copy_if_exists(
        &example_dir.join("resources"),
        &output_dir.join("resources"),
    )?;
    fs::copy(
        example_dir.join("Seishin.toml"),
        output_dir.join("Seishin.toml"),
    )
    .map_err(|error| error.to_string())?;
    fs::write(output_dir.join("index.html"), web_index_html(&out_name))
        .map_err(|error| error.to_string())?;

    println!("web build written to {}", output_dir.display());
    Ok(())
}

fn example_package_name(example_dir: &Path) -> Result<String, String> {
    let manifest =
        fs::read_to_string(example_dir.join("Cargo.toml")).map_err(|error| error.to_string())?;
    let manifest = manifest
        .parse::<toml::Value>()
        .map_err(|error| format!("invalid example Cargo.toml: {error}"))?;

    manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "package name not found in example Cargo.toml".to_string())
}

fn copy_if_exists(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Ok(());
    }

    copy_dir(from, to)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(from).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn web_index_html(out_name: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Seishin2D Web</title>
    <style>
      html, body {{ margin: 0; min-height: 100%; background: #111; }}
      canvas {{ display: block; margin: auto; outline: none; }}
    </style>
  </head>
  <body>
    <script type="module">
      import init from './{out_name}.js';
      init();
    </script>
  </body>
</html>
"#
    )
}

fn run_checked(program: &str, args: &[String]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with status {status}"))
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

fn run_in_dir(program: &str, args: &[&str], dir: &Path) -> ExitCode {
    match Command::new(program).args(args).current_dir(dir).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("failed to run {program}: {error}");
            ExitCode::FAILURE
        }
    }
}
