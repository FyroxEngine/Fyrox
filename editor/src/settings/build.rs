use crate::fyrox::core::reflect::prelude::*;
use fyrox::core::type_traits::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Reflect, TypeUuidProvider)]
#[type_uuid(id = "55e7651e-8840-4c81-aa93-3f01348855e6")]
pub struct EnvironmentVariable {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Default, Reflect, TypeUuidProvider)]
#[type_uuid(id = "1a9443df-bf75-42fb-93d3-860a0249168a")]
pub struct BuildProfile {
    pub name: String,
    pub command: String,
    pub run_sub_command: String,
    pub build_sub_command: String,
    pub args: Vec<String>,
    pub environment_variables: Vec<EnvironmentVariable>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, Reflect)]
pub struct BuildSettings {
    #[reflect(hidden)]
    pub selected_profile: usize,
    pub profiles: Vec<BuildProfile>,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            selected_profile: 0,
            profiles: vec![
                BuildProfile {
                    name: "Debug".to_string(),
                    command: "cargo".to_string(),
                    run_sub_command: "run".to_string(),
                    build_sub_command: "build".to_string(),
                    args: vec!["--package".to_string(), "executor".to_string()],
                    environment_variables: vec![],
                },
                BuildProfile {
                    name: "Debug (HR)".to_string(),
                    command: "cargo".to_string(),
                    run_sub_command: "run".to_string(),
                    build_sub_command: "build".to_string(),
                    args: vec!["--package".to_string(), "executor".to_string()],
                    environment_variables: vec![EnvironmentVariable {
                        name: "RUSTFLAGS".to_string(),
                        value: "-C prefer-dynamic=yes".to_string(),
                    }],
                },
                BuildProfile {
                    name: "Release".to_string(),
                    command: "cargo".to_string(),
                    run_sub_command: "run".to_string(),
                    build_sub_command: "build".to_string(),
                    args: vec![
                        "--package".to_string(),
                        "executor".to_string(),
                        "--release".to_string(),
                    ],
                    environment_variables: vec![],
                },
                BuildProfile {
                    name: "Release (HR)".to_string(),
                    command: "cargo".to_string(),
                    run_sub_command: "run".to_string(),
                    build_sub_command: "build".to_string(),
                    args: vec![
                        "--package".to_string(),
                        "executor".to_string(),
                        "--release".to_string(),
                    ],
                    environment_variables: vec![EnvironmentVariable {
                        name: "RUSTFLAGS".to_string(),
                        value: "-C prefer-dynamic=yes".to_string(),
                    }],
                },
            ],
        }
    }
}
