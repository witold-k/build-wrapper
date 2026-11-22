// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

mod common;

use common::{assert_build_fails, assert_pass, run, toolchain, Project};
use build_wrapper::{BuildSystem, StageStatus};

#[test]
fn cpp_meson_end_to_end() {
    if !toolchain("Meson/C++", &["meson", "ninja", "c++"]) { return; }
    let project = Project::new("meson");
    let first = run(&project.0, BuildSystem::Meson);
    assert_pass(&first);
    assert_eq!(first.stages[0].status, StageStatus::Succeeded);
    assert_eq!(first.stages[3].status, StageStatus::Succeeded);
    let second = run(&project.0, BuildSystem::Meson);
    assert_pass(&second);
    assert_eq!(second.stages[0].status, StageStatus::Skipped);
    project.use_broken_sources("meson");
    assert_build_fails(&run(&project.0, BuildSystem::Meson));
}

