// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::command::{BuildCommands, CommandSpec};
use crate::process;
use crate::result::{RunReport, Stage, StageResult, StageStatus};
use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug)]
pub enum BuildError {
    MissingBuildCommand,
    InvalidProjectDirectory(PathBuf),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBuildCommand => write!(f, "no build command configured"),
            Self::InvalidProjectDirectory(path) => {
                write!(f, "project directory does not exist: {}", path.display())
            }
        }
    }
}
impl Error for BuildError {}

/// Policy for running the configuration/setup stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetupMode {
    /// Configure only when the build directory has no recognized configuration.
    Auto,
    /// Always invoke setup, including for already configured build directories.
    Always,
    /// Never invoke setup.
    Never,
}

/// One deterministic validation pass. Does not repair, retry, or call an LLM.
#[derive(Clone, Debug)]
pub struct BltRunner {
    pub project_dir: PathBuf,
    pub commands: BuildCommands,
    pub run_setup: bool,
    pub setup_mode: SetupMode,
    pub setup_marker: Option<PathBuf>,
}

impl BltRunner {
    pub fn new(project_dir: impl Into<PathBuf>, commands: BuildCommands) -> Self {
        Self {
            project_dir: project_dir.into(),
            commands,
            run_setup: true,
            setup_mode: SetupMode::Always,
            setup_marker: None,
        }
    }

    /// Fail fast in setup → build → lint → test order.
    ///
    /// Every stage is recorded: missing optional commands are Skipped;
    /// stages after a failure are NotRun. A command launch failure is a
    /// Failed stage with error text rather than a panic.
    pub fn run(&self) -> Result<RunReport, BuildError> {
        self.run_with_observer(|_| {})
    }

    /// Invokes the observer immediately before each configured stage executes.
    /// This is useful for CLI progress messages and external orchestration.
    pub fn run_with_observer(
        &self,
        mut before_stage: impl FnMut(Stage),
    ) -> Result<RunReport, BuildError> {
        if self.commands.build.is_none() {
            return Err(BuildError::MissingBuildCommand);
        }
        if !self.project_dir.is_dir() {
            return Err(BuildError::InvalidProjectDirectory(self.project_dir.clone()));
        }
        let setup_enabled = self.run_setup && match self.setup_mode {
            SetupMode::Always => true,
            SetupMode::Never => false,
            SetupMode::Auto => self.setup_marker.as_ref().is_none_or(|marker| {
                let marker = if marker.is_absolute() {
                    marker.clone()
                } else {
                    self.project_dir.join(marker)
                };
                !marker.is_file()
            }),
        };
        let mut results = Vec::with_capacity(4);
        let stages = [
            (
                Stage::Setup,
                if setup_enabled { self.commands.setup.as_ref() } else { None },
            ),
            (Stage::Build, self.commands.build.as_ref()),
            (Stage::Lint, self.commands.lint.as_ref()),
            (Stage::Test, self.commands.test.as_ref()),
        ];
        let mut failed = false;
        for (stage, command) in stages {
            let result = if failed {
                StageResult {
                    stage,
                    status: StageStatus::NotRun,
                    command: command.cloned(),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                    error: None,
                }
            } else if let Some(command) = command {
                before_stage(stage);
                self.execute_stage(stage, command)
            } else {
                StageResult {
                    stage,
                    status: StageStatus::Skipped,
                    command: None,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                    error: None,
                }
            };
            failed |= result.status == StageStatus::Failed;
            results.push(result);
        }
        Ok(RunReport { stages: results })
    }

    fn execute_stage(&self, stage: Stage, cmd: &CommandSpec) -> StageResult {
        let start = Instant::now();
        let output = process::execute(cmd, &self.project_dir);
        let duration_ms = start.elapsed().as_millis();
        match output {
            Ok(output) => StageResult {
                stage,
                status: if output.status.success() {
                    StageStatus::Succeeded
                } else {
                    StageStatus::Failed
                },
                command: Some(cmd.clone()),
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                duration_ms,
                error: None,
            },
            Err(err) => StageResult {
                stage,
                status: StageStatus::Failed,
                command: Some(cmd.clone()),
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms,
                error: Some(err.to_string()),
            },
        }
    }
}
