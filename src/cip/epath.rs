pub fn encode_epath(tag: &str) -> Vec<u8> {
    let mut segments = Vec::new();

    for part in tag.split('.') {
        let (name, indices) = if let Some(idx) = part.find('[') {
            let (n, rest) = part.split_at(idx);
            let idx_str = &rest[1..rest.len() - 1];

            let idxs: Vec<u32> = idx_str
                .split(',')
                .filter_map(|s| s.trim().parse::<u32>().ok())
                .collect();

            (n, idxs)
        } else {
            (part, Vec::new())
        };

        // Symbolic segment
        segments.push(0x91);
        segments.push(name.len() as u8);
        segments.extend_from_slice(name.as_bytes());

        if name.len() % 2 != 0 {
            segments.push(0x00); // pad to even byte length
        }

        // Array subscripts
        for idx in indices {
            if idx <= 0xFF {
                segments.push(0x28);
                segments.push(idx as u8);
            } else if idx <= 0xFFFF {
                segments.push(0x29);
                segments.push(0x00);
                segments.extend_from_slice(&(idx as u16).to_le_bytes());
            } else {
                segments.push(0x2A);
                segments.push(0x00);
                segments.extend_from_slice(&idx.to_le_bytes());
            }
        }
    }

    // Word count = bytes / 2 (segments always padded to even length)
    let word_count = (segments.len() / 2) as u8;

    let mut out = Vec::with_capacity(1 + segments.len());
    out.push(word_count);
    out.extend_from_slice(&segments);
    out
}

pub fn encode_epath_with_slot(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let tag_epath = encode_epath(tag);
    let tag_words = tag_epath[0] as usize;
    let tag_segments = &tag_epath[1..];

    if let Some(slot) = slot {
        let mut out = Vec::new();

        // CIP port segment = 4 bytes = 2 words
        let port_words = 2usize;
        let total_words = port_words + tag_words;

        out.push(total_words as u8);

        // Port segment (port 1 → backplane)
        out.push(0x01); // port
        out.push(slot); // link address (CPU slot)
        out.push(0x00); // pad
        out.push(0x00); // pad

        out.extend_from_slice(tag_segments);
        out
    } else {
        tag_epath
    }
}
