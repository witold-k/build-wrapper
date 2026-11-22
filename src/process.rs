// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::command::CommandSpec;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

/// Runs a process without a shell; output() drains both pipes concurrently.
/// stdout and stderr remain separate, since their total interleaving is not
/// recoverable from independent pipes without explicit stream multiplexing.
pub fn execute(command: &CommandSpec, cwd: &Path) -> io::Result<Output> {
    if command.program.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty executable name",
        ));
    }

    Command::new(&command.program)
        .args(&command.args)
        .envs(&command.env)
        .current_dir(cwd)
        .output()
}
