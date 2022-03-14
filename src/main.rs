use bevy::{
    asset::AssetServerSettings,
    core_pipeline::ClearColor,
    prelude::{AddAsset, App, AssetServer, Color, Commands, Msaa, Res, SystemSet},
    render::{render_resource::WgpuFeatures, settings::WgpuSettings},
    window::WindowDescriptor,
};
use bevy_egui::EguiPlugin;
use bevy_flycam::{MovementSettings, NoCameraPlayerPlugin};
use bevy_mod_picking::{DebugCursorPickingPlugin, InteractablePickingPlugin, PickingPlugin};
use bevy_polyline::PolylinePlugin;
use std::{path::Path, sync::Arc, time::Duration};

mod bevy_flycam;
mod character_model;
mod components;
mod protocol;
mod render;
mod resources;
mod systems;
mod vfs_asset_io;
mod zms_asset_loader;

use rose_data::ZoneId;
use rose_file_readers::VfsIndex;

use character_model::CharacterModelList;
use render::RoseRenderPlugin;
use resources::{run_network_thread, AppState, LoadedZone, NetworkThread, NetworkThreadMessage};
use systems::{
    character_model_system, character_select_enter_system, character_select_exit_system,
    character_select_system, game_connection_system, game_state_enter_system, load_zone_system,
    login_connection_system, login_state_enter_system, login_state_exit_system, login_system,
    model_viewer_system, world_connection_system, zone_viewer_setup_system, zone_viewer_system,
};
use vfs_asset_io::VfsAssetIo;
use zms_asset_loader::ZmsAssetLoader;

pub struct VfsResource {
    vfs: Arc<VfsIndex>,
}

