use bevy_egui::egui;

use rose_data::{Item, ItemClass, ItemType, SkillCooldown, SkillId, StatusEffectType};
use rose_game_common::components::{ItemSlot, SkillSlot};

use crate::{
    components::{ConsumableCooldownGroup, Cooldowns},
    resources::{GameData, UiResources, UiSprite, UiSpriteSheetType},
};

#[derive(Copy, Clone, Debug)]
pub enum DragAndDropId {
    NotDraggable,
    Inventory(ItemSlot),
    Skill(SkillSlot),
    Hotbar(usize, usize),
    NpcStore(usize, usize),
    NpcStoreBuyList(usize),
    NpcStoreSellList(usize),
    PersonalStoreSell(usize),
    Bank(usize),
}

pub struct DragAndDropSlot<'a> {
    dnd_id: DragAndDropId,
    size: egui::Vec2,
    border_width: f32,
    sprite: Option<UiSprite>,
    socket_sprite: Option<UiSprite>,
    broken: bool,
    cooldown_percent: Option<f32>,
    quantity: Option<usize>,
    quantity_margin: f32,
    accepts: fn(&DragAndDropId) -> bool,
    dragged_item: Option<&'a mut Option<DragAndDropId>>,
    dropped_item: Option<&'a mut Option<DragAndDropId>>,
}

impl<'a> DragAndDropSlot<'a> {
    pub fn new(
        dnd_id: DragAndDropId,
        sprite: Option<UiSprite>,
        socket_sprite: Option<UiSprite>,
        broken: bool,
        quantity: Option<usize>,
        cooldown_percent: Option<f32>,
        accepts: fn(&DragAndDropId) -> bool,
        dragged_item: &'a mut Option<DragAndDropId>,
        dropped_item: &'a mut Option<DragAndDropId>,
        size: impl Into<egui::Vec2>,
    ) -> Self {
        Self {
            dnd_id,
            size: size.into(),
            border_width: 1.0,
            sprite,
            socket_sprite,
            broken,
            cooldown_percent,
            quantity,
            quantity_margin: 2.0,
            accepts,
            dragged_item: Some(dragged_item),
            dropped_item: Some(dropped_item),
        }
    }

    pub fn with_item(
        dnd_id: DragAndDropId,
        item: Option<&Item>,
        cooldowns: Option<&Cooldowns>,
        game_data: &GameData,
        ui_resources: &UiResources,
        accepts: fn(&DragAndDropId) -> bool,
        dragged_item: &'a mut Option<DragAndDropId>,
        dropped_item: &'a mut Option<DragAndDropId>,
        size: impl Into<egui::Vec2>,
    ) -> Self {
        let item_data =
            item.and_then(|item| game_data.items.get_base_item(item.get_item_reference()));
        let sprite = item_data.and_then(|item_data| {
            ui_resources.get_sprite_by_index(UiSpriteSheetType::Item, item_data.icon_index as usize)
        });
        let socket_sprite = item
            .and_then(|item| item.as_equipment())
            .and_then(|equipment_item| {
                if equipment_item.has_socket {
                    if equipment_item.gem > 300 {
                        let gem_item_data =
                            game_data.items.get_gem_item(equipment_item.gem as usize)?;
                        ui_resources.get_sprite_by_index(
                            UiSpriteSheetType::ItemSocketGem,
                            gem_item_data.gem_sprite_id as usize,
                        )
                    } else {
                        ui_resources.get_item_socket_sprite()
                    }
                } else {
                    None
                }
            });
        let broken = item
            .and_then(|item| item.as_equipment())
            .map_or(false, |item| item.life == 0);
        let quantity = match item {
            Some(Item::Stackable(stackable_item)) => Some(stackable_item.quantity as usize),
            _ => None,
        };
        let mut cooldown_percent = None;
        if let Some(cooldowns) = cooldowns {
            if let Some(item) = item.as_ref() {
                if item.get_item_type() == ItemType::Consumable {
                    if let Some(consumable_item_data) =
                        game_data.items.get_consumable_item(item.get_item_number())
                    {
                        if matches!(consumable_item_data.item_data.class, ItemClass::MagicItem) {
                            cooldown_percent = cooldowns.get_consumable_cooldown_percent(
                                ConsumableCooldownGroup::MagicItem,
                            );
                        } else if let Some(status_effect) = consumable_item_data
                            .apply_status_effect
                            .and_then(|(status_effect_id, _)| {
                                game_data.status_effects.get_status_effect(status_effect_id)
                            })
                        {
                            match status_effect.status_effect_type {
                                StatusEffectType::IncreaseHp => {
                                    cooldown_percent = cooldowns.get_consumable_cooldown_percent(
                                        ConsumableCooldownGroup::HealthRecovery,
                                    )
                                }
                                StatusEffectType::IncreaseMp => {
                                    cooldown_percent = cooldowns.get_consumable_cooldown_percent(
                                        ConsumableCooldownGroup::ManaRecovery,
                                    )
                                }
                                _ => {
                                    cooldown_percent = cooldowns.get_consumable_cooldown_percent(
                                        ConsumableCooldownGroup::Others,
                                    )
                                }
                            }
                        } else {
                            cooldown_percent = cooldowns
                                .get_consumable_cooldown_percent(ConsumableCooldownGroup::Others);
                        }
                    }
                }
            }
        }

        Self {
            dnd_id,
            size: size.into(),
            border_width: 1.0,
            sprite,
            socket_sprite,
            broken,
            cooldown_percent,
            quantity,
            quantity_margin: 2.0,
            accepts,
            dragged_item: Some(dragged_item),
            dropped_item: Some(dropped_item),
        }
    }

