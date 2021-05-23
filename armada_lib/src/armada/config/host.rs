use cidr_utils::cidr::{IpCidr, IpCidrIpAddrIterator, Ipv4Cidr, Ipv6Cidr};
use std::net::IpAddr;

const IPV4_BITMASK: u8 = 32;
const IPV6_BITMASK: u8 = 128;

pub struct HostIterator {
    inner: Vec<IpCidr>,
    current_cidr_idx: Option<usize>,
    current_cidr_iterator: Option<IpCidrIpAddrIterator>,
}

// we implement a slightly incorrect clone here intentionally
impl Clone for HostIterator {
    fn clone(&self) -> Self {
        HostIterator {
            inner: self.inner.clone(),
            current_cidr_idx: None,
            current_cidr_iterator: None
        }
    }
}

impl HostIterator {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            current_cidr_idx: None,
            current_cidr_iterator: None,
        }
    }

    pub fn size(&self) -> u128 {
        use num_traits::ToPrimitive;

        self.inner.iter()
            .fold(0, |acc, cidr| acc + cidr.size().to_u128().expect("Cidr range is too large to report back a size. Crashing here is in your best interest."))
    }

    pub fn add_ip(mut self, addr: IpAddr) -> Self {
        let ip_cidr = match addr {
            IpAddr::V4(ipv4_addr) => IpCidr::V4(
                Ipv4Cidr::from_prefix_and_bits(ipv4_addr, IPV4_BITMASK)
                    .expect("Failed to convert to IPv4 CIDR"),
            ),
            IpAddr::V6(ipv6_addr) => IpCidr::V6(
                Ipv6Cidr::from_prefix_and_bits(ipv6_addr, IPV6_BITMASK)
                    .expect("Failed to convert to IPv6 CIDR"),
            ),
        };

        self.inner.push(ip_cidr);

        self
    }

    pub fn add_ips(mut self, addrs: Vec<IpAddr>) -> Self {
        for addr in addrs {
            self = self.add_ip(addr);
        }

        self
    }

    pub fn add_cidr(mut self, range: IpCidr) -> Self {
        self.inner.push(range);
        self
    }

    pub fn reset(&mut self) {
        self.current_cidr_idx = None;
        self.current_cidr_iterator = None;
    }

    fn rotate_iterator(&mut self) {
        match &mut self.current_cidr_idx {
            Some(idx) if *idx < self.inner.len() => *idx += 1,
            Some(idx) => {}
            None => self.current_cidr_idx = Some(0),
        }

        // reset the iterator
        self.current_cidr_iterator = None;

        if let Some(idx) = self.current_cidr_idx {
            if let Some(ip_cidr) = self.inner.get(idx) {
                self.current_cidr_iterator = Some(ip_cidr.iter());
            }
        }
    }
}

impl Iterator for HostIterator {
    type Item = IpAddr;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &self.current_cidr_idx {
                Some(idx) if *idx == self.inner.len() => break None,
                _ => {}
            }

            match self.current_cidr_iterator.as_mut().map(|iter| iter.next()) {
                Some(Some(next_ip)) => break Some(next_ip),
                Some(None) | None => self.rotate_iterator(),
            }
        }
    }
}
