//! Forward-compatible MS-RDPEGFX capability negotiation.

use ironrdp_core::{ensure_size, invalid_field_err, Decode, DecodeResult, ReadCursor};
use ironrdp_pdu::rdp::vc::dvc::gfx::{
    CapabilitiesAdvertisePdu, CapabilitiesV10Flags, CapabilitiesV103Flags,
    CapabilitiesV104Flags, CapabilitiesV107Flags, CapabilitiesV81Flags, CapabilitySet,
};

const ADVERTISE_COMMAND: u16 = 0x12;
const GFX_HEADER_SIZE: usize = 8;
const CAPABILITY_HEADER_SIZE: usize = 8;

pub(super) fn is_advertisement(data: &[u8]) -> bool {
    data.get(..2) == Some(ADVERTISE_COMMAND.to_le_bytes().as_slice())
}

/// Each cap has version:u32, dataLength:u32, and exactly dataLength payload bytes.
/// IronRDP 0.7 rejects new version codes before consuming their declared length.
/// Skip only those unknown codes; known versions still use IronRDP's decoder.
pub(super) fn decode_advertisement(data: &[u8]) -> DecodeResult<CapabilitiesAdvertisePdu> {
    let mut cursor = ReadCursor::new(data);
    ensure_size!(in: cursor, size: GFX_HEADER_SIZE + 2);
    if cursor.read_u16() != ADVERTISE_COMMAND {
        return Err(invalid_field_err!("command", "expected CapabilitiesAdvertise"));
    }
    // Reserved header flags are ignored by IronRDP's existing ClientPdu decoder.
    let _flags = cursor.read_u16();
    let length = cursor.read_u32() as usize;
    if length != data.len() {
        return Err(invalid_field_err!("pduLength", "capability PDU length mismatch"));
    }
    let count = usize::from(cursor.read_u16());
    if count == 0 {
        return Err(invalid_field_err!("count", "empty capability advertisement"));
    }
    ensure_size!(in: cursor, size: count * CAPABILITY_HEADER_SIZE);
    let mut known = Vec::new();
    for _ in 0..count {
        if let Some(capability) = decode_capability(&mut cursor)? {
            known.push(capability);
        }
    }
    if !cursor.is_empty() {
        return Err(invalid_field_err!("count", "unconsumed capability data"));
    }
    Ok(CapabilitiesAdvertisePdu(known))
}

fn decode_capability(cursor: &mut ReadCursor<'_>) -> DecodeResult<Option<CapabilitySet>> {
    ensure_size!(in: cursor, size: CAPABILITY_HEADER_SIZE);
    let entry = cursor.remaining();
    let version = cursor.read_u32();
    let data_length = cursor.read_u32() as usize;
    ensure_size!(in: cursor, size: data_length);
    cursor.read_slice(data_length);

    // Version codes and payload sizes from ironrdp-pdu 0.7 CapabilityVersion.
    let expected_length = match version {
        0x0008_0004 | 0x0008_0105 | 0x000a_0002 | 0x000a_0200 | 0x000a_0301
        | 0x000a_0400 | 0x000a_0502 | 0x000a_0600 | 0x000a_0601 | 0x000a_0701 => 4,
        0x000a_0100 => 16,
        _ => {
            tracing::info!(version = format_args!("0x{version:08x}"), data_length,
                "GFX skipping unknown capability version");
            return Ok(None);
        }
    };
    if data_length != expected_length {
        return Err(invalid_field_err!("dataLength", "invalid known capability length"));
    }
    let mut bounded = ReadCursor::new(&entry[..CAPABILITY_HEADER_SIZE + data_length]);
    let capability = CapabilitySet::decode(&mut bounded)?;
    Ok(Some(capability))
}

pub(super) struct SelectedCapability {
    pub capability: CapabilitySet,
    pub avc420: bool,
    pub avc444: bool,
}

pub(super) fn select_capability(caps: &[CapabilitySet]) -> DecodeResult<SelectedCapability> {
    // Highest known protocol version wins. For duplicate versions, prioritize
    // AVC_DISABLED (0x20), then use the flag word for deterministic tie-breaking.
    let capability = caps.iter()
        .filter(|cap| !matches!(cap, CapabilitySet::Unknown(_)))
        .max_by_key(|cap| {
            let (version, flags) = capability_key(cap);
            (version, flags & 0x20 != 0, flags)
        })
        .ok_or_else(|| invalid_field_err!("capabilities", "no supported capability versions"))?
        .clone();
    let (avc420, avc444) = match &capability {
        CapabilitySet::V8_1 { flags } => (flags.contains(CapabilitiesV81Flags::AVC420_ENABLED), false),
        CapabilitySet::V10 { flags } | CapabilitySet::V10_2 { flags } => {
            let enabled = !flags.contains(CapabilitiesV10Flags::AVC_DISABLED);
            (enabled, enabled)
        }
        CapabilitySet::V10_1 => (true, true),
        CapabilitySet::V10_3 { flags } => {
            let enabled = !flags.contains(CapabilitiesV103Flags::AVC_DISABLED);
            (enabled, enabled)
        }
        CapabilitySet::V10_4 { flags } | CapabilitySet::V10_5 { flags }
        | CapabilitySet::V10_6 { flags } | CapabilitySet::V10_6Err { flags } => {
            let enabled = !flags.contains(CapabilitiesV104Flags::AVC_DISABLED);
            (enabled, enabled)
        }
        CapabilitySet::V10_7 { flags } => {
            let enabled = !flags.contains(CapabilitiesV107Flags::AVC_DISABLED);
            (enabled, enabled)
        }
        _ => (false, false),
    };
    Ok(SelectedCapability { capability, avc420, avc444 })
}

fn capability_key(cap: &CapabilitySet) -> (u8, u32) {
    match cap {
        CapabilitySet::V8 { flags } => (1, flags.bits()),
        CapabilitySet::V8_1 { flags } => (2, flags.bits()),
        CapabilitySet::V10 { flags } => (3, flags.bits()),
        CapabilitySet::V10_1 => (4, 0),
        CapabilitySet::V10_2 { flags } => (5, flags.bits()),
        CapabilitySet::V10_3 { flags } => (6, flags.bits()),
        CapabilitySet::V10_4 { flags } => (7, flags.bits()),
        CapabilitySet::V10_5 { flags } => (8, flags.bits()),
        CapabilitySet::V10_6 { flags } => (9, flags.bits()),
        CapabilitySet::V10_6Err { flags } => (10, flags.bits()),
        CapabilitySet::V10_7 { flags } => (11, flags.bits()),
        CapabilitySet::Unknown(_) => (0, 0),
    }
}
