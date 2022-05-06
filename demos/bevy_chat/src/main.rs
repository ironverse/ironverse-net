use bevy::{prelude::*};

mod net;
mod scene;
mod ui;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(scene::setup)
        .add_system(net::network)
        .add_system(ui::window)
        .run();
}