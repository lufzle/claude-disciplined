use std::process::ExitCode;

use claude_disciplined::{
    hook::{self, HookOutput},
    state::State,
};

fn main() -> ExitCode {
    // Read stdin
    let input: hook::HookInput = match serde_json::from_reader(std::io::stdin()) {
        Ok(input) => input,
        Err(_) => return ExitCode::SUCCESS, // Malformed input -> allow
    };

    // Read state from .workflow/state.yml
    let cwd = input.cwd.as_deref().map_or_else(
        || std::env::current_dir().unwrap_or_default(),
        std::path::PathBuf::from,
    );
    let state_path = cwd.join(".workflow/state.yml");

    let state: State = match std::fs::read_to_string(&state_path) {
        Ok(content) => match serde_yaml::from_str(&content) {
            Ok(state) => state,
            Err(_) => return ExitCode::SUCCESS, // Malformed state -> allow
        },
        Err(_) => return ExitCode::SUCCESS, // No state -> not workflow-managed
    };

    // Evaluate rules
    match hook::evaluate(&input, &state) {
        None => ExitCode::SUCCESS,
        Some(reason) => {
            let output = HookOutput::deny(reason);
            let json = serde_json::to_string(&output).expect("serialization");
            eprintln!("{json}");
            ExitCode::from(2)
        }
    }
}
