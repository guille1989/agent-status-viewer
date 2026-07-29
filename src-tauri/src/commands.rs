use tauri::State;

use crate::state::{AgentViewState, SharedState};

#[tauri::command]
pub fn get_status(state: State<SharedState>) -> AgentViewState {
    state.0.lock().unwrap().clone()
}
