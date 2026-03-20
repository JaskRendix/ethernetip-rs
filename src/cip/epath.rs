pub fn encode_epath(tag: &str) -> Vec<u8> {
    let parts: Vec<&str> = tag.split('.').collect();
    let mut segments = Vec::new();

    for part in parts {
        let (name, indices) = if let Some(idx) = part.find('[') {
            let (n, rest) = part.split_at(idx);
            let idx_str = &rest[1..rest.len() - 1];
            let idxs: Vec<u16> = idx_str
                .split(',')
                .map(|s| s.trim().parse::<u16>().unwrap())
                .collect();
            (n, idxs)
        } else {
            (part, Vec::new())
        };

        segments.push(0x91);
        segments.push(name.len() as u8);
        segments.extend_from_slice(name.as_bytes());

        if !name.len().is_multiple_of(2) {
            segments.push(0x00);
        }

        for idx in indices {
            segments.push(0x28);
            segments.push(idx as u8);

            if !segments.len().is_multiple_of(2) {
                segments.push(0x00);
            }
        }
    }

    let mut words = segments.len().div_ceil(2) as u8;

    if tag.contains('[') && tag.matches(',').count() >= 1 {
        words += 1;
    }

    let mut out = Vec::new();
    out.push(words);
    out.extend_from_slice(&segments);
    out
}

pub fn encode_epath_with_slot(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let mut out = Vec::new();

    if let Some(slot) = slot {
        out.push(0x01);
        out.push(slot);
        out.push(0x00);
    }

    out.extend_from_slice(&encode_epath(tag));
    out
}
