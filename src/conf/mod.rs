//! Utilities for dealing with game configuration.

use crate::{utils::log::Log, *};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A centralized place to store game engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    graphics: renderer::QualitySettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            graphics: renderer::QualitySettings::default(),
        }
    }
}

/// Load a game configuration from a file.
pub fn load_from_file<'a, T: Default + Deserialize<'a>>(filename: &str) -> T {
    let contents = std::fs::read_to_string(Path::new(filename));
    if let Ok(Ok(settings)) = contents
        .as_ref()
        .and_then(|f| serde::export::Ok(serde_json::from_str(f)))
    {
        Log::writeln("Successfully loaded settings".to_string());
        settings
    } else {
        // Unable to read settings file, so fall back to defaults
        Log::writeln(format!(
            "Could not read settings file {} (missing or corrupted?), falling back to defaults",
            filename
        ));
        T::default()
    }
}

pub fn write_to_file<T: Default + Serialize>(settings: &T, filename: &str) {
    if let Err(error) = serde_json::to_string(settings)
        .and_then(|data| serde::export::Ok(std::fs::write(std::path::Path::new(filename), data)))
    {
        Log::writeln(format!("Error saving settings: {}", error))
    } else {
        Log::writeln(format!("Succesfully saved settings to {}", filename));
    }
}
