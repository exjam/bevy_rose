use bevy::prelude::Entity;
use rose_data::ZoneId;
use rose_game_common::messages::ClientEntityId;

pub struct ClientEntityList {
    pub client_entities: Vec<Option<Entity>>,
    pub player_entity: Option<Entity>,
    pub player_entity_id: Option<ClientEntityId>,
    pub zone_id: Option<ZoneId>,
}

impl Default for ClientEntityList {
    fn default() -> Self {
        Self {
            client_entities: vec![None; 4096],
            player_entity: None,
            player_entity_id: None,
            zone_id: None,
        }
    }
}

impl ClientEntityList {
    pub fn add(&mut self, id: ClientEntityId, entity: Entity) {
        self.client_entities[id.0 as usize] = Some(entity);
    }

    pub fn remove(&mut self, id: ClientEntityId) {
        self.client_entities[id.0 as usize] = None;
    }

    pub fn clear(&mut self) {
        self.client_entities.fill(None);
    }

    pub fn get(&self, id: ClientEntityId) -> Option<Entity> {
        self.client_entities[id.0 as usize]
    }
}