// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

//! Deterministic build, lint and test execution.
//!
//! The library does not access LLMs, modify source files, or run a shell.
//! Callers decide whether and how to repair failed builds.

pub mod blt;
pub mod build_system;
pub mod command;
pub mod process;
pub mod result;

pub use blt::{BltRunner, BuildError, SetupMode};
pub use build_system::BuildSystem;
pub use command::{BuildCommands, CommandSpec};
pub use result::{RunReport, Stage, StageResult, StageStatus};
