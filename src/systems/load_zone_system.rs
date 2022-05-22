use arrayvec::ArrayVec;
use bevy::{
    asset::LoadState,
    hierarchy::BuildChildren,
    math::{Quat, Vec2, Vec3},
    pbr::{AlphaMode, NotShadowCaster, NotShadowReceiver, StandardMaterial},
    prelude::{
        AssetServer, Assets, Color, Commands, Component, ComputedVisibility, DespawnRecursiveExt,
        Entity, EventReader, EventWriter, GlobalTransform, Handle, Local, Mesh, Query, Res, ResMut,
        Transform, Visibility, With,
    },
    render::{
        mesh::Indices,
        render_resource::{Face, PrimitiveTopology},
        view::NoFrustumCulling,
    },
};
use bevy_inspector_egui::Inspectable;
use bevy_rapier3d::prelude::{AsyncCollider, Collider, CollisionGroups};
use std::path::Path;

use rose_data::{WarpGateId, ZoneId, ZoneListEntry};
use rose_file_readers::{
    HimFile, IfoFile, IfoObject, LitFile, LitObject, StbFile, TilFile, ZonFile, ZonTile,
    ZonTileRotation, ZscCollisionFlags, ZscCollisionShape, ZscEffectType, ZscFile,
};

use crate::{
    components::{
        ActiveMotion, ColliderEntity, ColliderParent, EventObject, NightTimeEffect, WarpObject,
        COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_INSPECTABLE,
        COLLISION_GROUP_ZONE_EVENT_OBJECT, COLLISION_GROUP_ZONE_OBJECT,
        COLLISION_GROUP_ZONE_TERRAIN, COLLISION_GROUP_ZONE_WARP_OBJECT, COLLISION_GROUP_ZONE_WATER,
    },
    effect_loader::spawn_effect,
    events::{LoadZoneEvent, ZoneEvent},
    render::{
        EffectMeshMaterial, ParticleMaterial, RgbTextureLoader, SkyMaterial, StaticMeshMaterial,
        TerrainMaterial, TextureArray, TextureArrayBuilder, WaterMaterial, MESH_ATTRIBUTE_UV_1,
        TERRAIN_MESH_ATTRIBUTE_TILE_INFO,
    },
    resources::{CurrentZone, GameData},
    VfsResource,
};

const SKYBOX_MODEL_SCALE: f32 = 10.0;

#[derive(Inspectable)]
pub enum ZoneObjectPartCollisionShape {
    None,
    Sphere,
    AxisAlignedBoundingBox,
    ObjectOrientedBoundingBox,
    Polygon,
}

impl Default for ZoneObjectPartCollisionShape {
    fn default() -> Self {
        Self::AxisAlignedBoundingBox
    }
}

impl From<&Option<ZscCollisionShape>> for ZoneObjectPartCollisionShape {
    fn from(value: &Option<ZscCollisionShape>) -> Self {
        match value {
            Some(ZscCollisionShape::Sphere) => Self::Sphere,
            Some(ZscCollisionShape::AxisAlignedBoundingBox) => Self::AxisAlignedBoundingBox,
            Some(ZscCollisionShape::ObjectOrientedBoundingBox) => Self::ObjectOrientedBoundingBox,
            Some(ZscCollisionShape::Polygon) => Self::Polygon,
            None => Self::None,
        }
    }
}

#[derive(Inspectable, Default)]
pub struct ZoneObjectId {
    pub id: usize,
}

#[derive(Inspectable, Default)]
pub struct ZoneObjectPart {
    pub object_id: usize,
    pub mesh_path: String,
    pub collision_shape: ZoneObjectPartCollisionShape,
    pub collision_not_moveable: bool,
    pub collision_not_pickable: bool,
    pub collision_height_only: bool,
    pub collision_no_camera: bool,
}

#[derive(Inspectable, Default)]
pub struct ZoneObjectAnimatedObject {
    pub mesh_path: String,
    pub motion_path: String,
    pub texture_path: String,
}

#[derive(Inspectable, Default)]
pub struct ZoneObjectTerrain {
    pub block_x: u32,
    pub block_y: u32,
}

#[derive(Component, Inspectable)]
pub enum ZoneObject {
    AnimatedObject(ZoneObjectAnimatedObject),
    WarpObject(ZoneObjectId),
    WarpObjectPart(ZoneObjectPart),
    EventObject(ZoneObjectId),
    EventObjectPart(ZoneObjectPart),
    CnstObject(ZoneObjectId),
    CnstObjectPart(ZoneObjectPart),
    DecoObject(ZoneObjectId),
    DecoObjectPart(ZoneObjectPart),
    Terrain(ZoneObjectTerrain),
    Water,
}

