#[derive(Clone)]
pub struct PortIterator {
    inner: Vec<PortRegion>,
    idx: Option<usize>,
}

impl PortIterator {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
            idx: None,
        }
    }

    pub fn size(&self) -> u64 {
        self.inner.iter()
            .fold(0, |acc, port_region| acc + port_region.size() as u64)
    }

    pub fn add_port(mut self, port: u16) -> Self {
        self.inner.push(PortRegion {
            start: port,
            end: port,
            idx: None,
        });
        self
    }

    pub fn add_range(mut self, start: u16, end: u16) -> Self {
        self.inner.push(PortRegion {
            start,
            end,
            idx: None,
        });
        self
    }

    fn reset(&mut self) {
        self.inner.iter_mut().for_each(|inner| inner.reset());
        self.idx = None;
    }
}

#[derive(Clone)]
struct PortRegion {
    start: u16,
    end: u16,
    idx: Option<u16>,
}

impl PortRegion {
    fn reset(&mut self) {
        if let PortRegion { idx, .. } = self {
            *idx = None;
        }
    }

    fn size(&self) -> u16 {
        self.end - self.start + 1
    }
}

impl Iterator for PortRegion {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.idx {
            Some(idx) if *idx < self.end - self.start => {
                *idx += 1;
                Some(self.start + *idx)
            },
            Some(_) => None,
            idx @ None => {
                *idx = Some(0);

                Some(self.start)
            }
        }
    }
}

impl Iterator for PortIterator {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.idx {
                Some(idx) if *idx == self.inner.len() => break None,
                Some(idx) => {
                    match self
                        .inner
                        .get_mut(*idx)
                        .map(|port_region| port_region.next())
                    {
                        Some(Some(port)) => break Some(port),
                        Some(None) => *idx += 1,
                        None => break None,
                    }
                }
                idx @ None => *idx = Some(0),
            }
        }
    }
}
