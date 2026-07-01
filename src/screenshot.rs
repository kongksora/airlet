use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

#[derive(Resource)]
pub struct ScreenshotState {
    pub path: Option<String>,
    pub requested: bool,
    pub frames_before_capture: u32,
    pub exit_after_capture: bool,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        let path = std::env::var("AIRLET_SCREENSHOT").ok();
        Self {
            exit_after_capture: path.is_some(),
            path,
            requested: false,
            frames_before_capture: 180,
        }
    }
}

pub fn auto_screenshot(
    mut commands: Commands,
    mut state: ResMut<ScreenshotState>,
    model_state: Res<crate::model_view::ModelSpawnState>,
) {
    if state.path.is_none() || state.requested {
        return;
    }
    if !model_state.spawned {
        return;
    }
    if state.frames_before_capture > 0 {
        state.frames_before_capture -= 1;
        return;
    }

    let path = state.path.clone().unwrap();
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    state.requested = true;
}

pub fn exit_after_screenshot(state: Res<ScreenshotState>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = &state.path else {
        return;
    };
    if state.exit_after_capture && state.requested && std::path::Path::new(path).exists() {
        exit.write(AppExit::Success);
    }
}