    pub fn with_skill(
        dnd_id: DragAndDropId,
        skill: Option<&SkillId>,
        cooldowns: Option<&Cooldowns>,
        game_data: &GameData,
        ui_resources: &UiResources,
        accepts: fn(&DragAndDropId) -> bool,
        dragged_item: &'a mut Option<DragAndDropId>,
        dropped_item: &'a mut Option<DragAndDropId>,
        size: impl Into<egui::Vec2>,
    ) -> Self {
        let skill_data = skill.and_then(|skill| game_data.skills.get_skill(*skill));

        let sprite = skill_data.and_then(|skill_data| {
            ui_resources
                .get_sprite_by_index(UiSpriteSheetType::Skill, skill_data.icon_number as usize)
        });

        let cooldown_percent = if let Some(cooldowns) = cooldowns {
            skill_data.and_then(|skill_data| match &skill_data.cooldown {
                SkillCooldown::Skill(_) => cooldowns.get_skill_cooldown_percent(skill_data.id),
                SkillCooldown::Group(group, _) => {
                    cooldowns.get_skill_group_cooldown_percent(*group)
                }
            })
        } else {
            None
        };

        Self {
            dnd_id,
            size: size.into(),
            border_width: 1.0,
            sprite,
            socket_sprite: None,
            broken: false,
            cooldown_percent,
            quantity: None,
            quantity_margin: 2.0,
            accepts,
            dragged_item: Some(dragged_item),
            dropped_item: Some(dropped_item),
        }
    }
}

fn generate_cooldown_mesh(cooldown: f32, content_rect: egui::Rect) -> egui::epaint::Mesh {
    use egui::epaint::*;

    let segment_size = Vec2::new(content_rect.width() / 2.0, content_rect.height() / 2.0);
    let mut mesh = Mesh::default();

    let add_vert = |mesh: &mut Mesh, x, y| {
        let pos = mesh.vertices.len();
        mesh.vertices.push(Vertex {
            pos: Pos2::new(x, y),
            uv: WHITE_UV,
            color: Color32::from_rgba_unmultiplied(40, 40, 40, 160),
        });
        pos as u32
    };

    /*
     * 2 1+9 8
     * 3  0  7
     * 4  5  6
     */
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.min.y + segment_size.y,
    );
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.min.y,
    );
    add_vert(&mut mesh, content_rect.min.x, content_rect.min.y);
    add_vert(
        &mut mesh,
        content_rect.min.x,
        content_rect.min.y + segment_size.y,
    );
    add_vert(&mut mesh, content_rect.min.x, content_rect.max.y);
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.max.y,
    );
    add_vert(&mut mesh, content_rect.max.x, content_rect.max.y);
    add_vert(
        &mut mesh,
        content_rect.max.x,
        content_rect.min.y + segment_size.y,
    );
    add_vert(&mut mesh, content_rect.max.x, content_rect.min.y);
    add_vert(
        &mut mesh,
        content_rect.min.x + segment_size.x,
        content_rect.min.y,
    );

    /*
     * Triangles:
     * _______
     * |\ | /|
     * |_\|/_|
     * | /|\ |
     * |/ | \|
     * -------
     */
    const TRIANGLES_COUNT: f32 = 8.0;
    let segments = cooldown * TRIANGLES_COUNT;
    let num_segments = segments.trunc() as u32;
    for segment_id in 0..num_segments {
        mesh.add_triangle(0, segment_id + 1, segment_id + 2);
    }

    let fract_segments = segments.fract();
    if fract_segments > 0.0 {
        if let (Some(vert_1), Some(vert_2)) = (
            mesh.vertices.get(num_segments as usize + 1).map(|x| x.pos),
            mesh.vertices.get(num_segments as usize + 2).map(|x| x.pos),
        ) {
            let vertex_id = add_vert(
                &mut mesh,
                (vert_2.x - vert_1.x) * fract_segments + vert_1.x,
                (vert_2.y - vert_1.y) * fract_segments + vert_1.y,
            );
            mesh.add_triangle(0, num_segments + 1, vertex_id);
        }
    }

    mesh
}

