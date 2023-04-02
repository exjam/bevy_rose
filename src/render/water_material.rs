use std::marker::PhantomData;

use bevy::{
    asset::Handle,
    core_pipeline::core_3d::Transparent3d,
    ecs::{
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::{
        error, AddAsset, App, Assets, Commands, FromWorld, HandleUntyped, IntoSystemAppConfig,
        IntoSystemConfig, Mesh, Msaa, Plugin, Query, Res, ResMut, Resource, Time, World,
    },
    reflect::TypeUuid,
    render::{
        extract_component::ExtractComponentPlugin,
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::{
            encase, AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferBindingType,
            BufferDescriptor, BufferUsages, FilterMode, PipelineCache, RenderPipelineDescriptor,
            Sampler, SamplerBindingType, SamplerDescriptor, ShaderSize, ShaderStages, ShaderType,
            SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
            TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        view::{ExtractedView, VisibleEntities},
        Extract, ExtractSchedule, RenderApp, RenderSet,
    },
};

use crate::render::{
    zone_lighting::{SetZoneLightingBindGroup, ZoneLightingUniformMeta},
    TextureArray,
};

pub const WATER_MESH_MATERIAL_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0x333959e64b35d5d9);

#[derive(Default)]
pub struct WaterMaterialPlugin;

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        let mut shader_assets = app.world.resource_mut::<Assets<Shader>>();
        shader_assets.set_untracked(
            WATER_MESH_MATERIAL_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("shaders/water_material.wgsl")),
        );

        let render_device = app.world.resource::<RenderDevice>();
        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: WaterUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("water_texture_index"),
        });

        app.add_asset::<WaterMaterial>()
            .add_plugin(ExtractComponentPlugin::<Handle<WaterMaterial>>::default())
            .add_plugin(RenderAssetPlugin::<WaterMaterial>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<Transparent3d, DrawWaterMaterial>()
                .init_resource::<WaterMaterialPipeline>()
                .insert_resource(WaterUniformMeta { buffer })
                .init_resource::<SpecializedMeshPipelines<WaterMaterialPipeline>>()
                .add_system(extract_water_uniform_data.in_schedule(ExtractSchedule))
                .add_system(prepare_water_texture_index.in_set(RenderSet::Prepare))
                .add_system(queue_water_material_meshes.in_set(RenderSet::Queue));
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct WaterUniformData {
    pub current_index: i32,
    pub next_index: i32,
    pub next_weight: f32,
}

fn extract_water_uniform_data(mut commands: Commands, time: Extract<Res<Time>>) {
    let time = time.elapsed_seconds() * 10.0;
    let current_index = (time as i32) % 25;
    let next_index = (current_index + 1) % 25;
    let next_weight = time.fract();

    commands.insert_resource(WaterUniformData {
        current_index,
        next_index,
        next_weight,
    });
}

#[derive(Resource)]
pub struct WaterUniformMeta {
    buffer: Buffer,
}

fn prepare_water_texture_index(
    water_uniform_data: Res<WaterUniformData>,
    water_uniform_meta: ResMut<WaterUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; WaterUniformData::SHADER_SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(water_uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&water_uniform_meta.buffer, 0, buffer.as_ref());
}

#[derive(Resource)]
pub struct WaterMaterialPipeline {
    pub mesh_pipeline: MeshPipeline,
    pub material_layout: BindGroupLayout,
    pub zone_lighting_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    pub sampler: Sampler,
}

impl SpecializedMeshPipeline for WaterMaterialPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.fragment.as_mut().unwrap().targets[0]
            .as_mut()
            .unwrap()
            .blend = Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        });

        descriptor
            .depth_stencil
            .as_mut()
            .unwrap()
            .depth_write_enabled = false;

        descriptor.layout.insert(1, self.material_layout.clone());
        descriptor
            .layout
            .insert(3, self.zone_lighting_layout.clone());

        let vertex_layout = layout.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(descriptor)
    }
}

