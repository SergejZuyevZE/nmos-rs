mod event_handler;
mod mdns;

use std::thread;
use std::time::Duration;
use std::{collections::BinaryHeap, sync::Arc};

use axum::http::Method;
use axum::Server;
pub use event_handler::EventHandler;
use mdns::MdnsContext;
use nmos_rs_model::{resource, Model};
use tokio::sync::mpsc;
use tower::make::Shared;
use tower::ServiceBuilder;
use tower_http::cors::{self, CorsLayer};
use tracing::info;

use crate::{
    error::Result,
    node::mdns::{NmosMdnsConfig, NmosMdnsEvent, NmosMdnsRegistry},
    service::NmosService,
};

pub struct NodeBuilder {
    event_handler: Option<Arc<dyn EventHandler>>,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Self {
            event_handler: None,
        }
    }

    pub fn event_handler<H: EventHandler + 'static>(mut self, event_handler: H) -> Self {
        self.event_handler = Some(Arc::new(event_handler));

        self
    }

    pub async fn build(self) -> Node {
        // Create nmos model
        let mut model = Model::new();

        // Create new node
        let node = resource::NodeBuilder::new("Test").build();
        let device = resource::DeviceBuilder::new(&node, "devicetype").build();
        let receiver = resource::ReceiverBuilder::new(
            &device,
            resource::Format::Video,
            resource::Transport::RtpMulticast,
        )
        .build();

        model.insert_node(node).await;
        model.insert_device(device).await;
        model.insert_receiver(receiver).await;

        // Wrap model in Arc
        let model = Arc::new(model);

        // Make service
        let service = NmosService::new(model.clone());

        Node {
            event_handler: self.event_handler,
            model,
            service,
        }
    }
}

pub struct Node {
    event_handler: Option<Arc<dyn EventHandler>>,
    model: Arc<Model>,
    service: NmosService,
}

impl Node {
    pub fn builder() -> NodeBuilder {
        NodeBuilder::new()
    }

    pub async fn start(self) -> Result<()> {
        info!("Starting nmos-rs node");

        // Channel for receiving MDNS events
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mdns_thread = thread::spawn(move || {
            // Create context
            let mut context = MdnsContext::new(&NmosMdnsConfig {}, tx);

            let poller = context.start();

            loop {
                // Poll every 100 ms
                poller.poll();
                thread::sleep(Duration::from_millis(100));
            }
        });

        let mut registries = BinaryHeap::new();

        let mdns_receiver = async {
            while let Some(event) = rx.recv().await {
                if let NmosMdnsEvent::Discovery(_, Ok(discovery)) = event {
                    let mdns_registry = NmosMdnsRegistry::parse(&discovery);
                    registries.push(mdns_registry);
                }
            }
        };

        // Create server
        let app = ServiceBuilder::new()
            .layer(
                CorsLayer::new()
                    .allow_methods([Method::GET, Method::POST])
                    .allow_origin(cors::Any),
            )
            .service(self.service);

        let addr = ([0, 0, 0, 0], 3000).into();
        let server = Server::bind(&addr).serve(Shared::new(app));

        tokio::select! {
            _ = mdns_receiver => {}
            _ = server => {}
        };

        Ok(())
    }
}
