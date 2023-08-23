use bevy::prelude::{Local, ParamSet, Query, Res, ResMut, State, With};
use bevy_egui::{egui, EguiContexts};
use regex::Regex;

use rose_data::{EquipmentIndex, EquipmentItem, Item, ItemReference, ItemType};
use rose_data_irose::encode_item_type;
use rose_game_common::{components::Equipment, messages::client::ClientMessage};

use crate::{
    components::PlayerCharacter,
    resources::{AppState, GameConnection, GameData, UiResources, UiSpriteSheetType},
    ui::{tooltips::PlayerTooltipQuery, ui_add_item_tooltip, UiStateDebugWindows},
};

pub struct UiStateDebugItemList {
    filter_item_type: ItemType,
    filter_name: String,
    filtered_items: Vec<u16>,
    spawn_as_drop: bool,
    spawn_has_socket: bool,
    spawn_gem: usize,
    spawn_grade: u8,
    spawn_quantity: usize,
}

impl Default for UiStateDebugItemList {
    fn default() -> Self {
        Self {
            filter_item_type: ItemType::Face,
            filter_name: String::new(),
            filtered_items: Vec::default(),
            spawn_as_drop: false,
            spawn_has_socket: false,
            spawn_gem: 0,
            spawn_grade: 0,
            spawn_quantity: 1,
        }
    }
}

