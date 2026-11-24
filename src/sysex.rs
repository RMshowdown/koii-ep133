//! Teenage Engineering SysEx framing for the EP-133 K.O. II.
//!
//! This protocol is NOT in the official TE guide. It comes from community
//! docs (MIT): https://github.com/kmorrill/ep-series-sysex (docs/sysex-protocol.md)
//! and https://github.com/icherniukh/ep133-krate (PROTOCOL.md). Code here is our own.
//!
//! Frame shapes (F0 and F7 included):
//!   request  F0 00 20 76 33 40 [60|id_hi] [id_lo] [command] <packed7 body> F7
//!   reply    F0 00 20 76 33 40 [20|id_hi] [id_lo] [command] [status] <packed7 body> F7
//!   debug    F0 00 20 76 33 33 <ASCII text> F7   -> device is unhappy: stop all file traffic.

pub const TE_MANUFACTURER: [u8; 3] = [0x00, 0x20, 0x76];
/// Product byte of the EP-133 K.O. II.
pub const EP133_PRODUCT: u8 = 0x33;
const PROTOCOL_MARKER: u8 = 0x40;
const DEBUG_MARKER: u8 = 0x33;

pub const CMD_GREET: u8 = 0x01;
pub const CMD_FILE: u8 = 0x05;

/// Encode 8-bit bytes for SysEx: groups of 7 bytes, each group led by a byte
/// holding their high bits (bit 0 = first byte of the group).
pub fn pack7(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + raw.len() / 7 + 1);
    for chunk in raw.chunks(7) {
        let mut flags = 0u8;
        for (bit, byte) in chunk.iter().enumerate() {
            if byte & 0x80 != 0 {
                flags |= 1 << bit;
            }
        }
        out.push(flags);
        out.extend(chunk.iter().map(|b| b & 0x7F));
    }
    out
}

/// Reverse of [`pack7`].
pub fn unpack7(encoded: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(encoded.len());
    for group in encoded.chunks(8) {
        let flags = group[0];
        for (bit, byte) in group[1..].iter().enumerate() {
            let high = if flags & (1 << bit) != 0 { 0x80 } else { 0 };
            out.push(byte | high);
        }
    }
    out
}

/// A full request frame (F0 ... F7). `request_id` is 12 bits.
pub fn build_request(request_id: u16, command: u8, raw_body: &[u8]) -> Vec<u8> {
    let id = request_id & 0x0FFF;
    let hi = ((id >> 7) & 0x1F) as u8;
    let lo = (id & 0x7F) as u8;
    let mut frame = vec![0xF0];
    frame.extend(TE_MANUFACTURER);
    frame.extend([EP133_PRODUCT, PROTOCOL_MARKER, 0x60 | hi, lo, command]);
    frame.extend(pack7(raw_body));
    frame.push(0xF7);
    frame
}

/// What an incoming SysEx frame means for a request we are waiting on.
#[derive(Debug, PartialEq)]
pub enum Reply {
    /// The answer to our request. `status` 0 = OK.
    Answer { status: u8, body: Vec<u8> },
    /// Firmware debug text: stop talking to the device's file system.
    Debug(String),
    /// Something else (another reply, an event, not TE).
    Other,
}

pub fn parse_reply(frame: &[u8], request_id: u16, command: u8) -> Reply {
    // Strip F0 / F7.
    let data = match frame {
        [0xF0, inner @ .., 0xF7] => inner,
        [0xF0, inner @ ..] => inner,
        _ => return Reply::Other,
    };
    if data.len() < 5 || data[..3] != TE_MANUFACTURER {
        return Reply::Other;
    }
    if data[4] == DEBUG_MARKER {
        let text = String::from_utf8_lossy(&data[5..]).trim().to_string();
        return Reply::Debug(text);
    }
    if data.len() < 9 || data[3] != EP133_PRODUCT || data[4] != PROTOCOL_MARKER {
        return Reply::Other;
    }
    let id = request_id & 0x0FFF;
    let expected_hi = 0x20 | ((id >> 7) & 0x1F) as u8;
    if data[5] != expected_hi || data[6] != (id & 0x7F) as u8 || data[7] != command {
        return Reply::Other;
    }
    Reply::Answer {
        status: data[8],
        body: unpack7(&data[9..]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack7_matches_documented_example() {
        // Example from kmorrill/ep-series-sysex docs/sysex-protocol.md.
        assert_eq!(pack7(&[0x07, 0xD0, 0x00, 0x2C]), vec![0x02, 0x07, 0x50, 0x00, 0x2C]);
    }

    #[test]
    fn pack7_round_trip() {
        let raw: Vec<u8> = (0..=255u8).collect();
        assert_eq!(unpack7(&pack7(&raw)), raw);
    }

    #[test]
    fn request_frame_layout() {
        // METADATA GET on /sounds, page 0: raw 07 02 03 E8 00 00 (from krate PROTOCOL.md).
        let frame = build_request(0x0123, CMD_FILE, &[0x07, 0x02, 0x03, 0xE8, 0x00, 0x00]);
        assert_eq!(&frame[..9], &[0xF0, 0x00, 0x20, 0x76, 0x33, 0x40, 0x62, 0x23, 0x05]);
        assert_eq!(*frame.last().unwrap(), 0xF7);
        assert_eq!(unpack7(&frame[9..frame.len() - 1]), vec![0x07, 0x02, 0x03, 0xE8, 0x00, 0x00]);
    }

    #[test]
    fn reply_is_matched_by_id_and_command() {
        let mut frame = vec![0xF0, 0x00, 0x20, 0x76, 0x33, 0x40, 0x22, 0x23, 0x05, 0x00];
        frame.extend(pack7(&[0x00, 0x00, b'{', b'}', 0x00]));
        frame.push(0xF7);
        assert_eq!(
            parse_reply(&frame, 0x0123, CMD_FILE),
            Reply::Answer { status: 0, body: vec![0x00, 0x00, b'{', b'}', 0x00] }
        );
        assert_eq!(parse_reply(&frame, 0x0124, CMD_FILE), Reply::Other);
        assert_eq!(parse_reply(&frame, 0x0123, CMD_GREET), Reply::Other);
    }

    #[test]
    fn debug_frame_is_detected() {
        let mut frame = vec![0xF0, 0x00, 0x20, 0x76, 0x33, 0x33];
        frame.extend(b"err lfs 6327");
        frame.push(0xF7);
        assert_eq!(parse_reply(&frame, 1, CMD_FILE), Reply::Debug("err lfs 6327".into()));
    }
}
