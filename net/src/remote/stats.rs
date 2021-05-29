pub use quinn_proto::ConnectionStats;

#[derive(Default)]
pub struct ConnectionStatsDiff {
    pub tx: UdpStatsDiff,
    pub rx: UdpStatsDiff,
}

impl ConnectionStatsDiff {
    pub fn new(from: ConnectionStats, to: ConnectionStats) -> Self {
        Self {
            tx: UdpStatsDiff {
                datagrams: to.udp_tx.datagrams - from.udp_tx.datagrams,
                bytes: to.udp_tx.bytes - from.udp_tx.bytes,
                transmits: to.udp_tx.transmits - from.udp_tx.transmits,
            },
            rx: UdpStatsDiff {
                datagrams: to.udp_rx.datagrams - from.udp_rx.datagrams,
                bytes: to.udp_rx.bytes - from.udp_rx.bytes,
                transmits: to.udp_rx.transmits - from.udp_rx.transmits,
            },
        }
    }
}

#[derive(Default)]
pub struct UdpStatsDiff {
    pub datagrams: u64,
    pub bytes: u64,
    pub transmits: u64,
}
