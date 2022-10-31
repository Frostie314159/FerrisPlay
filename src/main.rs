use bitflags::bitflags;
use console::Term;
use eui48::MacAddress;
use futures::task::noop_waker_ref;
use std::{any::Any, net::IpAddr, sync::Arc, task::Context, time::Duration, error::Error};
use tokio::sync::mpsc;
use zeroconf::{prelude::*, MdnsBrowser, ServiceDiscovery, ServiceType, linux::txt_record::AvahiTxtRecord};

const AIR_PLAY_SERVICE_NAME: &str = "airplay";
const AIR_PLAY_SERVICE_PROTOCOL: &str = "tcp";

bitflags! {
    struct AirPlayServiceFeatures: u16 {
        const VIDEO = 0x1;
        const PHOTO = 0x2;
        const VIDEO_FAIR_PLAY = 0x4;
        const VIDEO_VOLUME_CONTROL = 0x8;
        const VIDEO_HTTP_LIVE_STREAMS = 0x10;
        const SLIDESHOW = 0x20;
        const SCREEN = 0x40;
        const SCREEN_ROTATE = 0x80;
        const AUDIO = 0x100;
        const AUDIO_REDUNDANT = 0x200;
        const FPSAPV2PT5_AES_GCM = 0x400;
        const PHOTO_CACHING = 0x800;
    }
}
struct AirPlayService {
    name: String,
    address: IpAddr,
    device_id: MacAddress,
    features: AirPlayServiceFeatures,
    model: String,
}
impl AirPlayService {
    fn from_service_discovery(value: ServiceDiscovery) -> Option<Self> {
        let txt_record = value
            .txt()
            .clone()?;
        Some(AirPlayService {
            name: value.name().to_string(),
            address: value
                .address()
                .parse()
                .ok()?,
            device_id: txt_record
                .get("deviceid")?
                .parse()
                .ok()?,
            features: AirPlayServiceFeatures::from_bits(
                u16::from_str_radix(
                    txt_record
                        .get("features")?
                        .split_at(2)
                        .1,
                    16,
                )
                .ok()?,
            )
            .unwrap_or(AirPlayServiceFeatures::AUDIO),
            model: txt_record
                .get("model")?,
        })
    }
}

fn init_discoverer() -> Result<(MdnsBrowser, mpsc::Receiver<ServiceDiscovery>), zeroconf::error::Error> {
    let mut mdns_browser = MdnsBrowser::new(ServiceType::new(
        AIR_PLAY_SERVICE_NAME,
        AIR_PLAY_SERVICE_PROTOCOL,
    )?);
    let (tx, rx) = mpsc::channel(0xff);
    mdns_browser.set_context(Box::new(tx));
    let on_service_discover = move |result: zeroconf::Result<ServiceDiscovery>,
                                    context: Option<Arc<dyn Any>>| {
        let ctx = context.expect("No context was provided");
        let tx = ctx
            .downcast_ref::<mpsc::Sender<ServiceDiscovery>>()
            .expect("Failed to downcast!");
        if let Ok(disc) = result {
            tx.blocking_send(disc)
                .expect("Failed to send ServiceRegistration!");
        }
    };
    mdns_browser.set_service_discovered_callback(Box::new(on_service_discover));
    Ok((mdns_browser, rx))
}

fn print_services(services: &Vec<AirPlayService>) {
    for i in 0..services.len() {
        println!("{}: {}", i, services[i].name);
    }
}
fn is_air_play_1(service: &ServiceDiscovery) -> bool {
    if let Some(txt) = service.txt() {
        !txt.contains_key("fv")
    } else {
        false
    }
}
fn discover_and_pick_service(term: &Term) {
    println!("Discovered AirPlay services: ");
    let (mut mdns_browser, mut rx) = init_discoverer().expect("Failed to initialize discovery!");
    let mut discovered_services: Vec<AirPlayService> = Vec::new();
    let ev_loop = mdns_browser
        .browse_services()
        .expect("Failed to create event loop!");
    loop {
        ev_loop
            .poll(Duration::from_secs(0))
            .expect("Failed to poll event loop!");
        match rx.poll_recv(&mut Context::from_waker(noop_waker_ref())) {
            std::task::Poll::Ready(Some(data)) => {
                println!("{:#?}", data);
                if is_air_play_1(&data) {
                    term.clear_last_lines(discovered_services.len()).expect("Failed to clear console lines!");
                    if let Some(service) = AirPlayService::from_service_discovery(data) {
                        discovered_services.push(service);
                    }
                    print_services(&discovered_services);
                }
            }
            std::task::Poll::Ready(None) => {
                panic!("Discovery thread crashed!");
            }
            std::task::Poll::Pending => {}
        }
    }
}

fn main() {
    let term = Term::stdout();
    println!("Welcome to FerrisPlay!");
    discover_and_pick_service(&term);
}
