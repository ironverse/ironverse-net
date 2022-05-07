use bevy::tasks::Task;
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use ironverse::BOOTSTRAP_ADDRESSES;
use ironverse::libp2p::gossipsub::GossipsubMessage;
use ironverse::{
    Client, NetAction, NetEvent,
    tokio::sync::mpsc::error,
    libp2p::{
        swarm::SwarmEvent, 
        gossipsub::{
            GossipsubEvent, 
            IdentTopic as Topic, 
            error::GossipsubHandlerError
        }
    }
};
use crate::ui::ChatEvent;

#[derive(Default)]
pub struct NetResource {
    pub started: bool,
    pub client: Option<Client>,
    pub task: Option<Task<()>>,
}

const TOPIC: &str = "test-topic";

pub fn write_network(mut local: ResMut<NetResource>, mut chat_reader: EventReader<ChatEvent>) {
    for event in chat_reader.iter() {
        if event.is_outgoing {
            match local.client.as_mut() {
                None => info!("no client! message not sent: {:?}", event),
                Some(client) => {
                    if event.message.starts_with("/name") {
                        if let Err(e) = client.to_net_sender.blocking_send(NetAction::Publish(Topic::new(TOPIC), event.message.clone().into())) {
                            info!("{:?}", e);
                        }
                    } else if event.message.starts_with("/dial") {
                        let parts: Vec<&str> = event.message.split(" ").collect();
                        if parts.len() == 2 {
                            match parts[1].parse() {
                                Ok(addr) => if let Err(e) = client.to_net_sender.blocking_send(NetAction::Dial(addr)) { info!("{:?}", e); },
                                Err(e) => info!("{:?}", e)
                            }
                        }
                    } else if event.message.starts_with("/sub") {
                        let parts: Vec<&str> = event.message.split(" ").collect();
                        if parts.len() == 2 {
                            if let Err(e) = client.to_net_sender.blocking_send(NetAction::Subscribe(Topic::new(parts[1]))) {
                                info!("{:?}", e);
                            };
                        }
                    } else if event.message.starts_with("/unsub") {
                        let parts: Vec<&str> = event.message.split(" ").collect();
                        if parts.len() == 2 {
                            if let Err(e) = client.to_net_sender.blocking_send(NetAction::Unsubscribe(Topic::new(parts[1]))) {
                                info!("{:?}", e);
                            };
                        }
                    } else {
                        if let Err(e) = client.to_net_sender.blocking_send(NetAction::Publish(Topic::new(TOPIC), event.message.clone().into())) {
                            info!("{:?}", e);
                        }
                    };
                }
            }
        }
    }
}

pub fn read_network(mut local: ResMut<NetResource>, thread_pool: Res<AsyncComputeTaskPool>, mut chat_writer: EventWriter<ChatEvent>) {
    if !local.started {
        local.started = true;

        let (client, config) = ironverse::setup(BOOTSTRAP_ADDRESSES.len() + 1);
        chat_writer.send(ChatEvent::new_system_msg(&format!("{:?}", config.local_peer_id)));
        
        local.task = Some(thread_pool.spawn(async move {
            ironverse::start_swarm(config).await.unwrap();
        }));
        for addr in BOOTSTRAP_ADDRESSES {
            let addr = addr.parse().unwrap();
            if let Err(e) = client.to_net_sender.blocking_send(NetAction::Dial(addr)) {
                chat_writer.send(ChatEvent::new_system_msg(&format!("{:?}", e)));
            }
        }
        client.to_net_sender.blocking_send(NetAction::Subscribe(Topic::new(TOPIC))).unwrap();
        local.client = Some(client);
    }
    
    match local.client.as_mut() {
        None => {},
        Some(client) => {
            match client.from_net_receiver.try_recv() {
                Err(e) => {
                    match e {
                        error::TryRecvError::Empty => {},
                        _ => chat_writer.send(ChatEvent::new_system_msg(&format!("{:?}", e)))
                    }
                }
                Ok(event) => {
                    match event {
                        NetEvent::SwarmEvent(event) => match_swarm_events(event, chat_writer),
                        _ => chat_writer.send(ChatEvent::new_system_msg(&format!("{:?}", event))),
                    }
                    ;
                }
            }
        }
    }
}

fn match_swarm_events(event: SwarmEvent<GossipsubEvent, GossipsubHandlerError>, mut chat_writer: EventWriter<ChatEvent>) {
    match event {
        SwarmEvent::Behaviour(GossipsubEvent::Message {
            propagation_source: peer_id,
            message_id: _,
            message,
        }) => {
            chat_writer.send(parse_message(peer_id.to_string(), message));
        },
        SwarmEvent::NewListenAddr { address, .. } => {
            chat_writer.send(ChatEvent::new_system_msg(&format!("Listening on {:?}", address)));
        },
        SwarmEvent::ConnectionEstablished { peer_id, endpoint: _, num_established: _, concurrent_dial_errors: _ } => {
            chat_writer.send(ChatEvent::new_system_msg(&format!("ConnectionEstablished with {:?}", peer_id)));
        }
        _ => {}
    }
}

fn parse_message(peer_id: String, message: GossipsubMessage) -> ChatEvent {
    let message = String::from_utf8_lossy(&message.data).to_string();
    ChatEvent { 
        is_outgoing: false, 
        sender: peer_id, 
        message: message,
    }
}