pub enum LoadZoneState {
    None,
    Loading(ZoneId),
    Loaded(ZoneId),
}

impl Default for LoadZoneState {
    fn default() -> Self {
        Self::None
    }
}

#[allow(clippy::too_many_arguments)]
pub fn load_zone_system(
    mut commands: Commands,
    (asset_server, game_data, vfs_resource): (Res<AssetServer>, Res<GameData>, Res<VfsResource>),
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<TerrainMaterial>>,
    mut effect_mesh_materials: ResMut<Assets<EffectMeshMaterial>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut sky_materials: ResMut<Assets<SkyMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut static_mesh_materials: ResMut<Assets<StaticMeshMaterial>>,
    (mut water_materials, mut texture_arrays): (
        ResMut<Assets<WaterMaterial>>,
        ResMut<Assets<TextureArray>>,
    ),
    mut load_zone_state: Local<LoadZoneState>,
    mut loading_current_zone: Local<Option<CurrentZone>>,
    mut load_zone_event: EventReader<LoadZoneEvent>,
    mut zone_events: EventWriter<ZoneEvent>,
    query_sky: Query<Entity, With<Handle<SkyMaterial>>>,
    query_zone_objects: Query<(Entity, Option<&Handle<Mesh>>), With<ZoneObject>>,
) {
    let current_zone_id = match *load_zone_state {
        LoadZoneState::None => None,
        LoadZoneState::Loading(zone_id) => Some(zone_id),
        LoadZoneState::Loaded(zone_id) => Some(zone_id),
    };

    // Check if we need to load a new zone
    if let Some(load_zone_event) = load_zone_event.iter().last() {
        *load_zone_state = LoadZoneState::Loading(load_zone_event.id);
    }

    let load_zone_id = match *load_zone_state {
        LoadZoneState::None => None,
        LoadZoneState::Loading(zone_id) => Some(zone_id),
        LoadZoneState::Loaded(zone_id) => Some(zone_id),
    };

    if current_zone_id == load_zone_id {
        if let LoadZoneState::Loading(zone_id) = *load_zone_state {
            let mut loaded = true;

            // Check if zone has finished loading
            for (_, mesh) in query_zone_objects.iter() {
                if let Some(handle) = mesh {
                    if matches!(asset_server.get_load_state(handle), LoadState::Loading) {
                        loaded = false;
                        break;
                    }
                }
            }

            if loaded {
                if let Some(current_zone) = loading_current_zone.take() {
                    commands.insert_resource(current_zone);
                }

                *load_zone_state = LoadZoneState::Loaded(zone_id);
                zone_events.send(ZoneEvent::Loaded(zone_id));
            }
        }

        // Nothing to do
        return;
    }
    let next_zone_id = load_zone_id.unwrap();
    *load_zone_state = LoadZoneState::Loading(next_zone_id);

    // Despawn old zone
    for (entity, _) in query_zone_objects.iter() {
        commands.entity(entity).despawn_recursive();
    }

    for entity in query_sky.iter() {
        commands.entity(entity).despawn_recursive();
    }

    commands.remove_resource::<CurrentZone>();

    // Spawn new zone
    if let Some(zone_list_entry) = game_data.zone_list.get_zone(next_zone_id) {
        *loading_current_zone = load_zone(
            &mut commands,
            &asset_server,
            &game_data,
            &vfs_resource,
            &mut meshes,
            &mut terrain_materials,
            &mut effect_mesh_materials,
            &mut particle_materials,
            &mut sky_materials,
            &mut standard_materials,
            &mut static_mesh_materials,
            &mut water_materials,
            &mut texture_arrays,
            zone_list_entry,
        )
        .ok();
    }
}

