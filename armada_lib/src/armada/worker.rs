use crate::armada::tcp_ext::{TcpReceiverExt, TcpSenderExt};
use crate::armada::work::{ArmadaWork, ArmadaWorkMessage};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::transport::{
    transport_channel, TransportChannelType,
    TransportProtocol, TransportReceiver, TransportSender,
};
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};
use anyhow::Context;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, warn};
use twox_hash::XxHash64;

const BATCH_SEND_SIZE: usize = 32;
const BATCH_RECV_SIZE: usize = 32;
//const OPEN_PORT_REPORTING_SIZE: usize = 16;
const RATE_LIMIT_RESOLUTION: u64 = 100;
const MS_IN_SECOND: usize = 1_000;

pub(crate) struct ArmadaWorker {
    work_queue: UnboundedReceiver<ArmadaWork>,
}

impl ArmadaWorker {
    pub(crate) fn new(work_queue: UnboundedReceiver<ArmadaWork>) -> Self {
        Self { work_queue }
    }

    /// Runs the Armada worker, only processing (and sending) packets with the specified port
    pub(crate) fn run(mut self, port: u16) -> anyhow::Result<()> {
        // todo: increase buffer size
        let ipv4_protocol =
            TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Tcp));
        let ipv6_protocol =
            TransportChannelType::Layer4(TransportProtocol::Ipv6(IpNextHeaderProtocols::Tcp));

        let (mut ipv4_tcp_sender, mut ipv4_tcp_receiver) =
            transport_channel(1024 * 16, ipv4_protocol)
                .context("Error on raw socket initialization")?;

        let (mut ipv6_tcp_sender, mut ipv6_tcp_receiver) =
            transport_channel(1024 * 16, ipv6_protocol)
                .context("Error on raw socket initialization")?;

        let mut tcp_seq = rand::random::<u32>();

        while let Some(work) = self.work_queue.blocking_recv() {
            if let Err(e) = self.process_work(
                work,
                &mut ipv4_tcp_sender,
                &mut ipv4_tcp_receiver,
                &mut ipv6_tcp_sender,
                &mut ipv6_tcp_receiver,
                port,
                &mut tcp_seq,
            ) {
                error!(err = ?e, "scan failed");
            }
        }

        Ok(())
    }

    fn process_work(
        &self,
        work_unit: ArmadaWork,
        ipv4_tcp_sender: &mut TransportSender,
        ipv4_tcp_receiver: &mut TransportReceiver,
        ipv6_tcp_sender: &mut TransportSender,
        ipv6_tcp_receiver: &mut TransportReceiver,
        listening_port: u16,
        tcp_seq: &mut u32,
    ) -> anyhow::Result<()> {
        let ArmadaWork {
            mut remote_addrs,
            port_retries,
            port_timeout,
            packets_per_second,
            source_ipv4_addrs,
            source_ipv6_addrs,
            reporting_channel,
        } = work_unit;

        let mut requeued_addrs = Vec::with_capacity(1024 * 8);

        // results list
        let mut open_ports = Vec::new();
        let mut expiry_list = Vec::with_capacity(1024 * 8);
        let mut packet_retry_tracker =
            HashMap::<SocketAddr, u8, BuildHasherDefault<XxHash64>>::with_capacity_and_hasher(
                1024 * 16,
                Default::default(),
            );
        let mut inflight_addrs =
            HashSet::<SocketAddr, BuildHasherDefault<XxHash64>>::with_capacity_and_hasher(
                1024 * 16,
                Default::default(),
            );

        let mut source_ipv4_cycle = source_ipv4_addrs.iter().cycle();
        let mut source_ipv6_cycle = source_ipv6_addrs.iter().cycle();

        let mut current_packets_sent_for_bucket = 0usize;
        let mut next_packet_bucket_expiry = Instant::now()
            .checked_add(Duration::from_millis(RATE_LIMIT_RESOLUTION))
            .unwrap();

        let mut total_processed_ports = 0u128;
        let mut total_packets_sent = 0u128;

        'driver: loop {
            /*
               1. Send up-to some limit of pending addrs a syn packet
                   1. craft a tcp SYN packet
                   2. clone + swap remote with addr
                   3. send
                   4. add to inflight_addr
                   5. after batch, add batch to expiry_list
               2. Receive up-to some limit of packets
                   1. recv up-to some number of filtered (destination-bound) packets or when first terminated
                   2. check if indicative of response (syn-ack)
                   3. convert to socket-addr
                   4. remove from hashset and, if present, add to open_ports
               3. process expired addrs
                   1. peek expiry_list
                   2. if instant older than 'now', pop set off of expiry_list
                   3. for each addr, remove from hashset
                   4. if present in hashset, add to pending, otherwise discard
               4. if inflight is empty and pending is empty, return happily else loop
            */

            while requeued_addrs.len() < BATCH_SEND_SIZE * 16 {
                match remote_addrs.next() {
                    Some((addr, port)) => requeued_addrs.push(SocketAddr::new(addr, port)),
                    // if the iterator is empty AND we have no more addrs to process we can kill the whole driver loop
                    None if inflight_addrs.is_empty() && requeued_addrs.is_empty() => break 'driver,
                    None => break,
                }
            }

            // pick the next ipv4 and ipv6 addresses that we should send packets from
            let source_ipv4_addr = source_ipv4_cycle.next();
            let source_ipv6_addr = source_ipv6_cycle.next();

            // unless we send too many packets, we're clear to send
            let mut rate_limit_unviolated = true;

            if let Some(packet_per_second_limit) = &packets_per_second {
                if next_packet_bucket_expiry.le(&Instant::now()) {
                    current_packets_sent_for_bucket = 0;
                    next_packet_bucket_expiry = Instant::now()
                        .checked_add(Duration::from_millis(RATE_LIMIT_RESOLUTION))
                        .unwrap();

                    // might as well send a stats update
                    reporting_channel.send(ArmadaWorkMessage::stats(
                        total_processed_ports,
                        inflight_addrs.len() as u128,
                        total_packets_sent
                    )).context("Failed to send a stats update")?;
                }

                // if the number of packets we've sent so far is below the packet limit for our resolution, mark as "clear to send" otherwise don't
                rate_limit_unviolated = (*packet_per_second_limit / (MS_IN_SECOND / RATE_LIMIT_RESOLUTION as usize)) > current_packets_sent_for_bucket;
            } else {
                // even though we don't need to enforce a rate limit, the bucket timeout presents a good opportunity to push a stats update

                if next_packet_bucket_expiry.le(&Instant::now()) {
                    next_packet_bucket_expiry = Instant::now()
                        .checked_add(Duration::from_millis(RATE_LIMIT_RESOLUTION))
                        .unwrap();

                    // send our stats update
                    reporting_channel
                        .send(ArmadaWorkMessage::stats(
                            total_processed_ports,
                            inflight_addrs.len() as u128,
                            total_packets_sent
                        ))
                        .context("Failed to send stats update over reporting channel.")?;
                }
            }

            // if we're not pushing any rate limits, we should do some sending
            if rate_limit_unviolated {
                // Send packets
                let addresses_sent_packets = self.send_packets(
                    ipv4_tcp_sender,
                    ipv6_tcp_sender,
                    &mut requeued_addrs,
                    source_ipv4_addr,
                    source_ipv6_addr,
                    listening_port,
                    tcp_seq,
                );

                total_packets_sent += addresses_sent_packets.len() as u128;

                if !addresses_sent_packets.is_empty() {
                    current_packets_sent_for_bucket += addresses_sent_packets.len();

                    inflight_addrs.extend(addresses_sent_packets.clone());

                    // mark for expiration
                    let expiration = Instant::now().checked_add(port_timeout.clone()).unwrap();
                    expiry_list.push((expiration, addresses_sent_packets));
                }
            } else {
                std::thread::sleep(next_packet_bucket_expiry.duration_since(Instant::now()));
            }

            // receive remotes that syn-ack'd
            let received_remotes_v4 =
                self.record_open_sockets_from_response(ipv4_tcp_receiver, listening_port);
            let received_remotes_v6 =
                self.record_open_sockets_from_response(ipv6_tcp_receiver, listening_port);

            // save the remotes that were actually in-flight
            received_remotes_v4
                .into_iter()
                .chain(received_remotes_v6)
                .filter(|remote_addr| inflight_addrs.remove(&remote_addr))
                .for_each(|remote_addr| {
                    // if a port was deemed open, we can update this statistic
                    total_processed_ports += 1;
                    packet_retry_tracker.remove(&remote_addr);
                    open_ports.push(remote_addr);
                });

            if !open_ports.is_empty() {
                // send our stats update
                reporting_channel.send(ArmadaWorkMessage::stats(
                    total_processed_ports,
                    inflight_addrs.len() as u128,
                    total_packets_sent
                )).context("Failed to send stats message to reporting channel.")?;
                // we'll empty the open ports vec into our update here
                reporting_channel.send(
                    ArmadaWorkMessage::results(open_ports.drain(.. open_ports.len()).collect())
                ).context("Failed to send results message to reporting channel.")?;
            }

            self.process_expiration(&mut expiry_list)
                .into_iter()
                .filter(|expired_remote| inflight_addrs.remove(&expired_remote))
                .filter(|expired_remote| {
                    let retry_counter = packet_retry_tracker
                        .entry(expired_remote.clone())
                        .or_insert(0);

                    if *retry_counter == port_retries {
                        // this port has been deemed closed and therefore has been "processed"
                        total_processed_ports += 1;
                        packet_retry_tracker.remove(&expired_remote);
                        false
                    } else {
                        *retry_counter += 1;
                        true
                    }
                })
                .for_each(|expired_remote| {
                    requeued_addrs.push(expired_remote);
                });
        }

        // send the final stats and results before closing up shop
        reporting_channel
            .send(ArmadaWorkMessage::stats(total_processed_ports, inflight_addrs.len() as u128, total_packets_sent))
            .context("Failed to send final stats message over reporting channel.")?;

        reporting_channel
            .send(ArmadaWorkMessage::results(open_ports))
            .context("Failed to send final results message over reporting channel.")?;

        Ok(())
    }

    /// Pulls socket addresses off the queued address list and sends them SYN TCP packets via IPv4 or IPv6
    fn send_packets(
        &self,
        ipv4_tcp_sender: &mut TransportSender,
        ipv6_tcp_sender: &mut TransportSender,
        requeued_addrs: &mut Vec<SocketAddr>,
        source_ipv4: Option<&Ipv4Addr>,
        source_ipv6: Option<&Ipv6Addr>,
        listening_port: u16,
        tcp_seq: &mut u32,
    ) -> Vec<SocketAddr> {
        use crate::armada::packet::{create_syn_tcp_packet_v4, create_syn_tcp_packet_v6};

        let mut sent_addrs = Vec::with_capacity(BATCH_SEND_SIZE);
        let mut syn_tcp_buffer = [0; 32];

        for _ in 0 .. BATCH_SEND_SIZE {
            let remote = match requeued_addrs.pop() {
                Some(remote) => remote,
                None => break,
            };

            let remote_port = remote.port();

            let (tcp_sender, packet) = match (&remote.ip(), source_ipv4, source_ipv6) {
                (IpAddr::V4(remote_ipv4), Some(source_ipv4_addr), _) => {
                    let packet = create_syn_tcp_packet_v4(
                        source_ipv4_addr,
                        remote_ipv4,
                        listening_port,
                        remote_port,
                        &mut syn_tcp_buffer,
                        tcp_seq,
                    );

                    (&mut *ipv4_tcp_sender, packet)
                }
                (IpAddr::V6(remote_ipv6), _, Some(source_ipv6_addr)) => {
                    let packet = create_syn_tcp_packet_v6(
                        source_ipv6_addr,
                        remote_ipv6,
                        listening_port,
                        remote_port,
                        &mut syn_tcp_buffer,
                        tcp_seq,
                    );

                    (&mut *ipv6_tcp_sender, packet)
                }
                (IpAddr::V4(_), None, _) => {
                    error!("Attempted to port scan an IPv4 address without any provided IPv4 source addresses. Port will be skipped.");
                    continue;
                }
                (IpAddr::V6(_), _, None) => {
                    error!("Attempted to port scan an IPv6 address without any provided IPv6 source addresses. Port will be skipped.");
                    continue;
                }
            };

            let packet = match packet {
                Some(packet) => packet,
                None => {
                    warn!(
                        "Unable to create SYN packet for {}. Port will be skipped.",
                        remote
                    );
                    continue;
                }
            };

            match tcp_sender.try_send_to(packet, remote.ip()) {
                Ok(Some(_)) => sent_addrs.push(remote),
                _ => {
                    //eprintln!("ERR: {:?}", e);
                    requeued_addrs.push(remote);
                    break;
                }
            }
        }

        sent_addrs
    }

    /// Receives some number of responses from the socket and determines which sockets indicate an open status
    fn record_open_sockets_from_response(
        &self,
        tcp_receiver: &mut TransportReceiver,
        listening_port: u16,
    ) -> Vec<SocketAddr> {
        use pnet::packet::tcp::TcpFlags::{NS, CWR, ECE, URG, ACK, RST, SYN, PSH, FIN};

        let mut results = Vec::with_capacity(BATCH_RECV_SIZE);

        while let Ok(Some((packet, remote))) = tcp_receiver.try_next() {
            let flag = packet.get_flags();

            let seq = packet.get_sequence();
            let ns_flag = flag & NS != 0;
            let cwr_flag = flag & CWR != 0;
            let ece_flag = flag & ECE != 0;
            let urg_flag = flag & URG != 0;
            let ack_flag = flag & ACK != 0;
            let psh_flag = flag & PSH != 0;
            let rst_flag = flag & RST != 0;
            let syn_flag = flag & SYN != 0;
            let fin_flag = flag & FIN != 0;

            if remote == IpAddr::from([103,97,3,19]) {
                println!("{:#?}:", packet.get_source());
                println!("\tRemote: {remote:#?}");
                println!("\tSEQ: {seq}");
                println!("\tNS: {ns_flag}");
                println!("\tCWR: {cwr_flag}");
                println!("\tECE: {ece_flag}");
                println!("\tURG: {urg_flag}");
                println!("\tACK: {ack_flag}");
                println!("\tPSH: {psh_flag}");
                println!("\tRST: {rst_flag}");
                println!("\tSYN: {syn_flag}");
                println!("\tFIN: {fin_flag}");
            }

            // todo: if reset packet back, return "definitely closed"

            if packet.get_destination() == listening_port && !rst_flag && ack_flag {
                results.push(SocketAddr::new(remote, packet.get_source()));

                // if we've reached the capacity for this vec, we've processed enough and can return
                if results.len() == results.capacity() {
                    break;
                }
            }
        }

        results
    }

    /// Process all currently expired packets
    fn process_expiration(
        &self,
        expiry_list: &mut Vec<(Instant, Vec<SocketAddr>)>,
    ) -> Vec<SocketAddr> {
        // assume send size for efficient writing
        let mut all_expired_remotes = Vec::with_capacity(BATCH_SEND_SIZE);

        let now = Instant::now();

        loop {
            match expiry_list.first() {
                Some((expiry, _)) if expiry.le(&now) => {
                    let (_, expired_remotes) = expiry_list.pop().expect("This should not be possible as we just confirmed an item exists.");

                    all_expired_remotes.extend(expired_remotes);
                }
                _ => break,
            }
        }

        all_expired_remotes
    }
}