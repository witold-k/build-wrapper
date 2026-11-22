// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use build_wrapper::{BltRunner, BuildCommands, BuildSystem, CommandSpec, SetupMode, Stage, StageStatus};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct TempProject(PathBuf);
impl TempProject {
    fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("build-wrapper-{}-{n}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempProject {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

#[test]
fn detects_supported_build_systems_and_prioritizes_cargo() {
    for (file, expected) in [
        ("Cargo.toml", BuildSystem::Cargo),
        ("CMakeLists.txt", BuildSystem::Cmake),
        ("Justfile", BuildSystem::Just),
        ("lakefile.lean", BuildSystem::Lake),
        ("lakefile.toml", BuildSystem::Lake),
        ("Makefile", BuildSystem::Make),
        ("pom.xml", BuildSystem::Maven),
        ("meson.build", BuildSystem::Meson),
    ] {
        let project = TempProject::new();
        fs::write(project.0.join(file), "").unwrap();
        assert_eq!(BuildSystem::detect(&project.0), Some(expected));
    }
    let project = TempProject::new();
    fs::write(project.0.join("Justfile"), "").unwrap();
    fs::write(project.0.join("Cargo.toml"), "").unwrap();
    assert_eq!(BuildSystem::detect(&project.0), Some(BuildSystem::Cargo));
}

#[test]
fn profiles_cover_all_original_build_systems() {
    let root = PathBuf::from("/tmp/example");
    for system in [
        BuildSystem::Cargo, BuildSystem::Cmake, BuildSystem::Just, BuildSystem::Lake,
        BuildSystem::Make, BuildSystem::Maven, BuildSystem::Meson,
    ] {
        assert!(system.commands(&root, &root.join("build")).build.is_some());
    }
    let cargo = BuildSystem::Cargo.commands(&root, &root.join("target"));
    assert!(cargo.lint.is_some());
    assert!(cargo.test.is_some());
    let cmake = BuildSystem::Cmake.commands(&root, &root.join("build"));
    assert!(cmake.setup.is_some());
    assert_eq!(cmake.test.unwrap().program, "ctest");
}


#[test]
fn lean_lake_profile_and_aliases() {
    assert_eq!(BuildSystem::parse("lean"), Some(BuildSystem::Lake));
    assert_eq!(BuildSystem::parse("lake"), Some(BuildSystem::Lake));
    let project = TempProject::new();
    fs::write(project.0.join("lakefile.toml"), "name = \"example\"").unwrap();
    assert_eq!(BuildSystem::detect(&project.0), Some(BuildSystem::Lake));
    let system = BuildSystem::Lake;
    assert_eq!(system.default_target_dir(&project.0), PathBuf::from(".lake/build"));
    let commands = system.commands(&project.0, &system.default_target_dir(&project.0));
    assert_eq!(commands.build.unwrap(), CommandSpec::new("lake").args(["build"]));
    assert!(commands.setup.is_none());
    assert!(commands.lint.is_none());
    assert!(commands.test.is_none());
}

#[cfg(unix)]
mod unix_tests {
    use super::*;

    fn sh(source: &str) -> CommandSpec {
        CommandSpec::new("sh").args(["-c", source])
    }

    #[test]
    fn observer_reports_only_executed_stages() {
        let project = TempProject::new();
        let commands = BuildCommands {
            build: Some(sh("exit 1")),
            lint: Some(sh("exit 0")),
            test: Some(sh("exit 0")),
            ..Default::default()
        };
        let mut seen = Vec::new();
        let report = BltRunner::new(&project.0, commands)
            .run_with_observer(|stage| seen.push(stage))
            .unwrap();
        assert_eq!(seen, vec![Stage::Build]);
        assert_eq!(report.stages[2].status, StageStatus::NotRun);
    }

    #[test]
    fn executes_all_stages_in_order_and_captures_output() {
        let project = TempProject::new();
        let commands = BuildCommands {
            setup: Some(sh("printf setup")),
            build: Some(sh("printf build; printf err >&2")),
            lint: Some(sh("printf lint")),
            test: Some(sh("printf test")),
        };
        let report = BltRunner::new(&project.0, commands).run().unwrap();
        assert!(report.succeeded());
        assert_eq!(report.stages.len(), 4);
        assert_eq!(report.stages.iter().map(|s| s.stage).collect::<Vec<_>>(),
            vec![Stage::Setup, Stage::Build, Stage::Lint, Stage::Test]);
        assert!(report.stages.iter().all(|s| s.status == StageStatus::Succeeded));
        assert_eq!(report.stages[1].stdout, "build");
        assert_eq!(report.stages[1].stderr, "err");
        assert_eq!(report.stages[1].exit_code, Some(0));
    }

    #[test]
    fn first_failure_stops_following_stages() {
        let project = TempProject::new();
        let commands = BuildCommands {
            build: Some(sh("printf diagnostic >&2; exit 17")),
            lint: Some(sh("touch should_not_exist")),
            test: Some(sh("touch should_not_exist_either")),
            ..Default::default()
        };
        let report = BltRunner::new(&project.0, commands).run().unwrap();
        assert!(!report.succeeded());
        assert_eq!(report.first_failure().unwrap().stage, Stage::Build);
        assert_eq!(report.stages[1].exit_code, Some(17));
        assert_eq!(report.stages[1].stderr, "diagnostic");
        assert_eq!(report.stages[2].status, StageStatus::NotRun);
        assert_eq!(report.stages[3].status, StageStatus::NotRun);
        assert!(!project.0.join("should_not_exist").exists());
    }

    #[test]
    fn skipped_optional_stages_and_missing_executable_are_distinct() {
        let project = TempProject::new();
        let commands = BuildCommands {
            build: Some(sh("exit 0")),
            ..Default::default()
        };
        let report = BltRunner::new(&project.0, commands).run().unwrap();
        assert!(report.succeeded());
        assert_eq!(report.stages[0].status, StageStatus::Skipped);
        assert_eq!(report.stages[2].status, StageStatus::Skipped);
        assert_eq!(report.stages[3].status, StageStatus::Skipped);

        let commands = BuildCommands {
            build: Some(CommandSpec::new("nonexistent-build-wrapper-test-executable-xyz")),
            ..Default::default()
        };
        let report = BltRunner::new(&project.0, commands).run().unwrap();
        assert_eq!(report.stages[1].status, StageStatus::Failed);
        assert!(report.stages[1].error.is_some());
        assert_eq!(report.stages[1].exit_code, None);
    }

    #[test]
    fn missing_build_command_is_an_error() {
        let project = TempProject::new();
        assert!(BltRunner::new(&project.0, BuildCommands::default()).run().is_err());
    }

    #[test]
    fn setup_failure_and_disabled_setup() {
        let project = TempProject::new();
        let commands = BuildCommands {
            setup: Some(sh("exit 4")),
            build: Some(sh("exit 0")),
            ..Default::default()
        };
        let report = BltRunner::new(&project.0, commands.clone()).run().unwrap();
        assert_eq!(report.first_failure().unwrap().stage, Stage::Setup);
        assert_eq!(report.stages[1].status, StageStatus::NotRun);
        let mut runner = BltRunner::new(&project.0, commands);
        runner.run_setup = false;
        assert!(runner.run().unwrap().succeeded());
    }

    #[test]
    fn cli_uses_same_library_runner_and_returns_machine_readable_results() {
        let project = TempProject::new();
        let config = project.0.join("commands.json");
        let commands = BuildCommands {
            build: Some(sh("printf cli; exit 9")),
            ..Default::default()
        };
        fs::write(&config, serde_json::to_string(&commands).unwrap()).unwrap();

        let output = std::process::Command::new(env!("CARGO_BIN_EXE_build-wrapper"))
            .arg("--project")
            .arg(&project.0)
            .arg("--commands")
            .arg(&config)
            .arg("--json")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report: build_wrapper::RunReport =
            serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report.first_failure().unwrap().stage, Stage::Build);
        assert_eq!(report.first_failure().unwrap().exit_code, Some(9));
        assert_eq!(report.first_failure().unwrap().stdout, "cli");
    }


    #[test]
    fn setup_auto_runs_only_when_marker_is_missing() {
        let project = TempProject::new();
        let marker = project.0.join("build").join("CMakeCache.txt");
        let commands = BuildCommands {
            setup: Some(sh("mkdir -p build; touch build/CMakeCache.txt")),
            build: Some(sh("test -f build/CMakeCache.txt")),
            ..Default::default()
        };
        let mut runner = BltRunner::new(&project.0, commands);
        runner.setup_mode = SetupMode::Auto;
        runner.setup_marker = Some(marker.clone());
        let first = runner.run().unwrap();
        assert!(first.succeeded());
        assert_eq!(first.stages[0].status, StageStatus::Succeeded);
        let second = runner.run().unwrap();
        assert!(second.succeeded());
        assert_eq!(second.stages[0].status, StageStatus::Skipped);
        runner.setup_mode = SetupMode::Always;
        assert_eq!(runner.run().unwrap().stages[0].status, StageStatus::Succeeded);
        runner.setup_mode = SetupMode::Never;
        assert_eq!(runner.run().unwrap().stages[0].status, StageStatus::Skipped);
    }

    #[test]
    fn configuration_markers_and_arguments_are_system_specific() {
        let project = TempProject::new();
        let root = &project.0;
        let cmake_marker = BuildSystem::Cmake.setup_marker(root, &PathBuf::from("build")).unwrap();
        assert_eq!(cmake_marker, root.join("build/CMakeCache.txt"));
        let meson_marker = BuildSystem::Meson.setup_marker(root, &PathBuf::from("build")).unwrap();
        assert_eq!(meson_marker, root.join("build/meson-private/coredata.dat"));
        assert!(BuildSystem::Cargo.setup_marker(root, &PathBuf::from("target")).is_none());
        let options = vec!["-DCMAKE_BUILD_TYPE=Debug".to_owned()];
        let cmake = BuildSystem::Cmake.commands_with_config_args(root, &PathBuf::from("build"), &options);
        assert!(cmake.setup.unwrap().args.contains(&options[0]));
    }


    #[test]
    fn cli_manual_configure_runs_setup_without_build() {
        let project = TempProject::new();
        let config = project.0.join("commands.json");
        let commands = BuildCommands {
            setup: Some(sh("printf configured")),
            build: Some(sh("touch build_must_not_run")),
            ..Default::default()
        };
        fs::write(&config, serde_json::to_string(&commands).unwrap()).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_build-wrapper"))
            .arg("--project").arg(&project.0)
            .arg("--commands").arg(&config)
            .arg("--configure")
            .output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"configured");
        assert!(!project.0.join("build_must_not_run").exists());
    }

    #[test]
    fn cli_configure_rejects_missing_setup_command() {
        let project = TempProject::new();
        let config = project.0.join("commands.json");
        let commands = BuildCommands {
            build: Some(sh("exit 0")),
            ..Default::default()
        };
        fs::write(&config, serde_json::to_string(&commands).unwrap()).unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_build-wrapper"))
            .arg("--project").arg(&project.0)
            .arg("--commands").arg(&config)
            .arg("--configure")
            .output().unwrap();
        assert_eq!(output.status.code(), Some(2));
    }

    #[test]
    fn command_json_round_trip_and_explicit_env() {
        let commands = BuildCommands {
            build: Some(sh("test \"$BW_TEST\" = yes").env("BW_TEST", "yes")),
            ..Default::default()
        };
        let json = serde_json::to_string(&commands).unwrap();
        let parsed: BuildCommands = serde_json::from_str(&json).unwrap();
        let project = TempProject::new();
        assert!(BltRunner::new(&project.0, parsed).run().unwrap().succeeded());
    }
}