#[allow(clippy::too_many_arguments)]
fn load_zone(
    commands: &mut Commands,
    asset_server: &AssetServer,
    game_data: &GameData,
    vfs_resource: &VfsResource,
    meshes: &mut ResMut<Assets<Mesh>>,
    terrain_materials: &mut ResMut<Assets<TerrainMaterial>>,
    effect_mesh_materials: &mut ResMut<Assets<EffectMeshMaterial>>,
    particle_materials: &mut ResMut<Assets<ParticleMaterial>>,
    sky_materials: &mut ResMut<Assets<SkyMaterial>>,
    standard_materials: &mut ResMut<Assets<StandardMaterial>>,
    static_mesh_materials: &mut ResMut<Assets<StaticMeshMaterial>>,
    water_materials: &mut ResMut<Assets<WaterMaterial>>,
    texture_arrays: &mut ResMut<Assets<TextureArray>>,
    zone_list_entry: &ZoneListEntry,
) -> Result<CurrentZone, anyhow::Error> {
    let zone_file = vfs_resource
        .vfs
        .read_file::<ZonFile, _>(&zone_list_entry.zon_file_path)?;
    let zsc_cnst = vfs_resource
        .vfs
        .read_file::<ZscFile, _>(&zone_list_entry.zsc_cnst_path)
        .ok();
    let zsc_deco = vfs_resource
        .vfs
        .read_file::<ZscFile, _>(&zone_list_entry.zsc_deco_path)
        .ok();

    // TODO: Cache these
    let zsc_event_object = vfs_resource
        .vfs
        .read_file::<ZscFile, _>("3DDATA/SPECIAL/EVENT_OBJECT.ZSC")
        .ok();
    let zsc_special_object = vfs_resource
        .vfs
        .read_file::<ZscFile, _>("3DDATA/SPECIAL/LIST_DECO_SPECIAL.ZSC")
        .ok();
    let stb_morph_object = vfs_resource
        .vfs
        .read_file::<StbFile, _>("3DDATA/STB/LIST_MORPH_OBJECT.STB")
        .ok();

    // Update skybox
    if let Some(skybox_data) = zone_list_entry
        .skybox_id
        .and_then(|skybox_id| game_data.skybox.get_skybox_data(skybox_id))
    {
        commands.spawn_bundle((
            asset_server.load::<Mesh, _>(skybox_data.mesh.path()),
            sky_materials.add(SkyMaterial {
                texture_day: Some(asset_server.load(RgbTextureLoader::convert_path(
                    skybox_data.texture_day.path(),
                ))),
                texture_night: Some(asset_server.load(RgbTextureLoader::convert_path(
                    skybox_data.texture_night.path(),
                ))),
            }),
            Transform::from_scale(Vec3::splat(SKYBOX_MODEL_SCALE)),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            NoFrustumCulling,
        ));
    }

    // Load zone tile array
    let mut tile_texture_array_builder = TextureArrayBuilder::new();
    for path in zone_file.tile_textures.iter() {
        if path == "end" {
            break;
        }

        tile_texture_array_builder.add(path.clone());
    }
    let tile_texture_array = texture_arrays.add(tile_texture_array_builder.build(asset_server));

    // Load zone water array
    let mut water_texture_array_builder = TextureArrayBuilder::new();
    for i in 1..=25 {
        water_texture_array_builder.add(format!("3DDATA/JUNON/WATER/OCEAN01_{:02}.DDS", i));
    }
    let water_material = water_materials.add(WaterMaterial {
        water_texture_array: texture_arrays.add(water_texture_array_builder.build(asset_server)),
    });

    // Load the zone
    let zone_path = zone_list_entry
        .zon_file_path
        .path()
        .parent()
        .unwrap_or_else(|| Path::new(""));

    for block_y in 0..64u32 {
        for block_x in 0..64u32 {
            let tilemap = vfs_resource
                .vfs
                .read_file::<TilFile, _>(zone_path.join(format!("{}_{}.TIL", block_x, block_y)));
            let heightmap = vfs_resource
                .vfs
                .read_file::<HimFile, _>(zone_path.join(format!("{}_{}.HIM", block_x, block_y)));

            if let (Ok(heightmap), Ok(tilemap)) = (heightmap, tilemap) {
                let block_terrain_material = terrain_materials.add(TerrainMaterial {
                    lightmap_texture: asset_server.load(&format!(
                        "{}/{1:}_{2:}/{1:}_{2:}_PLANELIGHTINGMAP.DDS.rgb_texture",
                        zone_path.to_str().unwrap(),
                        block_x,
                        block_y,
                    )),
                    tile_array_texture: tile_texture_array.clone(),
                });

                load_block_heightmap(
                    commands,
                    meshes.as_mut(),
                    heightmap,
                    tilemap,
                    &zone_file.tiles,
                    block_terrain_material,
                    block_x,
                    block_y,
                );
            }

            let ifo = vfs_resource
                .vfs
                .read_file::<IfoFile, _>(zone_path.join(format!("{}_{}.IFO", block_x, block_y)));
            if let Ok(ifo) = ifo {
                let lightmap_path = zone_path.join(format!("{}_{}/LIGHTMAP/", block_x, block_y));
                load_block_waterplanes(
                    commands,
                    meshes.as_mut(),
                    ifo.water_size,
                    &ifo.water_planes,
                    &water_material,
                );

                if let Some(zsc_event_object) = zsc_event_object.as_ref() {
                    for event_object in ifo.event_objects.iter() {
                        let event_entity = load_block_object(
                            commands,
                            asset_server,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            standard_materials.as_mut(),
                            static_mesh_materials.as_mut(),
                            zsc_event_object,
                            &lightmap_path,
                            None,
                            &event_object.object,
                            event_object.object.object_id as usize,
                            ZoneObject::EventObject,
                            ZoneObject::EventObjectPart,
                            COLLISION_GROUP_ZONE_EVENT_OBJECT,
                        );

                        commands.entity(event_entity).insert(EventObject::new(
                            event_object.quest_trigger_name.clone(),
                            event_object.script_function_name.clone(),
                        ));
                    }
                }

                if let Some(zsc_special_object) = zsc_special_object.as_ref() {
                    for warp_object in ifo.warps.iter() {
                        let warp_entity = load_block_object(
                            commands,
                            asset_server,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            standard_materials.as_mut(),
                            static_mesh_materials.as_mut(),
                            zsc_special_object,
                            &lightmap_path,
                            None,
                            warp_object,
                            1,
                            ZoneObject::WarpObject,
                            ZoneObject::WarpObjectPart,
                            COLLISION_GROUP_ZONE_WARP_OBJECT,
                        );

                        commands
                            .entity(warp_entity)
                            .insert(WarpObject::new(WarpGateId::new(warp_object.warp_id)));
                    }
                }

                if let Some(zsc_cnst) = zsc_cnst.as_ref() {
                    let cnst_lit = vfs_resource
                        .vfs
                        .read_file::<LitFile, _>(zone_path.join(format!(
                            "{}_{}/LIGHTMAP/BUILDINGLIGHTMAPDATA.LIT",
                            block_x, block_y
                        )))
                        .ok();

                    for (object_id, object_instance) in ifo.cnst_objects.iter().enumerate() {
                        let lit_object = cnst_lit.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == object_id + 1)
                        });

                        load_block_object(
                            commands,
                            asset_server,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            standard_materials.as_mut(),
                            static_mesh_materials.as_mut(),
                            zsc_cnst,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            object_instance.object_id as usize,
                            ZoneObject::CnstObject,
                            ZoneObject::CnstObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                    }
                }

                if let Some(zsc_deco) = zsc_deco.as_ref() {
                    let deco_lit = vfs_resource
                        .vfs
                        .read_file::<LitFile, _>(zone_path.join(format!(
                            "{}_{}/LIGHTMAP/OBJECTLIGHTMAPDATA.LIT",
                            block_x, block_y
                        )))
                        .ok();

                    for (object_id, object_instance) in ifo.deco_objects.iter().enumerate() {
                        let lit_object = deco_lit.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == object_id + 1)
                        });

                        load_block_object(
                            commands,
                            asset_server,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            standard_materials.as_mut(),
                            static_mesh_materials.as_mut(),
                            zsc_deco,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            object_instance.object_id as usize,
                            ZoneObject::DecoObject,
                            ZoneObject::DecoObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                    }
                }

                if let Some(stb_morph_object) = stb_morph_object.as_ref() {
                    for object_instance in ifo.animated_objects.iter() {
                        load_animated_object(
                            commands,
                            asset_server,
                            static_mesh_materials.as_mut(),
                            stb_morph_object,
                            object_instance,
                        );
                    }
                }
            }
        }
    }

    Ok(CurrentZone {
        id: zone_list_entry.id,
        grid_per_patch: zone_file.grid_per_patch,
        grid_size: zone_file.grid_size,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_block_heightmap(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    heightmap: HimFile,
    tilemap: TilFile,
    tile_info: &[ZonTile],
    material: Handle<TerrainMaterial>,
    block_x: u32,
    block_y: u32,
) {
    let offset_x = 160.0 * block_x as f32;
    let offset_y = 160.0 * (65.0 - block_y as f32);

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs_lightmap = Vec::new();
    let mut uvs_tile = Vec::new();
    let mut indices = Vec::new();
    let mut tile_ids = Vec::new();

    for tile_x in 0..16 {
        for tile_y in 0..16 {
            let tile = &tile_info[tilemap.get_clamped(tile_x, tile_y) as usize];
            let tile_array_index1 = tile.layer1 + tile.offset1;
            let tile_array_index2 = tile.layer2 + tile.offset2;
            let tile_rotation = match tile.rotation {
                ZonTileRotation::FlipHorizontal => 2,
                ZonTileRotation::FlipVertical => 3,
                ZonTileRotation::Flip => 4,
                ZonTileRotation::Clockwise90 => 5,
                ZonTileRotation::CounterClockwise90 => 6,
                _ => 0,
            };
            let tile_indices_base = positions.len() as u16;
            let tile_offset_x = tile_x as f32 * 4.0 * 2.5;
            let tile_offset_y = tile_y as f32 * 4.0 * 2.5;

            for y in 0..5 {
                for x in 0..5 {
                    let heightmap_x = x + tile_x as i32 * 4;
                    let heightmap_y = y + tile_y as i32 * 4;
                    let height = heightmap.get_clamped(heightmap_x, heightmap_y) / 100.0;
                    let height_l = heightmap.get_clamped(heightmap_x - 1, heightmap_y) / 100.0;
                    let height_r = heightmap.get_clamped(heightmap_x + 1, heightmap_y) / 100.0;
                    let height_t = heightmap.get_clamped(heightmap_x, heightmap_y - 1) / 100.0;
                    let height_b = heightmap.get_clamped(heightmap_x, heightmap_y + 1) / 100.0;
                    let normal = Vec3::new(
                        (height_l - height_r) / 2.0,
                        1.0,
                        (height_t - height_b) / 2.0,
                    )
                    .normalize();

                    positions.push([
                        tile_offset_x + x as f32 * 2.5,
                        height,
                        tile_offset_y + y as f32 * 2.5,
                    ]);
                    normals.push([normal.x, normal.y, normal.z]);
                    uvs_tile.push([x as f32 / 4.0, y as f32 / 4.0]);
                    uvs_lightmap.push([
                        (tile_x as f32 * 4.0 + x as f32) / 64.0,
                        (tile_y as f32 * 4.0 + y as f32) / 64.0,
                    ]);
                    tile_ids.push([tile_array_index1, tile_array_index2, tile_rotation]);
                }
            }

            for y in 0..(5 - 1) {
                for x in 0..(5 - 1) {
                    let start = tile_indices_base + y * 5 + x;
                    indices.push(start);
                    indices.push(start + 5);
                    indices.push(start + 1);

                    indices.push(start + 1);
                    indices.push(start + 5);
                    indices.push(start + 1 + 5);
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U16(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs_lightmap);
    mesh.insert_attribute(MESH_ATTRIBUTE_UV_1, uvs_tile);
    mesh.insert_attribute(TERRAIN_MESH_ATTRIBUTE_TILE_INFO, tile_ids);

    let mut collider_verts = Vec::new();
    let mut collider_indices = Vec::new();

    for y in 0..heightmap.height as i32 {
        for x in 0..heightmap.width as i32 {
            collider_verts.push(
                [
                    offset_x + x as f32 * 2.5,
                    heightmap.get_clamped(x, y) / 100.0,
                    -offset_y + y as f32 * 2.5,
                ]
                .into(),
            );
        }
    }

    for y in 0..(heightmap.height - 1) {
        for x in 0..(heightmap.width - 1) {
            let start = y * heightmap.width + x;
            collider_indices.push([start, start + heightmap.width, start + 1]);
            collider_indices.push([
                start + 1,
                start + heightmap.width,
                start + 1 + heightmap.width,
            ]);
        }
    }

    let terrain_collider_entity = commands
        .spawn_bundle((
            Collider::trimesh(collider_verts, collider_indices),
            CollisionGroups::new(
                COLLISION_GROUP_ZONE_TERRAIN,
                COLLISION_FILTER_INSPECTABLE
                    | COLLISION_FILTER_COLLIDABLE
                    | COLLISION_FILTER_CLICKABLE,
            ),
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();

    let entity = commands
        .spawn_bundle((
            ZoneObject::Terrain(ZoneObjectTerrain { block_x, block_y }),
            meshes.add(mesh),
            material,
            Transform::from_xyz(offset_x, 0.0, -offset_y),
            GlobalTransform::default(),
            Visibility::default(),
            ComputedVisibility::default(),
            NotShadowReceiver {},
            ColliderEntity::new(terrain_collider_entity),
        ))
        .add_child(terrain_collider_entity)
        .id();

    commands
        .entity(terrain_collider_entity)
        .insert(ColliderParent::new(entity));
}

fn load_block_waterplanes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    water_size: f32,
    water_planes: &[(
        rose_file_readers::types::Vec3<f32>,
        rose_file_readers::types::Vec3<f32>,
    )],
    water_material: &Handle<WaterMaterial>,
) {
    for (plane_start, plane_end) in water_planes {
        let start = Vec3::new(
            5200.0 + plane_start.x / 100.0,
            plane_start.y / 100.0,
            -(5200.0 + plane_start.z / 100.0),
        );
        let end = Vec3::new(
            5200.0 + plane_end.x / 100.0,
            plane_end.y / 100.0,
            -(5200.0 + plane_end.z / 100.0),
        );
        let uv_x = (end.x - start.x) / (water_size / 100.0);
        let uv_y = (end.z - start.z) / (water_size / 100.0);

        let vertices = [
            ([start.x, start.y, end.z], [0.0, 1.0, 0.0], [uv_x, uv_y]),
            ([start.x, start.y, start.z], [0.0, 1.0, 0.0], [uv_x, 0.0]),
            ([end.x, start.y, start.z], [0.0, 1.0, 0.0], [0.0, 0.0]),
            ([end.x, start.y, end.z], [0.0, 1.0, 0.0], [0.0, uv_y]),
        ];
        let indices = Indices::U32(vec![0, 2, 1, 0, 3, 2]);
        let collider_indices = vec![[0, 2, 1], [0, 3, 2]];

        let mut collider_verts = Vec::new();
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        for (position, normal, uv) in &vertices {
            collider_verts.push((*position).into());
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        let water_collider_entity = commands
            .spawn_bundle((
                Collider::trimesh(collider_verts, collider_indices),
                CollisionGroups::new(COLLISION_GROUP_ZONE_WATER, COLLISION_FILTER_INSPECTABLE),
                Transform::default(),
                GlobalTransform::default(),
            ))
            .id();

        let entity = commands
            .spawn()
            .insert_bundle((
                ZoneObject::Water,
                meshes.add(mesh),
                water_material.clone(),
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
                NotShadowCaster {},
                NotShadowReceiver {},
                ColliderEntity::new(water_collider_entity),
            ))
            .add_child(water_collider_entity)
            .id();

        commands
            .entity(water_collider_entity)
            .insert(ColliderParent::new(entity));
    }
}

fn load_block_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    vfs_resource: &VfsResource,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,
    particle_materials: &mut Assets<ParticleMaterial>,
    standard_materials: &mut Assets<StandardMaterial>,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    zsc: &ZscFile,
    lightmap_path: &Path,
    lit_object: Option<&LitObject>,
    object_instance: &IfoObject,
    object_id: usize,
    object_type: fn(ZoneObjectId) -> ZoneObject,
    part_object_type: fn(ZoneObjectPart) -> ZoneObject,
    collision_group: u32,
) -> Entity {
    let object = &zsc.objects[object_id as usize];
    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    #[derive(Clone)]
    enum LoadedMaterial {
        Static(Handle<StaticMeshMaterial>),
        Standard(Handle<StandardMaterial>),
    }

    let mut material_cache: Vec<Option<LoadedMaterial>> = vec![None; zsc.materials.len()];
    let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];

    let mut part_entities: ArrayVec<Entity, 32> = ArrayVec::new();
    let mut object_entity_commands = commands.spawn_bundle((
        object_type(ZoneObjectId { id: object_id }),
        object_transform,
        GlobalTransform::default(),
    ));

    let object_entity = object_entity_commands.id();

    object_entity_commands.with_children(|object_commands| {
        for (part_index, object_part) in object.parts.iter().enumerate() {
            let part_transform = Transform::default()
                .with_translation(
                    Vec3::new(
                        object_part.position.x,
                        object_part.position.z,
                        -object_part.position.y,
                    ) / 100.0,
                )
                .with_rotation(Quat::from_xyzw(
                    object_part.rotation.x,
                    object_part.rotation.z,
                    -object_part.rotation.y,
                    object_part.rotation.w,
                ))
                .with_scale(Vec3::new(
                    object_part.scale.x,
                    object_part.scale.z,
                    object_part.scale.y,
                ));

            let mesh_id = object_part.mesh_id as usize;
            let mesh = mesh_cache[mesh_id].clone().unwrap_or_else(|| {
                let handle = asset_server.load(zsc.meshes[mesh_id].path());
                mesh_cache.insert(mesh_id, Some(handle.clone()));
                handle
            });
            let lit_part = lit_object.and_then(|lit_object| lit_object.parts.get(part_index));
            let lightmap_texture = lit_part.map(|lit_part| {
                asset_server.load(RgbTextureLoader::convert_path(
                    &lightmap_path.join(&lit_part.filename),
                ))
            });
            let (lightmap_uv_offset, lightmap_uv_scale) = lit_part
                .map(|lit_part| {
                    let scale = 1.0 / lit_part.parts_per_row as f32;
                    (
                        Vec2::new(
                            (lit_part.part_index % lit_part.parts_per_row) as f32,
                            (lit_part.part_index / lit_part.parts_per_row) as f32,
                        ),
                        scale,
                    )
                })
                .unwrap_or((Vec2::new(0.0, 0.0), 1.0));

            let material_id = object_part.material_id as usize;
            let material = material_cache[material_id].clone().unwrap_or_else(|| {
                let zsc_material = &zsc.materials[material_id];

                let texture_path = zsc_material.path.path();
                let filename = texture_path
                    .with_extension("")
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                let diffuse_texture_path = zsc_material.path.path();
                let normal_texture =
                    texture_path.with_file_name(format!("{}_normal.dds", filename));
                let smoothness_texture =
                    texture_path.with_file_name(format!("{}_smoothness.dds", filename));
                let occlusion_texture = texture_path.with_file_name(format!("{}_ao.dds", filename));

                if vfs_resource.vfs.exists(normal_texture.clone()) {
                    let handle = standard_materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(asset_server.load(diffuse_texture_path)),
                        emissive: Color::BLACK,
                        emissive_texture: None,
                        perceptual_roughness: 1.0,
                        metallic: 1.0,
                        metallic_roughness_texture: Some(asset_server.load(smoothness_texture)),
                        reflectance: 0.05,
                        normal_map_texture: Some(asset_server.load(normal_texture)),
                        flip_normal_map_y: false,
                        occlusion_texture: Some(asset_server.load(occlusion_texture)),
                        double_sided: false,
                        cull_mode: Some(Face::Back),
                        unlit: false,
                        alpha_mode: AlphaMode::Opaque,
                    });
                    material_cache
                        .insert(material_id, Some(LoadedMaterial::Standard(handle.clone())));
                    LoadedMaterial::Standard(handle)
                } else {
                    let handle = static_mesh_materials.add(StaticMeshMaterial {
                        base_texture: Some(
                            asset_server
                                .load(RgbTextureLoader::convert_path(zsc_material.path.path())),
                        ),
                        lightmap_texture,
                        alpha_value: if zsc_material.alpha != 1.0 {
                            Some(zsc_material.alpha)
                        } else {
                            None
                        },
                        alpha_enabled: zsc_material.alpha_enabled,
                        alpha_test: zsc_material.alpha_test,
                        two_sided: zsc_material.two_sided,
                        z_write_enabled: zsc_material.z_write_enabled,
                        z_test_enabled: zsc_material.z_test_enabled,
                        specular_enabled: zsc_material.specular_enabled,
                        skinned: zsc_material.is_skin,
                        lightmap_uv_offset,
                        lightmap_uv_scale,
                        /*
                        pub blend_mode: SceneBlendMode,
                        pub glow: Option<ZscMaterialGlow>,
                        */
                    });

                    material_cache
                        .insert(material_id, Some(LoadedMaterial::Static(handle.clone())));
                    LoadedMaterial::Static(handle)
                }
            });

            let mut collision_filter = COLLISION_FILTER_INSPECTABLE;

            if object_part.collision_shape.is_some() {
                collision_filter |= COLLISION_FILTER_COLLIDABLE;

                if collision_group != COLLISION_GROUP_ZONE_WARP_OBJECT
                    && !object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE)
                {
                    collision_filter |= COLLISION_FILTER_CLICKABLE;
                }
            }

            let mut part_commands = object_commands.spawn_bundle((
                part_object_type(ZoneObjectPart {
                    object_id,
                    mesh_path: zsc.meshes[mesh_id].path().to_string_lossy().into(),
                    // collision_shape.is_none(): cannot be hit with any raycast
                    // collision_shape.is_some(): can be hit with forward raycast
                    collision_shape: (&object_part.collision_shape).into(),
                    // collision_not_moveable: does not hit downwards ray cast, but can hit forwards ray cast
                    collision_not_moveable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_MOVEABLE),
                    // collision_not_pickable: can not be clicked on with mouse
                    collision_not_pickable: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_PICKABLE),
                    // collision_height_only: ?
                    collision_height_only: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::HEIGHT_ONLY),
                    // collision_no_camera: does not collide with camera
                    collision_no_camera: object_part
                        .collision_flags
                        .contains(ZscCollisionFlags::NOT_CAMERA_COLLISION),
                }),
                mesh.clone(),
                part_transform,
                GlobalTransform::default(),
                Visibility::default(),
                ComputedVisibility::default(),
            ));

            match material {
                LoadedMaterial::Static(handle) => {
                    part_commands.insert_bundle((handle, NotShadowReceiver {}))
                }
                LoadedMaterial::Standard(handle) => part_commands.insert(handle),
            };

            part_commands.with_children(|builder| {
                // Transform for collider must be absolute
                let collider_transform = object_transform * part_transform;
                builder.spawn_bundle((
                    ColliderParent::new(object_entity),
                    AsyncCollider::Mesh(mesh),
                    CollisionGroups::new(collision_group, collision_filter),
                    collider_transform,
                ));
            });

            let active_motion = object_part.animation_path.as_ref().map(|animation_path| {
                ActiveMotion::new_repeating(asset_server.load(animation_path.path()))
            });
            if let Some(active_motion) = active_motion {
                part_commands.insert(active_motion);
            }

            part_entities.push(part_commands.id());
        }
    });

    for object_effect in object.effects.iter() {
        let effect_transform = Transform::default()
            .with_translation(
                Vec3::new(
                    object_effect.position.x,
                    object_effect.position.z,
                    -object_effect.position.y,
                ) / 100.0,
            )
            .with_rotation(Quat::from_xyzw(
                object_effect.rotation.x,
                object_effect.rotation.z,
                -object_effect.rotation.y,
                object_effect.rotation.w,
            ))
            .with_scale(Vec3::new(
                object_effect.scale.x,
                object_effect.scale.z,
                object_effect.scale.y,
            ));

        if let Some(effect_path) = zsc.effects.get(object_effect.effect_id as usize) {
            if let Some(effect_entity) = spawn_effect(
                &vfs_resource.vfs,
                commands,
                asset_server,
                particle_materials,
                effect_mesh_materials,
                effect_path.into(),
                false,
                None,
            ) {
                if let Some(parent_part_entity) = object_effect
                    .parent
                    .and_then(|parent_part_index| part_entities.get(parent_part_index as usize))
                {
                    commands
                        .entity(*parent_part_entity)
                        .add_child(effect_entity);
                } else {
                    commands.entity(object_entity).add_child(effect_entity);
                }

                commands.entity(effect_entity).insert(effect_transform);

                if matches!(object_effect.effect_type, ZscEffectType::DayNight) {
                    commands.entity(effect_entity).insert(NightTimeEffect);
                }
            }
        }
    }

    object_entity
}

