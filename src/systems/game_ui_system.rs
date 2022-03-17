use bevy::prelude::{Local, Res, ResMut};
use bevy_egui::{egui, EguiContext};
use rose_game_common::messages::client::ClientMessage;

use crate::resources::GameConnection;

#[derive(Default)]
pub struct GameUiState {
    textbox_text: String,
    textbox_history: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub fn game_ui_system(
    mut egui_context: ResMut<EguiContext>,
    mut ui_state: Local<GameUiState>,
    game_connection: Option<Res<GameConnection>>,
) {
    let mut chatbox_style = (*egui_context.ctx_mut().style()).clone();
    chatbox_style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(
        chatbox_style.visuals.widgets.noninteractive.bg_fill.r(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.g(),
        chatbox_style.visuals.widgets.noninteractive.bg_fill.b(),
        128,
    );

    egui::Window::new("Chat Box")
        .anchor(egui::Align2::LEFT_BOTTOM, [5.0, -5.0])
        .collapsible(false)
        .title_bar(false)
        .frame(egui::Frame::window(&chatbox_style))
        .show(egui_context.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(250.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for line in ui_state.textbox_history.iter() {
                            ui.label(line);
                        }
                    });

                ui.text_edit_singleline(&mut ui_state.textbox_text);

                if ui.input().key_pressed(egui::Key::Enter) {
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::Chat(ui_state.textbox_text.clone()))
                            .ok();
                        ui_state.textbox_text.clear();
                    }
                }
            });
        });
}