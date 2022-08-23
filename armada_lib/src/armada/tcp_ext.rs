use pnet::packet::tcp::TcpPacket;
use pnet::packet::{ipv4::Ipv4Packet, Packet};
use pnet::transport::TransportChannelType::{Layer3, Layer4};
use pnet::transport::TransportProtocol::{Ipv4, Ipv6};
use pnet::transport::{TransportReceiver, TransportSender};
use pnet_sys::{Buf, BufLen, MutBuf, SockAddr, SockLen};
use std::net::IpAddr;
use std::{mem, net};

pub trait TcpSenderExt {
    fn try_send_to<T: Packet>(
        &mut self,
        packet: T,
        destination: IpAddr,
    ) -> std::io::Result<Option<usize>>;
}

pub trait TcpReceiverExt {
    fn try_next(&mut self) -> std::io::Result<Option<(TcpPacket, IpAddr)>>;
}

impl TcpSenderExt for TransportSender {
    fn try_send_to<T: Packet>(
        &mut self,
        packet: T,
        destination: IpAddr,
    ) -> std::io::Result<Option<usize>> {
        let mut socket_addr_storage = unsafe { mem::zeroed() };

        let sockaddr = match destination {
            IpAddr::V4(ip_addr) => net::SocketAddr::V4(net::SocketAddrV4::new(ip_addr, 0)),
            IpAddr::V6(ip_addr) => net::SocketAddr::V6(net::SocketAddrV6::new(ip_addr, 0, 0, 0)),
        };

        let socket_len = pnet_sys::addr_to_sockaddr(sockaddr, &mut socket_addr_storage);
        let buffer = packet.packet();

        let send_len = unsafe {
            match libc::sendto(
                self.socket.fd,
                buffer.as_ptr() as Buf,
                buffer.len() as BufLen,
                0, //libc::MSG_DONTWAIT, // nonblocking so we just try to get the next message, fail otherwise
                (&mut socket_addr_storage as *mut pnet_sys::SockAddrStorage) as *mut SockAddr,
                socket_len,
            ) {
                // -1 == Would block! We couldn't send a packet immediately so let's return None
                -1 => return Ok(None),
                len if len < 0 => Err(std::io::Error::last_os_error()),
                len => Ok(len as usize),
            }?
        };

        Ok(Some(send_len))
    }
}

impl TcpReceiverExt for TransportReceiver {
    fn try_next(&mut self) -> std::io::Result<Option<(TcpPacket, IpAddr)>> {
        let buffer = &mut self.buffer;
        let mut socket_addr_storage: pnet_sys::SockAddrStorage = unsafe { mem::zeroed() };
        let mut caddrlen = mem::size_of::<pnet_sys::SockAddrStorage>() as SockLen;

        // this is safe as we're not moving or deallocating the memory while in use
        let recv_len = unsafe {
            match libc::recvfrom(
                self.socket.fd,
                buffer.as_ptr() as MutBuf,
                buffer.len() as BufLen,
                libc::MSG_DONTWAIT, // nonblocking so we just try to get the next message, fail otherwise
                (&mut socket_addr_storage as *mut pnet_sys::SockAddrStorage) as *mut SockAddr,
                &mut caddrlen,
            ) {
                // -1 == Would block! We don't have a packet immediately available so let's return None
                -1 => return Ok(None),
                len if len < 0 => Err(std::io::Error::last_os_error()),
                len => Ok(len as usize),
            }?
        };

        let offset = match self.channel_type {
            Layer4(Ipv4(_)) => {
                let ip_header = Ipv4Packet::new(&self.buffer[..]).unwrap();

                ip_header.get_header_length() as usize * 4usize
            }
            Layer4(Ipv6(_)) => {
                /*let ip_header = Ipv6Packet::new(&self.buffer[..]).unwrap();

                ip_header.get_header_length() as usize * 4usize*/

                // https://en.wikipedia.org/wiki/IPv6_packet#Fixed_header -> 40 octets
                40
            }
            Layer3(_) => {
                fixup_packet(&mut self.buffer[..]);

                0
            }
        };

        // sometimes hosts will return odd packets...
        if recv_len < offset {
            return Ok(None);
        }

        let packet = match TcpPacket::new(&self.buffer[offset..recv_len]) {
            Some(tcp_packet) => tcp_packet,
            None => return Ok(None),
        };

        let addr = pnet_sys::sockaddr_to_addr(
            &socket_addr_storage,
            mem::size_of::<pnet_sys::SockAddrStorage>(),
        )?;
        let ip = match addr {
            net::SocketAddr::V4(sa) => IpAddr::V4(*sa.ip()),
            net::SocketAddr::V6(sa) => IpAddr::V6(*sa.ip()),
        };

        Ok(Some((packet, ip)))
    }
}

#[cfg(any(target_os = "freebsd", target_os = "macos", target_os = "ios"))]
fn fixup_packet(buffer: &mut [u8]) {
    use pnet_packet::ipv4::MutableIpv4Packet;

    let buflen = buffer.len();
    let mut new_packet = MutableIpv4Packet::new(buffer).unwrap();

    let length = u16::from_be(new_packet.get_total_length());
    new_packet.set_total_length(length);

    // OS X does this awesome thing where it removes the header length
    // from the total length sometimes.
    let length =
        new_packet.get_total_length() as usize + (new_packet.get_header_length() as usize * 4usize);
    if length == buflen {
        new_packet.set_total_length(length as u16)
    }

    let offset = u16::from_be(new_packet.get_fragment_offset());
    new_packet.set_fragment_offset(offset);
}

#[cfg(all(
    not(target_os = "freebsd"),
    not(any(target_os = "macos", target_os = "ios"))
))]
fn fixup_packet(_buffer: &mut [u8]) {}
