use std::convert::{From, TryInto};
use std::io::Cursor;
use std::fs::read;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Entry {
    pub offset: u64,
    pub position: u64,
}

impl Entry {
    pub fn new(offset: u64, position: u64) -> Entry {
        Entry { offset, position }
    }
}

impl From<Entry> for Vec<u8> {
    fn from(entry: Entry) -> Self {
        let mut bytes = vec![];
        bytes.extend_from_slice(&entry.offset.to_be_bytes());
        bytes.extend_from_slice(&entry.position.to_be_bytes());
        bytes
    }
}

impl From<&[u8]> for Entry {
    fn from(bytes: &[u8]) -> Self {
        let bytes: &[u8; 16] = bytes.try_into().unwrap();
        let offset = u64::from_be_bytes( bytes[0..8].try_into().unwrap());
        let position = u64::from_be_bytes( bytes[8..16].try_into().unwrap());
        Entry { offset, position }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn entry_to_bytes() {
        let bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0A,
        ];
        let entry = super::Entry::new(0, 10);
        let res_bytes: Vec<u8> = entry.into();
        assert_eq!(bytes, res_bytes);
        let bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x6F,
        ];
        let entry = super::Entry::new(0, 111);
        let res_bytes: Vec<u8> = entry.into();
        assert_eq!(bytes, res_bytes);
    }

    #[test]
    fn byes_to_entry() {
        let bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0A,
        ];
        let entry = super::Entry::new(0, 10);
        assert_eq!(entry, super::Entry::from(bytes.as_ref()));
        let bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x6F,
        ];
        let entry = super::Entry::new(0, 111);
        assert_eq!(entry, super::Entry::from(bytes.as_ref()));
    }

    #[test]
    #[should_panic]
    fn panics_length() {
        let bytes = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
        ];
        super::Entry::from(bytes.as_ref());
    }
}