impl<'w> DragAndDropSlot<'w> {
    pub fn draw(&self, ui: &mut egui::Ui, accepts_dragged_item: bool) -> (bool, egui::Response) {
        let (rect, response) = ui.allocate_exact_size(
            self.size,
            if self.sprite.is_some() && !matches!(self.dnd_id, DragAndDropId::NotDraggable) {
                egui::Sense::click_and_drag()
            } else {
                egui::Sense::click()
            },
        );
        let mut dropped = false;

        if ui.is_rect_visible(rect) {
            use egui::epaint::*;

            // For some reason, we must do manual implementation of response.hovered
            let is_active = ui.ctx().input(|input| {
                let hovered = input
                    .pointer
                    .interact_pos()
                    .map_or(false, |cursor_pos| rect.contains(cursor_pos));

                if accepts_dragged_item && hovered {
                    if input.pointer.any_released()
                        && !input.pointer.button_down(egui::PointerButton::Primary)
                    {
                        dropped = true;
                    }
                    true
                } else {
                    false
                }
            });

            if let Some(sprite) = self.sprite.as_ref() {
                let content_rect = rect;
                let mut mesh = Mesh::with_texture(sprite.texture_id);
                mesh.add_rect_with_uv(
                    content_rect,
                    sprite.uv,
                    if !self.broken {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::LIGHT_RED
                    },
                );
                ui.painter().add(Shape::mesh(mesh));

                if let Some(socket_sprite) = self.socket_sprite.as_ref() {
                    let mut mesh = Mesh::with_texture(socket_sprite.texture_id);
                    mesh.add_rect_with_uv(
                        egui::Rect::from_min_size(
                            content_rect.min,
                            egui::vec2(socket_sprite.width, socket_sprite.height),
                        ),
                        socket_sprite.uv,
                        egui::Color32::WHITE,
                    );
                    ui.painter().add(Shape::mesh(mesh));
                }

                if let Some(cooldown_percent) = self.cooldown_percent {
                    ui.painter().add(Shape::mesh(generate_cooldown_mesh(
                        cooldown_percent,
                        content_rect,
                    )));
                }

                if let Some(quantity) = self.quantity {
                    let text_galley = ui.fonts(|fonts| {
                        fonts.layout_no_wrap(
                            format!("{}", quantity),
                            FontId::monospace(12.0),
                            Color32::WHITE,
                        )
                    });

                    ui.painter().add(egui::Shape::Rect(egui::epaint::RectShape {
                        rect: Rect::from_min_max(
                            egui::Pos2::new(
                                content_rect.max.x
                                    - text_galley.rect.right()
                                    - self.quantity_margin,
                                content_rect.min.y,
                            ),
                            egui::Pos2::new(
                                content_rect.max.x,
                                content_rect.min.y
                                    + self.quantity_margin * 2.0
                                    + text_galley.rect.height(),
                            ),
                        ),
                        rounding: egui::Rounding::none(),
                        fill: Color32::from_rgba_unmultiplied(50, 50, 50, 200),
                        stroke: Stroke::NONE,
                    }));

                    ui.painter().add(Shape::galley(
                        egui::Pos2::new(
                            content_rect.max.x - text_galley.rect.right(),
                            content_rect.min.y + self.quantity_margin,
                        ),
                        text_galley,
                    ));
                }

                if response.dragged_by(egui::PointerButton::Primary) {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        if !response.rect.contains(pointer_pos) {
                            let tooltip_painter = ui.ctx().layer_painter(egui::LayerId::new(
                                egui::Order::Tooltip,
                                egui::Id::new("dnd_tooltip"),
                            ));
                            let mut tooltip_mesh =
                                egui::epaint::Mesh::with_texture(sprite.texture_id);
                            tooltip_mesh.add_rect_with_uv(
                                response
                                    .rect
                                    .translate(pointer_pos - response.rect.center()),
                                sprite.uv,
                                egui::Color32::WHITE,
                            );
                            tooltip_painter.add(egui::epaint::Shape::mesh(tooltip_mesh));
                        }
                    }
                }
            }

            if is_active {
                ui.painter().add(egui::Shape::Rect(egui::epaint::RectShape {
                    rect: rect.shrink(self.border_width),
                    rounding: egui::Rounding::none(),
                    fill: Default::default(),
                    stroke: egui::Stroke {
                        width: self.border_width,
                        color: egui::Color32::YELLOW,
                    },
                }));
            }
        }
        (dropped, response)
    }
}

impl<'w> egui::Widget for DragAndDropSlot<'w> {
    fn ui(mut self, ui: &mut egui::Ui) -> egui::Response {
        let dnd_id = self.dnd_id;
        let dragged_item = self.dragged_item.take().unwrap();
        let dropped_item = self.dropped_item.take().unwrap();
        let accepts_dragged_item = dragged_item
            .as_ref()
            .map(|dnd_id| (self.accepts)(dnd_id))
            .unwrap_or(false);

        let (dropped, mut response) = self.draw(ui, accepts_dragged_item);

        if response.dragged_by(egui::PointerButton::Primary) {
            *dragged_item = Some(dnd_id);
        } else if dropped {
            *dropped_item = dragged_item.take();
            response.mark_changed();
        }

        response
    }
}
