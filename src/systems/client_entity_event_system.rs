use bevy::prelude::{
    AssetServer, Commands, EventReader, EventWriter, GlobalTransform, Query, Res, Transform,
};

use rose_data::SoundId;
use rose_file_readers::VfsPathBuf;
use rose_game_common::components::Npc;

use crate::{
    audio::SpatialSound,
    components::{PlayerCharacter, SoundCategory},
    events::{ChatboxEvent, ClientEntityEvent, SpawnEffectData, SpawnEffectEvent},
    resources::{GameData, SoundCache, SoundSettings},
};

pub fn client_entity_event_system(
    mut commands: Commands,
    mut client_entity_events: EventReader<ClientEntityEvent>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_player: Query<&PlayerCharacter>,
    query_global_transform: Query<&GlobalTransform>,
    query_npc: Query<(&Npc, &GlobalTransform)>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
    sound_cache: Res<SoundCache>,
) {
    let is_player = |entity| query_player.contains(entity);

    for event in client_entity_events.iter() {
        match *event {
            ClientEntityEvent::Die(entity) => {
                if let Ok((npc, global_transform)) = query_npc.get(entity) {
                    if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                        if let Some(sound_data) = npc_data
                            .die_sound_id
                            .and_then(|id| game_data.sounds.get_sound(id))
                        {
                            commands.spawn((
                                SoundCategory::NpcSounds,
                                sound_settings.gain(SoundCategory::NpcSounds),
                                SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                                Transform::from_translation(global_transform.translation()),
                                GlobalTransform::from_translation(global_transform.translation()),
                            ));
                        }

                        if let Some(die_effect_file_id) = npc_data.die_effect_file_id {
                            spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                                entity,
                                None,
                                SpawnEffectData::with_file_id(die_effect_file_id),
                            ));
                        }
                    }
                }
            }
            ClientEntityEvent::LevelUp(entity, level) => {
                let sound_category = if is_player(entity) {
                    if let Some(level) = level {
                        chatbox_events.send(ChatboxEvent::System(format!(
                            "Congratulations! You are now level {}!",
                            level
                        )));
                    }

                    SoundCategory::PlayerCombat
                } else {
                    SoundCategory::OtherCombat
                };

                if let Ok(global_transform) = query_global_transform.get(entity) {
                    if let Some(sound_data) = game_data.sounds.get_sound(SoundId::new(16).unwrap())
                    {
                        commands.spawn((
                            sound_category,
                            sound_settings.gain(sound_category),
                            SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                            Transform::from_translation(global_transform.translation()),
                            GlobalTransform::from_translation(global_transform.translation()),
                        ));
                    }
                }

                spawn_effect_events.send(SpawnEffectEvent::OnEntity(
                    entity,
                    None,
                    SpawnEffectData::with_path(VfsPathBuf::new("3DDATA/EFFECT/LEVELUP_01.EFT")),
                ));
            }
        }
    }
}
