use bevy::{prelude::Component, reflect::Reflect};

pub use rose_game_common::messages::ClientEntityId;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Reflect)]
pub enum ClientEntityType {
    Character,
    Monster,
    Npc,
    ItemDrop,
}

#[derive(Copy, Clone, Component, Reflect)]
pub struct ClientEntity {
    pub id: ClientEntityId,
    pub entity_type: ClientEntityType,
}

impl ClientEntity {
    pub fn new(id: ClientEntityId, entity_type: ClientEntityType) -> Self {
        Self { id, entity_type }
    }
}