impl FromWorld for WaterMaterialPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // Water Texture Array
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Water Texture Sampler
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // Water Uniform Meta
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(WaterUniformData::min_size()),
                    },
                    count: None,
                },
            ],
            label: Some("water_material_layout"),
        });

        WaterMaterialPipeline {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            material_layout,
            zone_lighting_layout: world
                .resource::<ZoneLightingUniformMeta>()
                .bind_group_layout
                .clone(),
            vertex_shader: Some(WATER_MESH_MATERIAL_SHADER_HANDLE.typed()),
            fragment_shader: Some(WATER_MESH_MATERIAL_SHADER_HANDLE.typed()),
            sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..Default::default()
            }),
        }
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "e9e46dcc-94db-4b31-819f-d5ecffc732f0"]
pub struct WaterMaterial {
    pub water_texture_array: Handle<TextureArray>,
}

#[derive(Debug, Clone)]
pub struct GpuWaterMaterial {
    pub bind_group: BindGroup,
    pub water_texture_array: Handle<TextureArray>,
}

impl RenderAsset for WaterMaterial {
    type ExtractedAsset = WaterMaterial;
    type PreparedAsset = GpuWaterMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<WaterMaterialPipeline>,
        SRes<RenderAssets<TextureArray>>,
        SRes<WaterUniformMeta>,
    );

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        material: Self::ExtractedAsset,
        (
            render_device,
            material_pipeline,
            gpu_texture_arrays,
            water_uniform_meta,
        ): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let water_texture_gpu_image = gpu_texture_arrays.get(&material.water_texture_array);
        if water_texture_gpu_image.is_none() {
            return Err(PrepareAssetError::RetryNextUpdate(material));
        }
        let water_texture_view = &water_texture_gpu_image.unwrap().texture_view;
        let water_texture_sampler = &material_pipeline.sampler;

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                // Water Texture Array
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(water_texture_view),
                },
                // Water Texture Sampler
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(water_texture_sampler),
                },
                // Water Texture Index
                BindGroupEntry {
                    binding: 2,
                    resource: water_uniform_meta.buffer.as_entire_binding(),
                },
            ],
            label: Some("water_material_bind_group"),
            layout: &material_pipeline.material_layout,
        });

        Ok(GpuWaterMaterial {
            bind_group,
            water_texture_array: material.water_texture_array,
        })
    }
}

pub struct SetWaterMaterialBindGroup<const I: usize>(PhantomData<WaterMaterial>);
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetWaterMaterialBindGroup<I> {
    type Param = SRes<RenderAssets<WaterMaterial>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<WaterMaterial>>;

    fn render<'w>(
        _: &P,
        _: ROQueryItem<'w, Self::ViewWorldQuery>,
        material_handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = materials.into_inner().get(material_handle).unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

type DrawWaterMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetWaterMaterialBindGroup<1>,
    SetMeshBindGroup<2>,
    SetZoneLightingBindGroup<3>,
    DrawMesh,
);

#[allow(clippy::too_many_arguments)]
pub fn queue_water_material_meshes(
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    material_pipeline: Res<WaterMaterialPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<WaterMaterialPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderAssets<WaterMaterial>>,
    material_meshes: Query<(&Handle<WaterMaterial>, &Handle<Mesh>, &MeshUniform)>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<Transparent3d>,
    )>,
) {
    for (view, visible_entities, mut transparent_phase) in views.iter_mut() {
        let draw_transparent_pbr = transparent_draw_functions
            .read()
            .get_id::<DrawWaterMaterial>()
            .unwrap();

        let rangefinder = view.rangefinder3d();
        let view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        for visible_entity in &visible_entities.entities {
            if let Ok((material_handle, mesh_handle, mesh_uniform)) =
                material_meshes.get(*visible_entity)
            {
                if render_materials.contains_key(material_handle) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let mesh_key =
                            MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                                | MeshPipelineKey::BLEND_ALPHA
                                | view_key;

                        let pipeline_id = pipelines.specialize(
                            &pipeline_cache,
                            &material_pipeline,
                            mesh_key,
                            &mesh.layout,
                        );
                        let pipeline_id = match pipeline_id {
                            Ok(id) => id,
                            Err(err) => {
                                error!("{}", err);
                                continue;
                            }
                        };

                        let distance = rangefinder.distance(&mesh_uniform.transform);
                        transparent_phase.add(Transparent3d {
                            entity: *visible_entity,
                            draw_function: draw_transparent_pbr,
                            pipeline: pipeline_id,
                            distance,
                        });
                    }
                }
            }
        }
    }
}
