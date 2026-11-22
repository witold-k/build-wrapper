// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

mod common;

use common::{assert_build_fails, assert_pass, run, toolchain, Project};
use build_wrapper::{BuildSystem, StageStatus};

#[test]
fn cpp_cmake_end_to_end() {
    if !toolchain("CMake/C++", &["cmake", "ninja", "c++", "ctest"]) { return; }
    let project = Project::new("cmake");
    let first = run(&project.0, BuildSystem::Cmake);
    assert_pass(&first);
    assert_eq!(first.stages[0].status, StageStatus::Succeeded);
    assert_eq!(first.stages[3].status, StageStatus::Succeeded);
    let second = run(&project.0, BuildSystem::Cmake);
    assert_pass(&second);
    assert_eq!(second.stages[0].status, StageStatus::Skipped);
    project.use_broken_sources("cmake");
    assert_build_fails(&run(&project.0, BuildSystem::Cmake));
}

