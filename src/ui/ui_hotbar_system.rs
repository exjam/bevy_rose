use bevy::{
    ecs::query::WorldQuery,
    input::Input,
    prelude::{Assets, EventWriter, KeyCode, Local, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};

use rose_game_common::components::{
    Equipment, Hotbar, HotbarSlot, Inventory, SkillList, HOTBAR_NUM_PAGES, HOTBAR_PAGE_SIZE,
};

use crate::{
    components::{Cooldowns, PlayerCharacter},
    events::PlayerCommandEvent,
    resources::{GameData, UiResources},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem, SkillTooltipType},
        ui_add_item_tooltip, ui_add_skill_tooltip,
        ui_inventory_system::GetItem,
        widgets::{DataBindings, Dialog, Widget},
        DialogInstance, DragAndDropId, DragAndDropSlot, UiStateDragAndDrop,
    },
};

const IID_BG_VERTICAL: i32 = 5;
const IID_BG_HORIZONTAL: i32 = 6;
const IID_BTN_ROTATE: i32 = 10;
const IID_BTN_HORIZONTAL_PREV: i32 = 11;
const IID_BTN_HORIZONTAL_NEXT: i32 = 12;
const IID_BTN_VERTICAL_PREV: i32 = 13;
const IID_BTN_VERTICAL_NEXT: i32 = 14;
const IID_NUMBER: i32 = 20;

pub struct UiStateHotBar {
    dialog_instance: DialogInstance,
    current_page: usize,
    is_vertical: bool,
}

