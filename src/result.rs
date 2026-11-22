// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::command::CommandSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Setup,
    Build,
    Lint,
    Test,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// No command was configured for this stage.
    Skipped,
    /// A prior stage failed, so this stage was not attempted.
    NotRun,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: Stage,
    pub status: StageStatus,
    pub command: Option<CommandSpec>,
    /// None for spawn failures, skipped stages, and signal termination.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u128,
    /// Failure to start or wait for the process, if applicable.
    pub error: Option<String>,
}

impl StageResult {
    pub fn succeeded(&self) -> bool {
        matches!(self.status, StageStatus::Succeeded | StageStatus::Skipped)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunReport {
    pub stages: Vec<StageResult>,
}

impl RunReport {
    pub fn succeeded(&self) -> bool {
        !self.stages.is_empty() && self.stages.iter().all(StageResult::succeeded)
    }

    pub fn first_failure(&self) -> Option<&StageResult> {
        self.stages.iter().find(|s| s.status == StageStatus::Failed)
    }
}
