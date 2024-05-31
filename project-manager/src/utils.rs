use std::process::Command;

pub fn is_tool_installed(name: &str) -> bool {
    let Ok(output) = Command::new(name).output() else {
        return false;
    };

    output.status.success()
}

pub fn is_production_ready() -> bool {
    is_tool_installed("rustc") && is_tool_installed("cargo")
}
