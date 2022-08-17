mod ability_values_system;
mod animation_effect_system;
mod animation_sound_system;
mod animation_system;
mod auto_login_system;
mod background_music_system;
mod character_model_system;
mod character_select_system;
mod client_entity_event_system;
mod collision_system;
mod command_system;
mod conversation_dialog_system;
mod cooldown_system;
mod damage_digit_render_system;
mod debug_inspector_system;
mod debug_render_collider_system;
mod debug_render_polylines_system;
mod debug_render_skeleton_system;
mod effect_system;
mod game_mouse_input_system;
mod game_system;
mod hit_event_system;
mod item_drop_model_system;
mod login_connection_system;
mod login_system;
mod model_viewer_system;
mod network_thread_system;
mod npc_idle_sound_system;
mod npc_model_system;
mod particle_sequence_system;
mod passive_recovery_system;
mod pending_damage_system;
mod pending_skill_effect_system;
mod player_command_system;
mod projectile_system;
mod quest_trigger_system;
mod spawn_effect_system;
mod spawn_projectile_system;
mod systemfunc_event_system;
mod update_position_system;
mod visible_status_effects_system;
mod world_connection_system;
mod world_time_system;
mod zone_time_system;
mod zone_viewer_system;

pub use ability_values_system::ability_values_system;
pub use animation_effect_system::animation_effect_system;
pub use animation_sound_system::animation_sound_system;
pub use animation_system::animation_system;
pub use auto_login_system::auto_login_system;
pub use background_music_system::background_music_system;
pub use character_model_system::{
    character_model_add_collider_system, character_model_blink_system,
    character_model_changed_collider_system, character_model_system,
    character_personal_store_model_add_collider_system,
};
pub use character_select_system::{
    character_select_enter_system, character_select_event_system, character_select_exit_system,
    character_select_input_system, character_select_models_system, character_select_system,
};
pub use client_entity_event_system::client_entity_event_system;
pub use collision_system::{
    collision_height_only_system, collision_player_system, collision_player_system_join_zoin,
};
pub use command_system::command_system;
pub use conversation_dialog_system::conversation_dialog_system;
pub use cooldown_system::cooldown_system;
pub use damage_digit_render_system::damage_digit_render_system;
pub use debug_inspector_system::DebugInspectorPlugin;
pub use debug_render_collider_system::debug_render_collider_system;
pub use debug_render_polylines_system::{
    debug_render_polylines_setup_system, debug_render_polylines_update_system,
};
pub use debug_render_skeleton_system::debug_render_skeleton_system;
pub use effect_system::effect_system;
pub use game_mouse_input_system::game_mouse_input_system;
pub use game_system::{game_state_enter_system, game_zone_change_system};
pub use hit_event_system::hit_event_system;
pub use item_drop_model_system::{item_drop_model_add_collider_system, item_drop_model_system};
pub use login_connection_system::login_connection_system;
pub use login_system::{
    login_event_system, login_state_enter_system, login_state_exit_system, login_system,
};
pub use model_viewer_system::{
    model_viewer_enter_system, model_viewer_exit_system, model_viewer_system,
};
pub use network_thread_system::network_thread_system;
pub use npc_idle_sound_system::npc_idle_sound_system;
pub use npc_model_system::{npc_model_add_collider_system, npc_model_system};
pub use particle_sequence_system::particle_sequence_system;
pub use passive_recovery_system::passive_recovery_system;
pub use pending_damage_system::pending_damage_system;
pub use pending_skill_effect_system::pending_skill_effect_system;
pub use player_command_system::player_command_system;
pub use projectile_system::projectile_system;
pub use quest_trigger_system::quest_trigger_system;
pub use spawn_effect_system::spawn_effect_system;
pub use spawn_projectile_system::spawn_projectile_system;
pub use systemfunc_event_system::system_func_event_system;
pub use update_position_system::update_position_system;
pub use visible_status_effects_system::visible_status_effects_system;
pub use world_connection_system::world_connection_system;
pub use world_time_system::world_time_system;
pub use zone_time_system::zone_time_system;
pub use zone_viewer_system::zone_viewer_enter_system;

#[cfg(not(target_arch = "wasm32"))]
mod game_connection_system;
#[cfg(not(target_arch = "wasm32"))]
pub use game_connection_system::game_connection_system;

#[cfg(target_arch = "wasm32")]
pub fn game_connection_system() {}