pub fn ui_debug_item_list_system(
    mut egui_context: EguiContexts,
    mut ui_state_debug_item_list: Local<UiStateDebugItemList>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut query_set: ParamSet<(
        Query<&mut Equipment>,
        Query<PlayerTooltipQuery, With<PlayerCharacter>>,
    )>,
    app_state: Res<State<AppState>>,
    game_connection: Option<Res<GameConnection>>,
    game_data: Res<GameData>,
    ui_resources: Res<UiResources>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Item List")
        .resizable(true)
        .default_height(300.0)
        .open(&mut ui_state_debug_windows.item_list_open)
        .show(egui_context.ctx_mut(), |ui| {
            let mut filter_changed = false;

            egui::Grid::new("item_list_controls_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Item Name Filter:");
                    if ui
                        .text_edit_singleline(&mut ui_state_debug_item_list.filter_name)
                        .changed()
                    {
                        filter_changed = true;
                    }
                    ui.end_row();

                    if matches!(app_state.get(), AppState::Game) {
                        ui.label("Spawn Quantity:");
                        ui.add(
                            egui::DragValue::new(&mut ui_state_debug_item_list.spawn_quantity)
                                .speed(1)
                                .clamp_range(1..=999usize),
                        );
                        ui.end_row();

                        ui.label("Socket:");
                        ui.add(egui::Checkbox::new(
                            &mut ui_state_debug_item_list.spawn_has_socket,
                            "Has socket",
                        ));
                        ui.end_row();

                        ui.label("Gem:");
                        egui::ComboBox::from_label("Gem")
                            .selected_text(
                                game_data
                                    .items
                                    .get_gem_item(ui_state_debug_item_list.spawn_gem)
                                    .map_or("None", |item_data| item_data.item_data.name),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut ui_state_debug_item_list.spawn_gem,
                                    0,
                                    "None",
                                );

                                for (item_reference, item_data) in game_data
                                    .items
                                    .iter_items(ItemType::Gem)
                                    .filter_map(|item_reference| {
                                        game_data
                                            .items
                                            .get_base_item(item_reference)
                                            .map(|item_data| (item_reference, item_data))
                                    })
                                {
                                    ui.selectable_value(
                                        &mut ui_state_debug_item_list.spawn_gem,
                                        item_reference.item_number,
                                        item_data.name,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Grade:");
                        ui.add(
                            egui::DragValue::new(&mut ui_state_debug_item_list.spawn_grade)
                                .speed(1)
                                .clamp_range(0..=9u8),
                        );
                        ui.end_row();

                        ui.label("Spawn item drop:");
                        ui.add(egui::Checkbox::new(
                            &mut ui_state_debug_item_list.spawn_as_drop,
                            "As item drop",
                        ));
                        ui.end_row();
                    }
                });

            ui.separator();

            let previous_item_list_type = ui_state_debug_item_list.filter_item_type;

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Face,
                    "Face",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Head,
                    "Head",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Body,
                    "Body",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Hands,
                    "Hands",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Feet,
                    "Feet",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Back,
                    "Back",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Weapon,
                    "Weapon",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::SubWeapon,
                    "SubWeapon",
                );
            });

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Jewellery,
                    "Jewellery",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Consumable,
                    "Consumable",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Gem,
                    "Gem",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Material,
                    "Material",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Quest,
                    "Quest",
                );
                ui.selectable_value(
                    &mut ui_state_debug_item_list.filter_item_type,
                    ItemType::Vehicle,
                    "Vehicle",
                );
            });

            if previous_item_list_type != ui_state_debug_item_list.filter_item_type {
                filter_changed = true;
            }

            if ui_state_debug_item_list.filter_name.is_empty()
                && ui_state_debug_item_list.filtered_items.is_empty()
            {
                filter_changed = true;
            }

            if filter_changed {
                let filter_name_re = if !ui_state_debug_item_list.filter_name.is_empty() {
                    Some(
                        Regex::new(&format!(
                            "(?i){}",
                            regex::escape(&ui_state_debug_item_list.filter_name)
                        ))
                        .unwrap(),
                    )
                } else {
                    None
                };

                ui_state_debug_item_list.filtered_items = game_data
                    .items
                    .iter_items(ui_state_debug_item_list.filter_item_type)
                    .filter_map(|item_reference| {
                        game_data
                            .items
                            .get_base_item(item_reference)
                            .map(|item_data| (item_reference, item_data))
                    })
                    .filter_map(|(item_reference, item_data)| {
                        if item_data.name.is_empty()
                            || !filter_name_re
                                .as_ref()
                                .map_or(true, |re| re.is_match(item_data.name))
                        {
                            None
                        } else {
                            Some(item_reference.item_number as u16)
                        }
                    })
                    .collect();
            }

            egui_extras::TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(egui_extras::Column::exact(45.0))
                .column(egui_extras::Column::initial(50.0).at_least(50.0))
                .column(egui_extras::Column::remainder().at_least(80.0))
                .column(egui_extras::Column::initial(60.0).at_least(60.0))
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.heading("Icon");
                    });
                    header.col(|ui| {
                        ui.heading("ID");
                    });
                    header.col(|ui| {
                        ui.heading("Name");
                    });
                    header.col(|ui| {
                        ui.heading("Action");
                    });
                })
                .body(|body| {
                    let equipment_index: Option<EquipmentIndex> =
                        ui_state_debug_item_list.filter_item_type.try_into().ok();

                    let is_equipment_item = ui_state_debug_item_list
                        .filter_item_type
                        .is_equipment_item();

                    body.rows(
                        45.0,
                        ui_state_debug_item_list.filtered_items.len()
                            + usize::from(is_equipment_item),
                        |row_index, mut row| {
                            if is_equipment_item && row_index == 0 {
                                row.col(|_| {});

                                row.col(|ui| {
                                    ui.label("0");
                                });

                                row.col(|ui| {
                                    ui.label("None");
                                });

                                row.col(|ui| {
                                    if matches!(app_state.get(), AppState::ModelViewer)
                                        && ui.button("Equip").clicked()
                                    {
                                        if let Some(equipment_index) = equipment_index {
                                            for mut equipment in query_set.p0().iter_mut() {
                                                equipment.equipped_items[equipment_index] = None;
                                            }
                                        }
                                        if matches!(
                                            ui_state_debug_item_list.filter_item_type,
                                            ItemType::Gem
                                        ) {
                                            for mut equipment in query_set.p0().iter_mut() {
                                                if let Some(weapon) = equipment.equipped_items
                                                    [EquipmentIndex::Weapon]
                                                    .as_mut()
                                                {
                                                    weapon.has_socket = false;
                                                    weapon.gem = 0;
                                                }

                                                if let Some(sub_weapon) = equipment.equipped_items
                                                    [EquipmentIndex::SubWeapon]
                                                    .as_mut()
                                                {
                                                    sub_weapon.has_socket = false;
                                                    sub_weapon.gem = 0;
                                                }
                                            }
                                        }
                                    }
                                });
                            } else if let Some(item_data) = ui_state_debug_item_list
                                .filtered_items
                                .get(row_index - usize::from(is_equipment_item))
                                .and_then(|id| {
                                    game_data.items.get_base_item(ItemReference::new(
                                        ui_state_debug_item_list.filter_item_type,
                                        *id as usize,
                                    ))
                                })
                            {
                                let item_reference = ItemReference::new(
                                    ui_state_debug_item_list.filter_item_type,
                                    *ui_state_debug_item_list
                                        .filtered_items
                                        .get(row_index - usize::from(is_equipment_item))
                                        .unwrap() as usize,
                                );

                                row.col(|ui| {
                                    if let Some(sprite) = ui_resources.get_sprite_by_index(
                                        UiSpriteSheetType::Item,
                                        item_data.icon_index as usize,
                                    ) {
                                        ui.add(
                                            egui::Image::new(sprite.texture_id, [40.0, 40.0])
                                                .uv(sprite.uv),
                                        )
                                        .on_hover_ui(
                                            |ui| {
                                                if let Some(item) =
                                                    Item::from_item_data(item_data, 1)
                                                {
                                                    let query = query_set.p1();
                                                    let player_tooltip_data =
                                                        query.get_single().ok();
                                                    ui_add_item_tooltip(
                                                        ui,
                                                        &game_data,
                                                        player_tooltip_data.as_ref(),
                                                        &item,
                                                    );
                                                }
                                            },
                                        );
                                    }
                                });

                                row.col(|ui| {
                                    ui.label(format!("{}", item_reference.item_number));
                                });

                                row.col(|ui| {
                                    ui.label(item_data.name);
                                });

                                row.col(|ui| match app_state.get() {
                                    AppState::Game => {
                                        if ui.button("Spawn").clicked() {
                                            if let Some(game_connection) = game_connection.as_ref()
                                            {
                                                if let Some(item_type) = encode_item_type(
                                                    ui_state_debug_item_list.filter_item_type,
                                                ) {
                                                    game_connection
                                                        .client_message_tx
                                                        .send(ClientMessage::Chat {
                                                            text: format!(
                                                                "{} {} {} {} {} {} {}",
                                                                if ui_state_debug_item_list
                                                                    .spawn_as_drop
                                                                {
                                                                    "/drop"
                                                                } else {
                                                                    "/item"
                                                                },
                                                                item_type,
                                                                item_reference.item_number,
                                                                ui_state_debug_item_list
                                                                    .spawn_quantity,
                                                                if ui_state_debug_item_list
                                                                    .spawn_has_socket
                                                                {
                                                                    "1"
                                                                } else {
                                                                    "0"
                                                                },
                                                                ui_state_debug_item_list.spawn_gem,
                                                                ui_state_debug_item_list
                                                                    .spawn_grade
                                                            ),
                                                        })
                                                        .ok();
                                                }
                                            }
                                        }
                                    }
                                    AppState::ModelViewer => {
                                        if let Some(equipment_index) = equipment_index {
                                            if ui.button("Equip").clicked() {
                                                for mut equipment in query_set.p0().iter_mut() {
                                                    equipment.equipped_items[equipment_index] =
                                                        Some(
                                                            EquipmentItem::from_item_data(
                                                                item_data,
                                                            )
                                                            .unwrap(),
                                                        );

                                                    if item_data.class.is_two_handed_weapon() {
                                                        equipment.equipped_items
                                                            [EquipmentIndex::SubWeapon] = None;
                                                    }
                                                }
                                            }
                                        }

                                        if matches!(
                                            ui_state_debug_item_list.filter_item_type,
                                            ItemType::Gem
                                        ) && ui.button("Equip").clicked()
                                        {
                                            for mut equipment in query_set.p0().iter_mut() {
                                                if let Some(weapon) = equipment.equipped_items
                                                    [EquipmentIndex::Weapon]
                                                    .as_mut()
                                                {
                                                    weapon.has_socket = true;
                                                    weapon.gem = item_reference.item_number as u16;
                                                }

                                                if let Some(sub_weapon) = equipment.equipped_items
                                                    [EquipmentIndex::SubWeapon]
                                                    .as_mut()
                                                {
                                                    sub_weapon.has_socket = true;
                                                    sub_weapon.gem =
                                                        item_reference.item_number as u16;
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                });
                            }
                        },
                    );
                });
        });
}
