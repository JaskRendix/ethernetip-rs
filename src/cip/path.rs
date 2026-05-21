#[derive(Clone, Debug)]
pub enum PathSegment {
    Port { port: u8, link: u8 },
    Class(u16),
    Instance(u16),
    Attribute(u16),
}

impl PathSegment {
    fn encode(&self, out: &mut Vec<u8>) {
        match *self {
            PathSegment::Port { port, link } => {
                out.extend_from_slice(&[port, link, 0x00, 0x00]);
            }
            PathSegment::Class(id) => {
                if id < 0x100 {
                    out.extend_from_slice(&[0x20, id as u8]);
                } else {
                    out.extend_from_slice(&[0x21, (id & 0xFF) as u8, (id >> 8) as u8]);
                }
            }
            PathSegment::Instance(id) => {
                if id < 0x100 {
                    out.extend_from_slice(&[0x24, id as u8]);
                } else {
                    out.extend_from_slice(&[0x25, (id & 0xFF) as u8, (id >> 8) as u8]);
                }
            }
            PathSegment::Attribute(id) => {
                out.extend_from_slice(&[0x30, id as u8]);
            }
        }
    }
}

pub fn encode_path(segments: &[PathSegment]) -> Vec<u8> {
    let mut raw = Vec::new();
    for seg in segments {
        seg.encode(&mut raw);
    }
    let words = (raw.len() / 2) as u8;

    let mut out = Vec::with_capacity(1 + raw.len());
    out.push(words);
    out.extend_from_slice(&raw);
    out
}
