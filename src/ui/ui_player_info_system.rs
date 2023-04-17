use bevy::{
    ecs::query::WorldQuery,
    prelude::{Assets, Entity, EventWriter, Query, Res, ResMut, With},
};
use bevy_egui::{egui, EguiContexts};
use rose_data::{AmmoIndex, EquipmentIndex, Item, ItemClass};
use rose_game_common::components::{
    AbilityValues, CharacterInfo, Equipment, ExperiencePoints, HealthPoints, Level, ManaPoints,
};

use crate::{
    components::PlayerCharacter,
    resources::{GameData, SelectedTarget, UiResources},
    ui::{
        tooltips::{PlayerTooltipQuery, PlayerTooltipQueryItem},
        ui_add_item_tooltip,
        widgets::{DataBindings, Dialog, DrawText},
        DragAndDropId, DragAndDropSlot, UiSoundEvent, UiStateWindows,
    },
};

const IID_GAUGE_HP: i32 = 6;
const IID_GAUGE_MP: i32 = 7;
const IID_GAUGE_EXP: i32 = 8;

// const IID_BTN_SELFTARGET: i32 = 10;
const IID_BTN_MENU: i32 = 11;
// const IID_BTN_DIALOG2ICON: i32 = 12;
// const IID_BTN_SCREENSHOT: i32 = 13;

#[derive(WorldQuery)]
pub struct PlayerQuery<'w> {
    entity: Entity,
    ability_values: &'w AbilityValues,
    character_info: &'w CharacterInfo,
    level: &'w Level,
    health_points: &'w HealthPoints,
    mana_points: &'w ManaPoints,
    experience_points: &'w ExperiencePoints,
    equipment: &'w Equipment,
}

fn add_equipped_weapon_slot(
    ui: &mut egui::Ui,
    pos: egui::Pos2,
    player: &PlayerQueryItem,
    player_tooltip_data: Option<&PlayerTooltipQueryItem>,
    game_data: &GameData,
    ui_resources: &UiResources,
) {
    let mut item = None;

    if let Some(weapon_item) = player.equipment.get_equipment_item(EquipmentIndex::Weapon) {
        item = Some(Item::Equipment(weapon_item.clone()));

        if let Some(weapon_item_data) = game_data
            .items
            .get_weapon_item(weapon_item.item.item_number)
        {
            let ammo_index = match weapon_item_data.item_data.class {
                ItemClass::Bow | ItemClass::Crossbow => Some(AmmoIndex::Arrow),
                ItemClass::Gun | ItemClass::DualGuns => Some(AmmoIndex::Bullet),
                ItemClass::Launcher => Some(AmmoIndex::Throw),
                _ => None,
            };

            if let Some(ammo_index) = ammo_index {
                if let Some(ammo) = player.equipment.get_ammo_item(ammo_index) {
                    item = Some(Item::Stackable(ammo.clone()));
                }
            }
        }
    }

    let mut dragged_item = None;
    let mut dropped_item = None;
    let response = ui
        .allocate_ui_at_rect(
            egui::Rect::from_min_size(ui.min_rect().min + pos.to_vec2(), egui::vec2(40.0, 40.0)),
            |ui| {
                egui::Widget::ui(
                    DragAndDropSlot::with_item(
                        DragAndDropId::NotDraggable,
                        item.as_ref(),
                        None,
                        game_data,
                        ui_resources,
                        |_| false,
                        &mut dragged_item,
                        &mut dropped_item,
                        [40.0, 40.0],
                    ),
                    ui,
                )
            },
        )
        .inner;

    if let Some(item) = item {
        response.on_hover_ui(|ui| {
            ui_add_item_tooltip(ui, game_data, player_tooltip_data, &item);
        });
    }
}

pub fn ui_player_info_system(
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_sound_events: EventWriter<UiSoundEvent>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_player_tooltip: Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
    dialog_assets: Res<Assets<Dialog>>,
    mut selected_target: ResMut<SelectedTarget>,
) {
    let dialog = if let Some(dialog) = dialog_assets.get(&ui_resources.dialog_player_info) {
        dialog
    } else {
        return;
    };

    let player = if let Ok(player) = query_player.get_single() {
        player
    } else {
        return;
    };
    let player_tooltip_data = query_player_tooltip.get_single().ok();

    let mut response_menu_button = None;

    let response = egui::Window::new("Player Info")
        .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
        .frame(egui::Frame::none())
        .title_bar(false)
        .resizable(false)
        .default_width(dialog.width)
        .default_height(dialog.height)
        .show(egui_context.ctx_mut(), |ui| {
            let hp = player.health_points.hp as f32 / player.ability_values.get_max_health() as f32;
            let mp = player.mana_points.mp as f32 / player.ability_values.get_max_mana() as f32;
            let need_xp = game_data
                .ability_value_calculator
                .calculate_levelup_require_xp(player.level.level);
            let xp = player.experience_points.xp as f32 / need_xp as f32;

            dialog.draw(
                ui,
                DataBindings {
                    sound_events: Some(&mut ui_sound_events),
                    response: &mut [(IID_BTN_MENU, &mut response_menu_button)],
                    gauge: &mut [
                        (
                            IID_GAUGE_HP,
                            &hp,
                            &format!(
                                "{}/{}",
                                player.health_points.hp,
                                player.ability_values.get_max_health()
                            ),
                        ),
                        (
                            IID_GAUGE_MP,
                            &mp,
                            &format!(
                                "{}/{}",
                                player.mana_points.mp,
                                player.ability_values.get_max_mana()
                            ),
                        ),
                        (IID_GAUGE_EXP, &xp, &format!("{:.2}%", xp * 100.0)),
                    ],
                    ..Default::default()
                },
                |ui, _| {
                    ui.add_label_in(
                        egui::Rect::from_min_max(egui::pos2(15.0, 8.0), egui::pos2(150.0, 25.0)),
                        egui::RichText::new(&player.character_info.name)
                            .color(egui::Color32::from_rgb(0, 255, 42))
                            .font(egui::FontId::new(
                                14.0,
                                egui::FontFamily::Name("Ubuntu-M".into()),
                            )),
                    );

                    ui.add_label_in(
                        egui::Rect::from_min_max(egui::pos2(180.0, 8.0), egui::pos2(230.0, 25.0)),
                        egui::RichText::new(format!("{}", player.level.level))
                            .color(egui::Color32::YELLOW)
                            .font(egui::FontId::new(
                                14.0,
                                egui::FontFamily::Name("Ubuntu-M".into()),
                            )),
                    );

                    add_equipped_weapon_slot(
                        ui,
                        egui::pos2(186.0, 36.0),
                        &player,
                        player_tooltip_data.as_ref(),
                        &game_data,
                        &ui_resources,
                    );
                },
            )
        });

    if let Some(response) = response {
        if response.response.clicked() {
            selected_target.selected = Some(player.entity);
        }
    }

    if response_menu_button.map_or(false, |r| r.clicked()) {
        ui_state_windows.menu_open = !ui_state_windows.menu_open;
    }
}