fn main() {
    let mut command = clap::Command::new("bevy_rose")
        .arg(
            clap::Arg::new("data-idx")
                .long("data-idx")
                .help("Path to data.idx")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("data-path")
                .long("data-path")
                .help("Optional path to extracted data, any files here override ones in data.idx")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("zone")
                .long("zone")
                .help("Runs as zone viewer, loading the specified zone")
                .takes_value(true),
        )
        .arg(
            clap::Arg::new("model-viewer")
                .long("model-viewer")
                .help("Run model viewer"),
        )
        .arg(clap::Arg::new("game").long("game").help("Run game"))
        .arg(
            clap::Arg::new("disable-vsync")
                .long("disable-vsync")
                .help("Disable v-sync to see accurate frame times"),
        );
    let data_path_error = command.error(
        clap::ErrorKind::ArgumentNotFound,
        "Must specify at least one of --data-idx or --data-path",
    );
    let matches = command.get_matches();

    let disable_vsync = matches.is_present("disable-vsync");
    let mut app_state = AppState::ZoneViewer;
    let view_zone_id = matches
        .value_of("zone")
        .and_then(|str| str.parse::<u16>().ok())
        .and_then(ZoneId::new)
        .unwrap_or_else(|| ZoneId::new(2).unwrap());
    if matches.is_present("game") {
        app_state = AppState::GameLogin;
    } else if matches.is_present("model-viewer") {
        app_state = AppState::ModelViewer;
    }

    let mut data_idx_path = matches.value_of("data-idx").map(Path::new);
    let data_extracted_path = matches.value_of("data-path").map(Path::new);

    if data_idx_path.is_none() && data_extracted_path.is_none() {
        if Path::new("data.idx").exists() {
            data_idx_path = Some(Path::new("data.idx"));
        } else {
            data_path_error.exit();
        }
    }

    let vfs = Arc::new(
        VfsIndex::with_paths(data_idx_path, data_extracted_path).expect("Failed to initialise VFS"),
    );

    let mut app = App::new();

    // Initialise bevy engine
    app.insert_resource(Msaa { samples: 4 })
        .insert_resource(AssetServerSettings {
            asset_folder: data_extracted_path
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "data".to_string()),
            watch_for_changes: false,
        })
        .insert_resource(WindowDescriptor {
            title: "Definitely not a ROSE client".to_string(),
            present_mode: if disable_vsync {
                bevy::window::PresentMode::Immediate
            } else {
                bevy::window::PresentMode::Fifo
            },
            width: 1920.0,
            height: 1080.0,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.70, 0.90, 1.0)))
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::TEXTURE_COMPRESSION_BC,
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin::default())
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::diagnostic::LogDiagnosticsPlugin {
            wait_duration: if disable_vsync {
                Duration::from_secs(5)
            } else {
                Duration::from_secs(30)
            },
            ..Default::default()
        })
        .add_plugin(bevy::diagnostic::FrameTimeDiagnosticsPlugin::default())
        .add_plugin(bevy::transform::TransformPlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default());

    let task_pool = app.world.resource::<bevy::tasks::IoTaskPool>().0.clone();
    app.insert_resource(VfsResource { vfs: vfs.clone() })
        .insert_resource(AssetServer::new(VfsAssetIo::new(vfs), task_pool))
        .add_plugin(bevy::asset::AssetPlugin::default());

    app.add_plugin(bevy::scene::ScenePlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::render::RenderPlugin::default())
        .add_plugin(bevy::core_pipeline::CorePipelinePlugin::default())
        .add_plugin(bevy::pbr::PbrPlugin::default());

    // Initialise 3rd party bevy plugins
    app.add_plugin(NoCameraPlayerPlugin)
        .insert_resource(MovementSettings {
            sensitivity: 0.00012,
            speed: 200.,
        })
        .add_plugin(PolylinePlugin)
        .add_plugin(PickingPlugin)
        .add_plugin(InteractablePickingPlugin)
        .add_plugin(DebugCursorPickingPlugin)
        .add_plugin(EguiPlugin);

    // Initialise rose stuff
    app.init_asset_loader::<ZmsAssetLoader>()
        .add_plugin(RoseRenderPlugin);

    app.add_system(load_zone_system)
        .add_system(character_model_system);

    // Setup state
    app.add_state(app_state);
    if matches!(app_state, AppState::ZoneViewer) {
        app.insert_resource(LoadedZone::with_next_zone(view_zone_id));
    } else {
        app.insert_resource(LoadedZone::default());
    }

    app.add_system_set(
        SystemSet::on_enter(AppState::ZoneViewer).with_system(zone_viewer_setup_system),
    )
    .add_system_set(SystemSet::on_update(AppState::ZoneViewer).with_system(zone_viewer_system));

    app.add_system_set(
        SystemSet::on_update(AppState::ModelViewer).with_system(model_viewer_system),
    );

    app.add_system_set(
        SystemSet::on_enter(AppState::GameLogin).with_system(login_state_enter_system),
    )
    .add_system_set(SystemSet::on_exit(AppState::GameLogin).with_system(login_state_exit_system))
    .add_system_set(SystemSet::on_update(AppState::GameLogin).with_system(login_system));

    app.add_system_set(
        SystemSet::on_enter(AppState::GameCharacterSelect)
            .with_system(character_select_enter_system),
    )
    .add_system_set(
        SystemSet::on_exit(AppState::GameCharacterSelect).with_system(character_select_exit_system),
    )
    .add_system_set(
        SystemSet::on_update(AppState::GameCharacterSelect).with_system(character_select_system),
    );

    app.add_system_set(SystemSet::on_enter(AppState::Game).with_system(game_state_enter_system));

    // Setup network
    let (network_thread_tx, network_thread_rx) =
        tokio::sync::mpsc::unbounded_channel::<NetworkThreadMessage>();
    let network_thread = std::thread::spawn(move || run_network_thread(network_thread_rx));
    app.insert_resource(NetworkThread::new(network_thread_tx.clone()))
        .add_system(login_connection_system)
        .add_system(world_connection_system)
        .add_system(game_connection_system);

    app.add_startup_system(load_resources);
    app.run();

    network_thread_tx.send(NetworkThreadMessage::Exit).ok();
    network_thread.join().ok();
}

fn load_resources(mut commands: Commands, vfs_resource: Res<VfsResource>) {
    commands.insert_resource(
        rose_data_irose::get_zone_list(&vfs_resource.vfs).expect("Failed to load zone list"),
    );

    commands.insert_resource(
        rose_data_irose::get_item_database(&vfs_resource.vfs)
            .expect("Failed to load item database"),
    );

    commands.insert_resource(
        CharacterModelList::new(&vfs_resource.vfs).expect("Failed to load character model list"),
    );
}
