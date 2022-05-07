use futures::{prelude::*};
use tokio::sync::mpsc;
use std::{
    error::Error, time::Duration
};
use libp2p::{
    identity, Multiaddr, PeerId, swarm::{SwarmEvent, DialError}, 
    gossipsub::{
        self, MessageAuthenticity, ValidationMode, GossipsubEvent, error::{GossipsubHandlerError, SubscriptionError, PublishError}, IdentTopic as Topic
    }
};

pub use libp2p;
pub use tokio;

#[derive(Debug)]
pub enum NetAction {
    Dial(Multiaddr),
    Subscribe(Topic),
    Unsubscribe(Topic),
    Publish(Topic, Vec<u8>),
}
#[derive(Debug)]
pub enum NetEvent {
    SwarmEvent(SwarmEvent<GossipsubEvent, GossipsubHandlerError>),
    SubscriptionError(SubscriptionError),
    PublishError(PublishError),
    DialError(DialError),
}
pub struct Client {
    pub to_net_sender: mpsc::Sender<NetAction>,
    pub from_net_receiver: mpsc::Receiver<NetEvent>,
}

pub struct SwarmConfig {
    pub from_net_sender: mpsc::Sender<NetEvent>,
    pub to_net_receiver: mpsc::Receiver<NetAction>,
    pub local_key: identity::Keypair,
    pub local_peer_id: PeerId,
}

pub const BOOTSTRAP_ADDRESSES: &[&str] = &[
    "/dns/1.node.ironverse.net/tcp/8000",
    "/dns/2.node.ironverse.net/tcp/8000",
    "/dns/3.node.ironverse.net/tcp/8000",
    "/dns/4.node.ironverse.net/tcp/8000",
    "/dns/5.node.ironverse.net/tcp/8000",
];

pub fn setup(channel_buffer: usize) -> (Client, SwarmConfig) {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let (to_net_sender, to_net_receiver) = mpsc::channel(channel_buffer);
    let (from_net_sender, from_net_receiver) = mpsc::channel(channel_buffer);

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
        // Set a custom gossipsub
        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .build()?;
        // build a gossipsub network behaviour
        let gossipsub: gossipsub::Gossipsub = gossipsub::Gossipsub::new(MessageAuthenticity::Signed(config.local_key), gossipsub_config)?;
        // build the swarm
        libp2p::Swarm::new(transport, gossipsub, config.local_peer_id)
    };
    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                config.from_net_sender.send(NetEvent::SwarmEvent(event)).await?;
            },
            to_net = config.to_net_receiver.recv() => {
                if let Some(netsend) = to_net { 
                    match netsend {
                        NetAction::Dial(address) => {
                            if let Err(e) = swarm.dial(address.clone()) {
                                config.from_net_sender.send(NetEvent::DialError(e)).await?;
                            }
                        },
                        NetAction::Subscribe(topic) => {
                            if let Err(e) = swarm.behaviour_mut().subscribe(&topic) {
                                config.from_net_sender.send(NetEvent::SubscriptionError(e)).await?;
                            }
                        },
                        NetAction::Unsubscribe(topic) => {
                            if let Err(e) = swarm.behaviour_mut().unsubscribe(&topic) {
                                config.from_net_sender.send(NetEvent::PublishError(e)).await?;
                            }
                        },
                        NetAction::Publish(topic, msg) => {
                            if let Err(e) = swarm.behaviour_mut().publish(topic, msg) {
                                config.from_net_sender.send(NetEvent::PublishError(e)).await?;
                            }
                        },
                    }
                }

            }
        }
    }
}