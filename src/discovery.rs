pub mod discovery {
    use std::{collections::HashMap, any::Any};
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use zeroconf::{prelude::TMdnsBrowser, MdnsBrowser, ServiceDiscovery, ServiceType};

    pub struct AirPlayDiscoverer {
        mdns_browser: MdnsBrowser,
        discovered_services: Vec<ServiceDiscovery>,
        rx_channel: mpsc::Receiver<ServiceDiscovery>
    }
    impl AirPlayDiscoverer {
        pub async fn new() -> Self {
            let (tx, rx) = mpsc::channel::<ServiceDiscovery>(0xff);
            let (ctx, crx) = std::sync::mpsc::channel::<bool>();
            //Initialize the MdnsBrowser and HashMap.
            let mut air_play_discoverer = AirPlayDiscoverer {
                mdns_browser: MdnsBrowser::new(
                    ServiceType::new("airplay", "tcp")
                        .expect("Failed to create MdnsBrowser!"),
                ),
                discovered_services: Vec::new(),
                rx_channel: rx
            };
            //Set the service_discovered_callback to a closure.
            air_play_discoverer
                .mdns_browser
                .set_service_discovered_callback(Box::new(move |result: zeroconf::Result<ServiceDiscovery>, _context: Option<Arc<dyn Any>>|{
                    //If the result is Ok add an Entry to the HashMap, with the kv-pair <SERVICE_NAME>:<SERVICE_DISCOVERY>.
                    if let Ok(disc) = result {
                        tx
                            .blocking_send(disc)
                            .expect("Failed to send ServiceDiscovery trough mpsc-channel!");
                    }
                }));
            air_play_discoverer
        }
    }
}
