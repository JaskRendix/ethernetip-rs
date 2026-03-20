use crate::types::{CipType, SymbolInfo};

pub fn build_symbol_browse_request() -> Vec<u8> {
    let path = vec![0x20, 0x6B, 0x24, 0x00];

    let mut cip = Vec::new();
    cip.push(0x03);
    cip.push((path.len() / 2) as u8);
    cip.extend_from_slice(&path);

    cip.extend_from_slice(&3u16.to_le_bytes());
    cip.extend_from_slice(&1u16.to_le_bytes());
    cip.extend_from_slice(&2u16.to_le_bytes());
    cip.extend_from_slice(&5u16.to_le_bytes());

    cip
}

pub fn parse_symbol_browse_response(buf: &[u8]) -> Vec<SymbolInfo> {
    let mut out = Vec::new();
    let mut pos = 0;

    while pos < buf.len() {
        if pos + 2 > buf.len() {
            break;
        }
        let _st_name = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        pos += 2;

        if pos + 1 > buf.len() {
            break;
        }
        let name_len = buf[pos] as usize;
        pos += 1;

        if pos + name_len > buf.len() {
            break;
        }
        let name = String::from_utf8_lossy(&buf[pos..pos + name_len]).into_owned();
        pos += name_len;

        if !name_len.is_multiple_of(2) && pos < buf.len() && buf[pos] == 0x00 {
            pos += 1;
        }

        if pos + 2 > buf.len() {
            break;
        }
        let _st_type = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        pos += 2;

        if pos + 2 > buf.len() {
            break;
        }
        let typ_code = buf[pos];
        pos += 2;

        let typ = match CipType::from_u8(typ_code) {
            Some(t) => t,
            None => continue,
        };

        if pos + 2 > buf.len() {
            break;
        }
        let _st_dims = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
        pos += 2;

        if pos + 1 > buf.len() {
            break;
        }
        let dim_count = buf[pos] as usize;
        pos += 1;

        let mut dims = [0u16; 3];
        for dim in dims.iter_mut().take(dim_count.min(3)) {
            if pos + 2 > buf.len() {
                break;
            }
            *dim = u16::from_le_bytes([buf[pos], buf[pos + 1]]);
            pos += 2;
        }

        let array_dims = if dims.iter().any(|&d| d != 0) {
            Some((dims[0], dims[1], dims[2]))
        } else {
            None
        };

        out.push(SymbolInfo {
            name,
            typ,
            array_dims,
        });

        if pos + 2 <= buf.len() && buf[pos] == 0x00 && buf[pos + 1] == 0x00 {
            pos += 2;
        }
    }

    out
}
