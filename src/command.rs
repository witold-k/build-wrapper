// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Executable and arguments. Never interpreted by a shell.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
        }
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// None skips an optional stage; build is mandatory for execution.
/// Commands are fully customizable and may come from a JSON configuration file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BuildCommands {
    #[serde(default)]
    pub setup: Option<CommandSpec>,
    #[serde(default)]
    pub build: Option<CommandSpec>,
    #[serde(default)]
    pub lint: Option<CommandSpec>,
    #[serde(default)]
    pub test: Option<CommandSpec>,
}
