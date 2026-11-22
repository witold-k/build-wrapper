// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

mod common;

use common::{assert_build_fails, assert_pass, run, toolchain, Project};
use build_wrapper::{BuildSystem, StageStatus};

#[test]
fn lean_lake_end_to_end() {
    if !toolchain("Lean/Lake", &["lake", "lean"]) { return; }
    let project = Project::new("lake");
    let first = run(&project.0, BuildSystem::Lake);
    assert_pass(&first);
    assert_eq!(first.stages[0].status, StageStatus::Skipped);
    assert_eq!(first.stages[3].status, StageStatus::Skipped);
    project.use_broken_sources("lake");
    assert_build_fails(&run(&project.0, BuildSystem::Lake));
}

