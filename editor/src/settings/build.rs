// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::fyrox::core::reflect::prelude::*;
use fyrox::core::type_traits::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Reflect, TypeUuidProvider)]
#[type_uuid(id = "55e7651e-8840-4c81-aa93-3f01348855e6")]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Reflect, TypeUuidProvider)]
#[type_uuid(id = "67b93136-17fe-4776-b5f0-f4a9ef3d8972")]
pub struct BuildCommand {
    pub command: String,
    pub args: Vec<String>,
    pub environment_variables: Vec<EnvironmentVariable>,
}

impl Display for BuildCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for var in self.environment_variables.iter() {
            write!(f, "{}=\"{}\" ", var.name, var.value)?;
        }

        write!(f, "{}", self.command)?;

        for arg in self.args.iter() {
            write!(f, " {}", arg)?;
        }

        Ok(())
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Reflect, TypeUuidProvider)]
#[type_uuid(id = "1a9443df-bf75-42fb-93d3-860a0249168a")]
pub struct BuildProfile {
    pub name: String,
    #[reflect(description = "A set of commands that will be used to build your game.")]
    pub build_commands: Vec<BuildCommand>,
    #[reflect(description = "A set of commands that will be used to run your game. \
        This set of commands will be executed right after build commands (if the \
        build was successful)")]
    pub run_command: BuildCommand,
}

impl BuildProfile {
    fn add_arg(&mut self, arg: &str) {
        for command in self
            .build_commands
            .iter_mut()
            .chain([&mut self.run_command])
        {
            command.args.push(arg.to_string());
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct BuildSettings {
    #[reflect(hidden)]
    pub selected_profile: usize,
    pub profiles: Vec<BuildProfile>,
}

impl Default for BuildSettings {
    fn default() -> Self {
        let debug = BuildProfile {
            name: "Debug".to_string(),
            build_commands: vec![BuildCommand {
                command: "cargo".to_string(),
                args: vec![
                    "build".to_string(),
                    "--package".to_string(),
                    "executor".to_string(),
                ],
                environment_variables: vec![],
            }],
            run_command: BuildCommand {
                command: "cargo".to_string(),
                args: vec![
                    "run".to_string(),
                    "--package".to_string(),
                    "executor".to_string(),
                ],
                environment_variables: vec![],
            },
        };

        let mut release = debug.clone();
        release.name = "Release".to_string();
        release.add_arg("--release");

        let debug_hot_reloading = BuildProfile {
            name: "Debug (HR)".to_string(),
            build_commands: vec![
                // Build the game plugin DLL first.
                BuildCommand {
                    command: "cargo".to_string(),
                    args: vec![
                        "build".to_string(),
                        "--package".to_string(),
                        "game_dylib".to_string(),
                        "--no-default-features".to_string(),
                        "--features".to_string(),
                        "dylib-engine".to_string(),                  
                        "--profile".to_string(),
                        "dev-hot-reload".to_string(),
                    ],
                    environment_variables: vec![EnvironmentVariable {
                        name: "RUSTFLAGS".to_string(),
                        value: "-C prefer-dynamic=yes".to_string(),
                    }],
                },
                // Build the executor.
                BuildCommand {
                    command: "cargo".to_string(),
                    args: vec![
                        "build".to_string(),
                        "--package".to_string(),
                        "executor".to_string(),  
                        "--no-default-features".to_string(),
                        "--features".to_string(),
                        "dylib".to_string(),                        
                        "--profile".to_string(),
                        "dev-hot-reload".to_string(),
                    ],
                    environment_variables: vec![EnvironmentVariable {
                        name: "RUSTFLAGS".to_string(),
                        value: "-C prefer-dynamic=yes".to_string(),
                    }],
                },
            ],
            run_command:
                // Run only executor, it will load the game plugin DLL.
                BuildCommand {
                    command: "cargo".to_string(),
                    args: vec![
                        "run".to_string(),
                        "--package".to_string(),
                        "executor".to_string(), 
                        "--no-default-features".to_string(),
                        "--features".to_string(),
                        "dylib".to_string(),
                        "--profile".to_string(),
                        "dev-hot-reload".to_string(),
                    ],
                    environment_variables: vec![EnvironmentVariable {
                        name: "RUSTFLAGS".to_string(),
                        value: "-C prefer-dynamic=yes".to_string(),
                    }],
                },

        };

        let mut release_hot_reloading = release.clone();
        release_hot_reloading.name = "Release (HR)".to_string();
        release_hot_reloading.add_arg("--release");

        Self {
            selected_profile: 0,
            profiles: vec![debug, debug_hot_reloading, release, release_hot_reloading],
        }
    }
}