impl Default for UiStateHotBar {
    fn default() -> Self {
        Self {
            dialog_instance: DialogInstance::new("DLGQUICKBAR.XML"),
            current_page: 0,
            is_vertical: false,
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery<'w> {
    hotbar: &'w mut Hotbar,
    cooldowns: &'w Cooldowns,
    equipment: &'w Equipment,
    inventory: &'w Inventory,
    skill_list: &'w SkillList,
}

fn hotbar_drag_accepts(drag_source: &DragAndDropId) -> bool {
    matches!(
        drag_source,
        DragAndDropId::Inventory(_) | DragAndDropId::Skill(_) | DragAndDropId::Hotbar(_, _)
    )
}

fn ui_add_hotbar_slot(
    ui: &mut egui::Ui,
    pos: egui::Pos2,
    hotbar_index: (usize, usize),
    player: &mut PlayerQueryItem,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
    ui_state_dnd: &mut UiStateDragAndDrop,
    use_slot: bool,
    player_command_events: &mut EventWriter<PlayerCommandEvent>,
) {
    let hotbar_slot = player.hotbar.pages[hotbar_index.0][hotbar_index.1].as_ref();
    let mut dropped_item = None;
    let drag_and_drop_slot = match hotbar_slot {
        Some(HotbarSlot::Skill(skill_slot)) => {
            let skill = player.skill_list.get_skill(*skill_slot);

            DragAndDropSlot::with_skill(
                DragAndDropId::Hotbar(hotbar_index.0, hotbar_index.1),
                skill.as_ref(),
                Some(player.cooldowns),
                game_data,
                ui_resources,
                hotbar_drag_accepts,
                &mut ui_state_dnd.dragged_item,
                &mut dropped_item,
                [40.0, 40.0],
            )
        }
        Some(HotbarSlot::Inventory(item_slot)) => {
            let item = (player.equipment, player.inventory).get_item(*item_slot);

            DragAndDropSlot::with_item(
                DragAndDropId::Hotbar(hotbar_index.0, hotbar_index.1),
                item.as_ref(),
                Some(player.cooldowns),
                game_data,
                ui_resources,
                hotbar_drag_accepts,
                &mut ui_state_dnd.dragged_item,
                &mut dropped_item,
                [40.0, 40.0],
            )
        }
        _ => DragAndDropSlot::new(
            DragAndDropId::Hotbar(hotbar_index.0, hotbar_index.1),
            None,
            None,
            false,
            None,
            None,
            hotbar_drag_accepts,
            &mut ui_state_dnd.dragged_item,
            &mut dropped_item,
            [40.0, 40.0],
        ),
    };

    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(pos, egui::vec2(40.0, 40.0)),
            |ui| egui::Widget::ui(drag_and_drop_slot, ui),
        )
        .inner;

    if use_slot || response.double_clicked() {
        player_command_events.send(PlayerCommandEvent::UseHotbar(
            hotbar_index.0,
            hotbar_index.1,
        ));
    }

    if hotbar_slot.is_some() {
        response.on_hover_ui(|ui| match hotbar_slot {
            Some(HotbarSlot::Inventory(item_slot)) => {
                if let Some(item) = (player.equipment, player.inventory).get_item(*item_slot) {
                    ui_add_item_tooltip(ui, game_data, player_tooltip_data, &item);
                }
            }
            Some(HotbarSlot::Skill(skill_slot)) => {
                if let Some(skill) = player.skill_list.get_skill(*skill_slot) {
                    let detailed = ui.input(|input| input.pointer.secondary_down());
                    ui_add_skill_tooltip(
                        ui,
                        if detailed {
                            SkillTooltipType::Detailed
                        } else {
                            SkillTooltipType::Simple
                        },
                        game_data,
                        player_tooltip_data,
                        skill,
                    );
                }
            }
            _ => {}
        });
    }

    match dropped_item {
        Some(DragAndDropId::Hotbar(page, index)) => {
            if page != hotbar_index.0 || index != hotbar_index.1 {
                let slot_a = player.hotbar.pages[hotbar_index.0][hotbar_index.1].take();
                let slot_b = player.hotbar.pages[page][index].take();

                player_command_events.send(PlayerCommandEvent::SetHotbar(page, index, slot_a));
                player_command_events.send(PlayerCommandEvent::SetHotbar(
                    hotbar_index.0,
                    hotbar_index.1,
                    slot_b,
                ));
            }
        }
        Some(DragAndDropId::Inventory(item_slot)) => {
            player_command_events.send(PlayerCommandEvent::SetHotbar(
                hotbar_index.0,
                hotbar_index.1,
                Some(HotbarSlot::Inventory(item_slot)),
            ));
        }
        Some(DragAndDropId::Skill(skill_slot)) => {
            player_command_events.send(PlayerCommandEvent::SetHotbar(
                hotbar_index.0,
                hotbar_index.1,
                Some(HotbarSlot::Skill(skill_slot)),
            ));
        }
        _ => {}
    }
}

pub fn ui_hotbar_system(
    mut egui_context: EguiContexts,
    mut ui_state_hot_bar: Local<UiStateHotBar>,
    mut ui_state_dnd: ResMut<UiStateDragAndDrop>,
    mut query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    keyboard_input: Res<Input<KeyCode>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
) {
    let ui_state_hot_bar = &mut *ui_state_hot_bar;
    let dialog = if let Some(dialog) = ui_state_hot_bar
        .dialog_instance
        .get_mut(&dialog_assets, &ui_resources)
    {
        dialog
    } else {
        return;
    };

    let mut player = if let Ok(player) = query_player.get_single_mut() {
        player
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    let use_hotbar_index = if !egui_context.ctx_mut().wants_keyboard_input() {
        if keyboard_input.just_pressed(KeyCode::F1) {
            Some(0)
        } else if keyboard_input.just_pressed(KeyCode::F2) {
            Some(1)
        } else if keyboard_input.just_pressed(KeyCode::F3) {
            Some(2)
        } else if keyboard_input.just_pressed(KeyCode::F4) {
            Some(3)
        } else if keyboard_input.just_pressed(KeyCode::F5) {
            Some(4)
        } else if keyboard_input.just_pressed(KeyCode::F6) {
            Some(5)
        } else if keyboard_input.just_pressed(KeyCode::F7) {
            Some(6)
        } else if keyboard_input.just_pressed(KeyCode::F8) {
            Some(7)
        } else {
            None
        }
    } else {
        None
    };

    let mut response_rotate_button = None;
    let mut response_hprev_button = None;
    let mut response_hnext_button = None;
    let mut response_vprev_button = None;
    let mut response_vnext_button = None;
    let is_vertical = ui_state_hot_bar.is_vertical;

    let screen_size = egui_context
        .ctx_mut()
        .input(|input| input.screen_rect().size());
    let default_position = egui::pos2(
        screen_size.x / 2.0 - dialog.width / 2.0,
        screen_size.y - dialog.height,
    );

    egui::Window::new("Hot Bar")
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .default_pos(default_position)
        .show(egui_context.ctx_mut(), |ui| {
            dialog.draw(
                ui,
                DataBindings {
                    visible: &mut [
                        (IID_BG_HORIZONTAL, !is_vertical),
                        (IID_BTN_HORIZONTAL_PREV, !is_vertical),
                        (IID_BTN_HORIZONTAL_NEXT, !is_vertical),
                        (IID_BG_VERTICAL, is_vertical),
                        (IID_BTN_VERTICAL_PREV, is_vertical),
                        (IID_BTN_VERTICAL_NEXT, is_vertical),
                    ],
                    response: &mut [
                        (IID_BTN_ROTATE, &mut response_rotate_button),
                        (IID_BTN_HORIZONTAL_PREV, &mut response_hprev_button),
                        (IID_BTN_HORIZONTAL_NEXT, &mut response_hnext_button),
                        (IID_BTN_VERTICAL_PREV, &mut response_vprev_button),
                        (IID_BTN_VERTICAL_NEXT, &mut response_vnext_button),
                    ],
                    ..Default::default()
                },
                |ui, _bindings| {
                    let current_page = ui_state_hot_bar.current_page;

                    for i in 0..HOTBAR_PAGE_SIZE {
                        let hotbar_index = (current_page, i);
                        let pos = if ui_state_hot_bar.is_vertical {
                            egui::vec2(2.0, 39.0 + (41.0) * i as f32 + (2 * i / 8) as f32 * 10.0)
                        } else {
                            egui::vec2(39.0 + (41.0) * i as f32 + (2 * i / 8) as f32 * 9.0, 20.0)
                        };
                        ui_add_hotbar_slot(
                            ui,
                            ui.min_rect().min + pos,
                            hotbar_index,
                            &mut player,
                            player_tooltip_data.as_ref(),
                            &game_data,
                            &ui_resources,
                            &mut ui_state_dnd,
                            use_hotbar_index.map_or(false, |use_index| use_index == i),
                            &mut player_command_events,
                        );
                    }
                },
            );
        });

    let previous_page = ui_state_hot_bar.current_page;

    if response_hnext_button.map_or(false, |r| r.clicked())
        || response_vnext_button.map_or(false, |r| r.clicked())
    {
        ui_state_hot_bar.current_page = (ui_state_hot_bar.current_page + 1) % HOTBAR_NUM_PAGES;
    }

    if response_hprev_button.map_or(false, |r| r.clicked())
        || response_vprev_button.map_or(false, |r| r.clicked())
    {
        if ui_state_hot_bar.current_page == 0 {
            ui_state_hot_bar.current_page = HOTBAR_NUM_PAGES - 1;
        } else {
            ui_state_hot_bar.current_page -= 1;
        }
    }

    if response_rotate_button.map_or(false, |r| r.clicked()) {
        ui_state_hot_bar.is_vertical = !ui_state_hot_bar.is_vertical;

        if let Some(Widget::Button(button)) = dialog.get_widget_mut(IID_BTN_ROTATE) {
            if ui_state_hot_bar.is_vertical {
                button.x = 17.0;
                button.y = 377.0;
            } else {
                button.x = 377.0;
                button.y = 27.0;
            }
        }

        if let Some(Widget::Image(sprite)) = dialog.get_widget_mut(IID_NUMBER) {
            if ui_state_hot_bar.is_vertical {
                sprite.x = 21.0;
                sprite.y = 20.0;
            } else {
                sprite.x = 19.0;
                sprite.y = 24.0;
            }
        }
    }

    if ui_state_hot_bar.current_page != previous_page {
        if let Some(Widget::Image(sprite)) = dialog.get_widget_mut(IID_NUMBER) {
            sprite.sprite = match ui_state_hot_bar.current_page {
                0 => ui_resources.get_sprite(0, "UI21_NUMBER_1"),
                1 => ui_resources.get_sprite(0, "UI21_NUMBER_2"),
                2 => ui_resources.get_sprite(0, "UI21_NUMBER_3"),
                3 => ui_resources.get_sprite(0, "UI21_NUMBER_4"),
                _ => None,
            };
        }
    }
}
