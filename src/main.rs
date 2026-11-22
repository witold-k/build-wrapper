// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use build_wrapper::{BltRunner, BuildCommands, BuildSystem, SetupMode, StageStatus};
use std::env;
use std::fs;
use std::path::PathBuf;

fn usage() -> &'static str {
    "Usage: build-wrapper [--project DIR] [--system cargo|cmake|just|lake|lean|make|maven|meson] \
     [--target DIR] [--commands FILE.json] [--setup auto|always|never] [--configure-arg ARG] [--no-setup] [--configure -- ARGS...] [--json]\n\n\
     --commands: JSON-encoded BuildCommands, replacing the detected profile.\n\
     Commands are executed directly, never through a shell.\n\
     Exit status: 0 successful, 1 validation failed, 2 configuration error.\n"
}

fn take_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("missing value for {flag}"))
}

fn run() -> Result<bool, String> {
    let mut project = PathBuf::from(".");
    let mut system_name = None;
    let mut target = None;
    let mut command_file = None;
    let mut setup_mode = SetupMode::Auto;
    let mut config_args = Vec::new();
    let mut configure_only = false;
    let mut json = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => project = PathBuf::from(take_value(&mut args, &arg)?),
            "--system" => system_name = Some(take_value(&mut args, &arg)?),
            "--target" => target = Some(PathBuf::from(take_value(&mut args, &arg)?)),
            "--commands" => command_file = Some(PathBuf::from(take_value(&mut args, &arg)?)),
            "--no-setup" => setup_mode = SetupMode::Never,
            "--setup" => setup_mode = match take_value(&mut args, &arg)?.as_str() {
                "auto" => SetupMode::Auto,
                "always" => SetupMode::Always,
                "never" => SetupMode::Never,
                value => return Err(format!("invalid setup mode: {value}")),
            },
            "--configure-arg" => config_args.push(take_value(&mut args, &arg)?),
            "--configure" => configure_only = true,
            "--" if configure_only => {
                config_args.extend(args);
                break;
            },
            "--json" => json = true,
            "--help" | "-h" => {
                print!("{}", usage());
                return Ok(true);
            }
            _ => return Err(format!("unknown argument: {arg}\n{}", usage())),
        }
    }
    project = project.canonicalize()
        .map_err(|e| format!("invalid project directory {}: {e}", project.display()))?;
    if configure_only && setup_mode == SetupMode::Never {
        return Err("--configure conflicts with --setup never".to_owned());
    }
    let mut setup_marker = None;
    let commands: BuildCommands = if let Some(config_path) = command_file {
        if !config_args.is_empty() {
            return Err("--configure-arg cannot be combined with --commands".to_owned());
        }
        let contents = fs::read_to_string(&config_path)
            .map_err(|e| format!("cannot read {}: {e}", config_path.display()))?;
        serde_json::from_str(&contents)
            .map_err(|e| format!("invalid commands JSON {}: {e}", config_path.display()))?
    } else {
        let system = match system_name {
            Some(name) => BuildSystem::parse(&name)
                .ok_or_else(|| format!("unsupported build system: {name}"))?,
            None => BuildSystem::detect(&project)
                .ok_or("cannot detect build system; use --system or --commands")?,
        };
        let target = target.unwrap_or_else(|| system.default_target_dir(&project));
        setup_marker = system.setup_marker(&project, &target);
        if setup_marker.is_none() && !config_args.is_empty() {
            return Err("--configure-arg is only supported for CMake and Meson".to_owned());
        }
        system.commands_with_config_args(&project, &target, &config_args)
    };
    if configure_only {
        let setup = commands.setup.as_ref()
            .ok_or("--configure requires a configured setup command (CMake or Meson)")?;
        let output = build_wrapper::process::execute(setup, &project)
            .map_err(|err| format!("cannot launch configure command: {err}"))?;
        if json {
            let report = serde_json::json!({
                "command": setup,
                "exit_code": output.status.code(),
                "success": output.status.success(),
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            });
            println!("{}", serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?);
        } else {
            use std::io::Write;
            std::io::stdout().write_all(&output.stdout).map_err(|err| err.to_string())?;
            std::io::stderr().write_all(&output.stderr).map_err(|err| err.to_string())?;
        }
        return Ok(output.status.success());
    }
    let mut runner = BltRunner::new(&project, commands);
    runner.setup_mode = setup_mode;
    runner.setup_marker = setup_marker;
    let report = runner.run().map_err(|e| e.to_string())?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
    } else {
        for result in &report.stages {
            println!("## [BLT] {:?}: {:?}", result.stage, result.status);
            if !result.stdout.is_empty() {
                print!("{}", result.stdout);
                if !result.stdout.ends_with('\n') { println!(); }
            }
            if !result.stderr.is_empty() {
                eprint!("{}", result.stderr);
                if !result.stderr.ends_with('\n') { eprintln!(); }
            }
            if let Some(err) = &result.error {
                eprintln!("process error: {err}");
            }
            if result.status == StageStatus::Failed {
                eprintln!("failed: {:?} (exit {:?})", result.stage, result.exit_code);
            }
        }
    }
    Ok(report.succeeded())
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}
