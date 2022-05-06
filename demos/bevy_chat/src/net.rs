use bevy::tasks::Task;
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use ironverse::{
    Client, NetAction, 
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

#[derive(Default)]
pub struct NetworkResource {
    pub started: bool,
    pub client: Option<Client>,
    pub task: Option<Task<()>>,
}

pub fn network(mut res: Local<NetworkResource>, thread_pool: Res<AsyncComputeTaskPool>) {
    if !res.started {
        res.started = true;

        let (client, config) = ironverse::setup();
        res.client = Some(client);
        info!("Local peer id: {:?}", config.local_peer_id);

        res.task = Some(thread_pool.spawn(async move {
            ironverse::start_swarm(config).await.unwrap();
        }));
    }
    
    match res.client.as_mut() {
        None => {},
        Some(client) => {
            match client.from_net_receiver.try_recv() {
                Err(e) => {
                    match e {
                        error::TryRecvError::Empty => {},
                        _ => info!("net receiver: {:?}", e)
                    }
                }
                Ok(event) => {
                    match_net_events(client, event);
                }
            }
        }
    }
}

fn match_net_events(client: &mut Client, event: SwarmEvent<GossipsubEvent, GossipsubHandlerError>) {
    match event {
        SwarmEvent::Behaviour(GossipsubEvent::Message {
            propagation_source: peer_id,
            message_id: id,
            message,
        }) => {
            info!(
                "Got message: {} with id: {} from peer: {:?}",
                String::from_utf8_lossy(&message.data),
                id,
                peer_id
            );
        },
        SwarmEvent::NewListenAddr { address, .. } => {
            info!("Listening on {:?}", address);
        },
        SwarmEvent::ConnectionEstablished { peer_id, endpoint, num_established, concurrent_dial_errors } => {
            info!("ConnectionEstablished with {:?}", peer_id);    
            if let Some(_) = concurrent_dial_errors {
                let topic = Topic::new("test-topic");
                client.to_net_sender.blocking_send(NetAction::Publish(topic, format!("hello {}", peer_id).into()));
            }
        }
        _ => {}
    }
}