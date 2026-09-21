use core::{cmp, fmt};

use ironrdp_pdu::fast_path::{EncryptionFlags, FastPathHeader, FastPathUpdatePdu, Fragmentation, UpdateCode};
use ironrdp_pdu::{Encode as _, WriteCursor};

// this is the maximum amount of data (not including headers) we can send in a single TS_FP_UPDATE_PDU
const MAX_FASTPATH_UPDATE_SIZE: usize = 16_374;

const FASTPATH_HEADER_SIZE: usize = 6;

pub(crate) struct UpdateFragmenter {
    code: UpdateCode,
    index: usize,
    data: Vec<u8>,
    position: usize,
}

impl fmt::Debug for UpdateFragmenter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UpdateFragmenter")
            .field("len", &self.data.len())
            .finish()
    }
}

impl UpdateFragmenter {
    pub(crate) fn new(code: UpdateCode, data: Vec<u8>) -> Self {
        Self {
            code,
            index: 0,
            data,
            position: 0,
        }
    }

    pub(crate) fn size_hint(&self) -> usize {
        FASTPATH_HEADER_SIZE + cmp::min(self.data.len(), MAX_FASTPATH_UPDATE_SIZE)
    }

    pub(crate) fn next(&mut self, dst: &mut [u8]) -> Option<usize> {
        let (consumed, written) = self.encode_next(dst)?;
        self.position += consumed;
        self.index = self.index.checked_add(1)?;
        Some(written)
    }

    fn encode_next(&mut self, dst: &mut [u8]) -> Option<(usize, usize)> {
        match self.data.len() - self.position {
            0 => None,

            1..=MAX_FASTPATH_UPDATE_SIZE => {
                let frag = if self.index > 0 {
                    Fragmentation::Last
                } else {
                    Fragmentation::Single
                };

                self.encode_fastpath(frag, &self.data[self.position..], dst)
                    .map(|written| (self.data.len() - self.position, written))
            }

            _ => {
                let frag = if self.index > 0 {
                    Fragmentation::Next
                } else {
                    Fragmentation::First
                };

                self.encode_fastpath(
                    frag,
                    &self.data[self.position..MAX_FASTPATH_UPDATE_SIZE + self.position],
                    dst,
                )
                .map(|written| (MAX_FASTPATH_UPDATE_SIZE, written))
            }
        }
    }

    fn encode_fastpath(&self, frag: Fragmentation, data: &[u8], dst: &mut [u8]) -> Option<usize> {
        let mut cursor = WriteCursor::new(dst);

        let update = FastPathUpdatePdu {
            fragmentation: frag,
            update_code: self.code,
            compression_flags: None,
            compression_type: None,
            data,
        };

        let header = FastPathHeader::new(EncryptionFlags::empty(), update.size());

        header.encode(&mut cursor).ok()?;
        update.encode(&mut cursor).ok()?;

        Some(cursor.pos())
    }
}
