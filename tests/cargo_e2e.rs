// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

mod common;

use common::{assert_build_fails, assert_pass, run, toolchain, Project};
use build_wrapper::{BuildSystem, StageStatus};
use std::process::Command;

#[test]
fn rust_cargo_end_to_end() {
    if !toolchain("Cargo/Rust", &["cargo", "rustc"]) { return; }
    if !Command::new("cargo").args(["clippy", "--version"]).output().is_ok_and(|o| o.status.success()) {
        if std::env::var_os("BUILD_WRAPPER_E2E_REQUIRED").is_some() {
            panic!("Cargo/Rust: cargo-clippy is required");
        }
        eprintln!("SKIP Cargo/Rust: cargo-clippy is unavailable");
        return;
    }
    let project = Project::new("cargo");
    let first = run(&project.0, BuildSystem::Cargo);
    assert_pass(&first);
    assert_eq!(first.stages[2].status, StageStatus::Succeeded);
    assert_eq!(first.stages[3].status, StageStatus::Succeeded);
    project.use_broken_sources("cargo");
    assert_build_fails(&run(&project.0, BuildSystem::Cargo));
}
