use std::{
    hash::{Hash, Hasher},
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::from_utf8,
    time::Duration,
};

use dns_parser::{Builder, Packet, QueryClass, QueryType, RData, ResourceRecord};
use iced::futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    stream::BoxStream,
};
use log::error;
use tokio::{net::UdpSocket, spawn, time::timeout};

#[derive(Clone, Debug)]
pub struct Server {
    pub address: SocketAddr,
    pub protocol: String,
    pub authentication: Authentication,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Copy, Default)]
pub struct LocalServerScanner(u64);

#[derive(Clone, Debug)]
pub enum Authentication {
    None,
}

impl Server {
    fn try_from(packet: &Packet) -> Option<Self> {
        let ip = Self::records(packet).find_map(|record| match record.data {
            RData::A(record) => Some(record.0.into()),
            RData::AAAA(record) => Some(record.0.into()),
            _ => None,
        })?;
        let port = Self::records(packet).find_map(|record| match record.data {
            RData::SRV(record) => Some(record.port),
            _ => None,
        })?;
        let (protocol, authentication, name, description) =
            Self::records(packet).find_map(|record| match &record.data {
                RData::TXT(record) => {
                    let mut iter = record.iter();
                    let protocol = from_utf8(iter.next()?).ok()?.to_owned();
                    let authentication = Authentication::try_parse(from_utf8(iter.next()?).ok()?)?;
                    let name = from_utf8(iter.next()?).ok()?.to_owned();
                    let description = from_utf8(iter.next()?).ok()?.to_owned();
                    Some((protocol, authentication, name, description))
                }
                _ => None,
            })?;
        let address = SocketAddr::new(ip, port);
        Some(Self {
            address,
            protocol,
            authentication,
            name,
            description,
        })
    }

    pub fn records<'a>(packet: &'a Packet) -> impl Iterator<Item = &'a ResourceRecord<'a>> {
        packet
            .answers
            .iter()
            .chain(packet.nameservers.iter())
            .chain(packet.additional.iter())
    }
}

impl Authentication {
    pub fn try_parse(text: &str) -> Option<Self> {
        match text {
            "none" => Some(Self::None),
            _ => None,
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            Authentication::None => true,
        }
    }
}

impl LocalServerScanner {
    pub fn rescan(&mut self) {
        self.0 += 1;
    }

    async fn scan(mut send: UnboundedSender<Server>) -> io::Result<()> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;
        let mut builder = Builder::new_query(0, false);
        builder.add_question(
            "_wosim-server._udp.local",
            true,
            QueryType::PTR,
            QueryClass::IN,
        );
        let buf = builder.build().unwrap();
        socket
            .send_to(&buf, SocketAddrV4::new(Ipv4Addr::new(224, 0, 0, 251), 5353))
            .await?;
        timeout(Duration::from_secs(3), async move {
            loop {
                let mut buf = [0; 4096];
                let (size, _) = socket.recv_from(&mut buf).await.unwrap();
                if let Ok(packet) = dns_parser::Packet::parse(&buf[0..size]) {
                    if let Some(entry) = Server::try_from(&packet) {
                        if let Err(error) = send.start_send(entry) {
                            error!("{}", error)
                        }
                    };
                }
            }
        })
        .await
        .unwrap_err();
        Ok(())
    }
}

impl<H: Hasher, I> iced_native::subscription::Recipe<H, I> for LocalServerScanner {
    type Output = Server;

    fn hash(&self, state: &mut H) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.0.hash(state)
    }

    fn stream(self: Box<Self>, _input: BoxStream<I>) -> BoxStream<Self::Output> {
        Box::pin({
            let (send, recv) = unbounded();
            spawn(Self::scan(send));
            recv
        })
    }
}