fn load_animated_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    static_mesh_materials: &mut Assets<StaticMeshMaterial>,
    stb_morph_object: &StbFile,
    object_instance: &IfoObject,
) {
    let object_id = object_instance.object_id as usize;
    let mesh_path = stb_morph_object.get(object_id, 1);
    let motion_path = stb_morph_object.get(object_id, 2);
    let texture_path = stb_morph_object.get(object_id, 3);

    let alpha_enabled = stb_morph_object.get_int(object_id, 4) != 0;
    let two_sided = stb_morph_object.get_int(object_id, 5) != 0;
    let alpha_test_enabled = stb_morph_object.get_int(object_id, 6) != 0;
    let z_test_enabled = stb_morph_object.get_int(object_id, 7) != 0;
    let z_write_enabled = stb_morph_object.get_int(object_id, 8) != 0;

    // TODO: Animated object material blend op
    let _src_blend = stb_morph_object.get_int(object_id, 9);
    let _dst_blend = stb_morph_object.get_int(object_id, 10);
    let _blend_op = stb_morph_object.get_int(object_id, 11);

    let object_transform = Transform::default()
        .with_translation(
            Vec3::new(
                object_instance.position.x,
                object_instance.position.z,
                -object_instance.position.y,
            ) / 100.0
                + Vec3::new(5200.0, 0.0, -5200.0),
        )
        .with_rotation(Quat::from_xyzw(
            object_instance.rotation.x,
            object_instance.rotation.z,
            -object_instance.rotation.y,
            object_instance.rotation.w,
        ))
        .with_scale(Vec3::new(
            object_instance.scale.x,
            object_instance.scale.z,
            object_instance.scale.y,
        ));

    let mesh = asset_server.load::<Mesh, _>(mesh_path);
    let material = static_mesh_materials.add(StaticMeshMaterial {
        base_texture: Some(
            asset_server.load(RgbTextureLoader::convert_path(Path::new(texture_path))),
        ),
        lightmap_texture: None,
        alpha_value: None,
        alpha_enabled,
        alpha_test: if alpha_test_enabled { Some(0.5) } else { None },
        two_sided,
        z_write_enabled,
        z_test_enabled,
        specular_enabled: false,
        skinned: false,
        lightmap_uv_offset: Vec2::new(0.0, 0.0),
        lightmap_uv_scale: 1.0,
    });

    // TODO: Animation object morph targets, blocked by lack of bevy morph targets
    let mut entity_commands = commands.spawn_bundle((
        ZoneObject::AnimatedObject(ZoneObjectAnimatedObject {
            mesh_path: mesh_path.to_string(),
            motion_path: motion_path.to_string(),
            texture_path: texture_path.to_string(),
        }),
        mesh.clone(),
        material,
        object_transform,
        GlobalTransform::default(),
        Visibility::default(),
        ComputedVisibility::default(),
    ));
    let object_entity = entity_commands.id();

    entity_commands.with_children(|builder| {
        builder.spawn_bundle((
            ColliderParent::new(object_entity),
            AsyncCollider::Mesh(mesh),
            CollisionGroups::new(COLLISION_GROUP_ZONE_OBJECT, COLLISION_FILTER_INSPECTABLE),
            object_transform,
        ));
    });
}
