use futures::{prelude::*};
use tokio::sync::mpsc;
use std::{
    collections::hash_map::DefaultHasher, error::Error, time::Duration, hash::{Hash, Hasher}
};
use libp2p::{
    identity, Multiaddr, PeerId, swarm::SwarmEvent, 
    gossipsub::{
        self, GossipsubMessage, MessageAuthenticity, ValidationMode, MessageId, GossipsubEvent, error::GossipsubHandlerError, IdentTopic as Topic
    }
};

pub use libp2p;
pub use tokio;

pub enum NetAction {
    Dial(Multiaddr),
    Subscribe(Topic),
    Unsubscribe(Topic),
    Publish(Topic, Vec<u8>),
}
pub struct Client {
    pub to_net_sender: mpsc::Sender<NetAction>,
    pub from_net_receiver: mpsc::Receiver<SwarmEvent<GossipsubEvent, GossipsubHandlerError>>,
}

pub struct SwarmConfig {
    pub from_net_sender: mpsc::Sender<SwarmEvent<GossipsubEvent, GossipsubHandlerError>>,
    pub to_net_receiver: mpsc::Receiver<NetAction>,
    pub local_key: identity::Keypair,
    pub local_peer_id: PeerId,
}

pub const BOOTSTRAP_ADDRESSES: &[&str] = &[
    "/ip4/0.0.0.0/tcp/0"
];

pub fn setup() -> (Client, SwarmConfig) {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let (to_net_sender, to_net_receiver) = mpsc::channel(1);
    let (from_net_sender, from_net_receiver) = mpsc::channel(1);

    (Client{
        to_net_sender,
        from_net_receiver,
    }, 
    SwarmConfig{
        from_net_sender,
        to_net_receiver,
        local_key,
        local_peer_id,
    })
}

pub async fn start_swarm(mut config: SwarmConfig) -> Result<(), Box<dyn Error>> {
    // Set up an encrypted TCP Transport over the Mplex and Yamux protocols
    //TODO: Replace with WebRTC transport
    let transport = libp2p::development_transport(config.local_key.clone()).await?;
    
    // Create a Swarm to manage peers and events
    let mut swarm = {
        // To content-address message, we can take the hash of message and use it as an ID.
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        // Set a custom gossipsub
        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the
            // same content will be propagated.
            .build()
            .expect("Valid config");
        // build a gossipsub network behaviour
        let gossipsub: gossipsub::Gossipsub =
            gossipsub::Gossipsub::new(MessageAuthenticity::Signed(config.local_key), gossipsub_config)
                .expect("Correct configuration");

        // build the swarm
        libp2p::Swarm::new(transport, gossipsub, config.local_peer_id)
    };

    // Listen on all interfaces and whatever port the OS assigns
    swarm
    .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
    .unwrap();

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                config.from_net_sender.send(event).await;
            },
            to_net = config.to_net_receiver.recv() => {
                if let Some(netsend) = to_net { 
                    match netsend {
                        NetAction::Dial(address) => {
                            swarm.dial(address);
                        },
                        NetAction::Subscribe(topic) => {
                            swarm.behaviour_mut().subscribe(&topic);
                        },
                        NetAction::Unsubscribe(topic) => {
                            swarm.behaviour_mut().subscribe(&topic);
                        },
                        NetAction::Publish(topic, msg) => {
                            swarm.behaviour_mut().publish(topic, msg);
                        },
                    }
                }

            }
        }
    }
}