use pnet::packet::ip::IpNextHeaderProtocols;
use std::net::{Ipv4Addr, Ipv6Addr};
use pnet::packet::tcp::MutableTcpPacket;

pub(crate) fn create_syn_tcp_packet_v4<'b>(
    source_ip: &Ipv4Addr,
    remote_ip: &Ipv4Addr,
    source_port: u16,
    remote_port: u16,
    buffer: &'b mut [u8],
    tcp_seq: &mut u32,
) -> Option<MutableTcpPacket<'b>> {
    use pnet::packet::Packet;

    let mut tcp_packet = create_syn_tcp_packet_inner(source_port, remote_port, buffer, tcp_seq)?;

    let checksum = pnet::util::ipv4_checksum(
        tcp_packet.packet(),
        8,
        &[],
        source_ip,
        remote_ip,
        IpNextHeaderProtocols::Tcp,
    );
    tcp_packet.set_checksum(checksum);

    Some(tcp_packet)
}

pub(crate) fn create_syn_tcp_packet_v6<'b>(
    source_ip: &Ipv6Addr,
    remote_ip: &Ipv6Addr,
    source_port: u16,
    remote_port: u16,
    buffer: &'b mut [u8],
    tcp_seq: &mut u32,
) -> Option<MutableTcpPacket<'b>> {
    use pnet::packet::Packet;

    let mut tcp_packet = create_syn_tcp_packet_inner(source_port, remote_port, buffer, tcp_seq)?;

    let checksum = pnet::util::ipv6_checksum(
        tcp_packet.packet(),
        8,
        &[],
        source_ip,
        remote_ip,
        IpNextHeaderProtocols::Tcp,
    );
    tcp_packet.set_checksum(checksum);

    Some(tcp_packet)
}

fn create_syn_tcp_packet_inner<'b>(
    source_port: u16,
    remote_port: u16,
    buffer: &'b mut [u8],
    tcp_seq: &mut u32,
) -> Option<MutableTcpPacket<'b>> {
    use pnet::packet::tcp::TcpFlags::SYN;
    use pnet::packet::tcp::TcpOption;

    // inc 2?
    *tcp_seq += 1;

    let mut tcp_packet = MutableTcpPacket::new(buffer)?;

    tcp_packet.set_source(source_port);
    tcp_packet.set_destination(remote_port);
    tcp_packet.set_sequence(*tcp_seq);
    tcp_packet.set_acknowledgement(0);
    tcp_packet.set_window(1024);
    tcp_packet.set_data_offset(8);
    tcp_packet.set_flags(SYN);
    tcp_packet.set_options(&[TcpOption::mss(1460)]);

    Some(tcp_packet)
}
