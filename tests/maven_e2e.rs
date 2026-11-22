// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

mod common;

use common::{assert_build_fails, assert_pass, run, toolchain, Project};
use build_wrapper::{BuildSystem, StageStatus};

#[test]
fn java_maven_end_to_end() {
    if !toolchain("Maven/Java", &["mvn", "java", "javac"]) { return; }
    let project = Project::new("maven");
    let first = run(&project.0, BuildSystem::Maven);
    assert_pass(&first);
    assert_eq!(first.stages[3].status, StageStatus::Succeeded);
    assert!(project.0.join("target/classes/example/Main.class").is_file());
    project.use_broken_sources("maven");
    assert_build_fails(&run(&project.0, BuildSystem::Maven));
}

