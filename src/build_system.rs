// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2026 Witold Kaminski

use crate::command::{BuildCommands, CommandSpec};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Built-in command profiles. Explicit command overrides are always possible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildSystem {
    Cargo,
    Cmake,
    Just,
    Lake,
    Make,
    Maven,
    Meson,
}

impl BuildSystem {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "cargo" => Some(Self::Cargo),
            "cmake" => Some(Self::Cmake),
            "just" => Some(Self::Just),
            "lake" | "lean" => Some(Self::Lake),
            "make" => Some(Self::Make),
            "maven" => Some(Self::Maven),
            "meson" => Some(Self::Meson),
            _ => None,
        }
    }

    /// Original aiagents precedence: Cargo, Meson, CMake, Maven, Just, Make.
    /// An explicit choice avoids ambiguities (e.g. both Cargo.toml and Justfile).
    pub fn detect(dir: &Path) -> Option<Self> {
        if dir.join("Cargo.toml").is_file() {
            Some(Self::Cargo)
        } else if dir.join("lakefile.lean").is_file() || dir.join("lakefile.toml").is_file() {
            Some(Self::Lake)
        } else if dir.join("meson.build").is_file() {
            Some(Self::Meson)
        } else if dir.join("CMakeLists.txt").is_file() {
            Some(Self::Cmake)
        } else if dir.join("pom.xml").is_file() {
            Some(Self::Maven)
        } else if dir.join("Justfile").is_file() {
            Some(Self::Just)
        } else if dir.join("Makefile").is_file() {
            Some(Self::Make)
        } else {
            None
        }
    }

    /// Return the conventional target directory relative to the project.
    /// Pass this to commands(), which resolves it against the project root.
    pub fn default_target_dir(self, _project: &Path) -> PathBuf {
        let relative = match self {
            Self::Cargo | Self::Just | Self::Maven => "target",
            Self::Lake => ".lake/build",
            Self::Cmake | Self::Make | Self::Meson => "build",
        };
        PathBuf::from(relative)
    }

    /// Marker indicating that configuration has completed.
    pub fn setup_marker(self, project: &Path, target: &Path) -> Option<PathBuf> {
        let target = if target.is_absolute() { target.to_path_buf() } else { project.join(target) };
        match self {
            Self::Cmake => Some(target.join("CMakeCache.txt")),
            Self::Meson => Some(target.join("meson-private").join("coredata.dat")),
            _ => None,
        }
    }

    /// Append user-supplied options to the setup command.
    pub fn commands_with_config_args(self, project: &Path, target: &Path, config_args: &[String]) -> BuildCommands {
        let mut commands = self.commands(project, target);
        if let Some(setup) = &mut commands.setup {
            setup.args.extend_from_slice(config_args);
        }
        commands
    }

    pub fn commands(self, project: &Path, target: &Path) -> BuildCommands {
        let project = project.canonicalize().unwrap_or_else(|_| project.to_path_buf());
        let target = if target.is_absolute() {
            target.to_path_buf()
        } else {
            project.join(target)
        };
        let t = target.to_string_lossy().into_owned();
        let p = project.to_string_lossy().into_owned();
        match self {
            Self::Cargo => BuildCommands {
                build: Some(CommandSpec::new("cargo").args([
                    "build".to_owned(), "--target-dir".to_owned(), t.clone(),
                ])),
                lint: Some(CommandSpec::new("cargo").args([
                    "clippy".to_owned(), "--target-dir".to_owned(), t.clone(),
                ])),
                test: Some(CommandSpec::new("cargo").args([
                    "test".to_owned(), "--target-dir".to_owned(), t,
                ])),
                ..Default::default()
            },
            Self::Cmake => BuildCommands {
                setup: Some(CommandSpec::new("cmake").args([
                    "-S".to_owned(), p, "-B".to_owned(), t.clone(), "-G".to_owned(), "Ninja".to_owned(),
                ])),
                build: Some(CommandSpec::new("cmake").args([
                    "--build".to_owned(), t.clone(),
                ])),
                test: Some(CommandSpec::new("ctest").args([
                    "--test-dir".to_owned(), t, "--output-on-failure".to_owned(),
                ])),
                ..Default::default()
            },
            Self::Meson => BuildCommands {
                setup: Some(CommandSpec::new("meson").args([
                    "setup".to_owned(), t.clone(),
                ])),
                build: Some(CommandSpec::new("meson").args([
                    "compile".to_owned(), "-C".to_owned(), t.clone(),
                ])),
                test: Some(CommandSpec::new("meson").args([
                    "test".to_owned(), "-C".to_owned(), t,
                ])),
                ..Default::default()
            },
            Self::Lake => BuildCommands {
                build: Some(CommandSpec::new("lake").args(["build"])),
                ..Default::default()
            },
            Self::Maven => BuildCommands {
                // Maven's output directory comes from the POM and is not
                // reliably overridden using -DoutputDirectory.
                build: Some(CommandSpec::new("mvn").args(["-DskipTests", "package"])),
                test: Some(CommandSpec::new("mvn").args(["test"])),
                ..Default::default()
            },
            Self::Just => BuildCommands {
                build: Some(CommandSpec::new("just").args(["build"])),
                ..Default::default()
            },
            Self::Make => BuildCommands {
                build: Some(CommandSpec::new("make")),
                ..Default::default()
            },
        }
    }
}
