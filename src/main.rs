use bevy::{
    core::FixedTimestep,
    core::Time,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    input::Input,
    math::Vec3,
    math::Vec3Swizzles,
    prelude::*,
    render::camera::Camera,
    render::camera::OrthographicCameraBundle,
    render::render_resource::TextureUsages,
    window::WindowDescriptor,
};
use bevy_ecs_tilemap::prelude::*;
use noise::{Fbm, MultiFractal, NoiseFn, Seedable};

// A simple camera system for moving and zooming the camera.
pub fn movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut OrthographicProjection), With<Camera>>,
) {
    for (mut transform, mut ortho) in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::A) {
            direction -= Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::D) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::W) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::S) {
            direction -= Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Z) {
            ortho.scale += 0.1;
        }

        if keyboard_input.pressed(KeyCode::X) {
            ortho.scale -= 0.1;
        }

        if ortho.scale < 0.5 {
            ortho.scale = 0.5;
        }

        let z = transform.translation.z;
        transform.translation += time.delta_seconds() * direction * 500.;
        // Important! We need to restore the Z values when moving the camera around.
        // Bevy has a specific camera setup and this can mess with how our layers are shown.
        transform.translation.z = z;
    }
}

#[allow(dead_code)]
#[derive(Component)]
pub struct Player;

// A simple camera system for moving and zooming the camera.
#[allow(dead_code)]
pub fn update(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
    mut map_query: MapQuery,
) {
    for mut transform in query.iter_mut() {
        let mut direction = Vec3::ZERO;

        if keyboard_input.pressed(KeyCode::Left) {
            direction -= Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Right) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Up) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Down) {
            direction -= Vec3::new(0.0, 1.0, 0.0);
        }

        transform.translation += time.delta_seconds() * direction * 50.;

        let sprite_pos_z =
            map_query.get_zindex_for_pixel_pos(transform.translation.xy().extend(1.0), 0u16, 0u16);

        dbg!(sprite_pos_z);
        transform.translation.z = sprite_pos_z;
    }
}

pub fn set_texture_filters_to_nearest(
    mut texture_events: EventReader<AssetEvent<Image>>,
    mut textures: ResMut<Assets<Image>>,
) {
    // quick and dirty, run this for all textures anytime a texture is created.
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(mut texture) = textures.get_mut(handle) {
                    texture.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST;
                }
            }
            _ => (),
        }
    }
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>, mut map_query: MapQuery) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    let texture_handle = asset_server.load("tiles.png");

    // Create map entity and component:
    let map_entity = commands.spawn().id();
    let mut map = Map::new(0u16, map_entity);

    let layer_settings = LayerSettings::new(
        MapSize(2, 2),
        ChunkSize(64, 64),
        TileSize(16.0, 16.0),
        TextureSize(96.0, 16.0),
    );

    let center = layer_settings.get_pixel_center();

    // Chunk sizes of 64x64 seem optimal for meshing updates.
    let (mut layer_builder, layer_entity) =
        LayerBuilder::<TileBundle>::new(&mut commands, layer_settings, 0u16, 0u16);
    map.add_layer(&mut commands, 0u16, layer_entity);

    layer_builder.for_each_tiles_mut(|tile_entity, tile_data| {
        // True here refers to tile visibility.
        *tile_data = Some(TileBundle {
            tile: Tile {
                texture_index: 1,
                ..Tile::default()
            },
            ..TileBundle::default()
        });
        // Tile entity might not exist at this point so you'll need to create it.
        if tile_entity.is_none() {
            *tile_entity = Some(commands.spawn().id());
        }
    });

    map_query.build_layer(&mut commands, layer_builder, texture_handle);

    // Spawn Map
    // Required in order to use map_query to retrieve layers/tiles.
    commands
        .entity(map_entity)
        .insert(map)
        .insert(Transform::from_xyz(-center.x, -center.y, 0.0))
        .insert(GlobalTransform::default());
}

fn get_island_shape(x: f64, y: f64) -> f64 {
    let a = 1.0;
    let b = 1.2;
    let value = x.abs().max(y.abs());

    value.powf(a) / value.powf(a) + (b - b * value).powf(a)
}
// In this example it's better not to use the default `MapQuery` SystemParam as
// it's faster to do it this way:
fn random(mut map_query: MapQuery, mut tile_query: Query<&mut Tile>) {
    // Generate a seed for the map

    let seed: u32 = fastrand::u32(..);
    fastrand::seed(seed as u64);
    // Create fbm noise
    let mut fbm = Fbm::new();
    fbm = fbm.set_seed(seed);
    fbm = fbm.set_frequency(0.2);
    if let Some(ground_layer) = map_query.get_layer(0u16, 0u16) {
        let chunk_width = ground_layer.1.settings.chunk_size.0;
        let chunk_height = ground_layer.1.settings.chunk_size.1;
        let map_width = ground_layer.1.settings.map_size.0;
        let map_height = ground_layer.1.settings.map_size.1;

        let actual_width = map_width * chunk_width;
        let actual_height = map_height * chunk_height;

        let half_actual_width = actual_width / 2;
        let half_actual_height = actual_height / 2;

        for x in 0..actual_width {
            for y in 0..actual_height {
                let high_x = x as f64 - half_actual_width as f64;
                let high_y = y as f64 - half_actual_height as f64;
                let mask = get_island_shape(high_x / 60.0, high_y / 60.0);
                let noise_value = fbm.get([high_x / 15.0, high_y / 15.0]) - (1.0 - mask);

                if let Ok(tile_c) = map_query.get_tile_entity(TilePos(x, y), 0u16, 0u16) {
                    if let Ok(mut tile) = tile_query.get_mut(tile_c) {
                        if noise_value > 0.0 {
                            if noise_value > 0.9 {
                                tile.texture_index = 4;
                            } else if noise_value > 0.7 {
                                tile.texture_index = 3; // Rock 2
                            } else if noise_value > 0.6 {
                                tile.texture_index = 2; // Rock 1
                            } else if noise_value > 0.4 {
                                tile.texture_index = 1; // Forest
                            } else {
                                tile.texture_index = 1; // Grass
                            }
                        }
                        let _ = map_query.notify_chunk_for_tile(TilePos(x, y), 0u16, 0u16);
                    }
                }
            }
        }
    }
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 1270.0,
            height: 720.0,
            title: String::from("Random Island Generator"),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(TilemapPlugin)
        .add_startup_system(startup.chain(random))
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(2.00))
                .with_system(random),
        )
        .add_system(movement)
        .add_system(set_texture_filters_to_nearest)
        .run();
}
