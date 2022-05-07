use bevy::{prelude::*};

mod net;
mod scene;
mod ui;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(scene::setup)

        .add_event::<ui::ChatEvent>()
        .add_plugin(bevy_egui::EguiPlugin)
        .init_resource::<ui::ChatResource>()
        .add_system(ui::chat_event_reader)
        .add_system(ui::chat_window)
        
        .init_resource::<net::NetResource>()
        .add_system(net::read_network)
        .add_system(net::write_network)
        .run();
}

//TODO: loosely couple chat and net
//TODO: simplify code
//TODO: implement topic selection
//TODO: fix name change
//TODO: in bevy system read all messages from channel each frame instead of just one
