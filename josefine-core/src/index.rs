use std::fs::OpenOptions;

use std::io::Write;
use std::path::PathBuf;

use memmap::MmapMut;

use crate::entry::Entry;

const MAX_BYTES_INDEX: u64 = 10 * 1024 * 1024;

pub struct Index {
    base_offset: u64,
    mmap: Box<MmapMut>,
}

impl Index {
    pub fn new(path: PathBuf, base_offset: u64) -> Index {
        let mut path = path.clone();
        path.push(format!("{}.index", base_offset));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .expect("Couldn't create index file.");

        file.set_len(MAX_BYTES_INDEX).unwrap();

        Index {
            base_offset,
            mmap: Box::new(unsafe { MmapMut::map_mut(&file).unwrap() }),
        }
    }

    pub fn write_at(&mut self, bytes: &[u8], offset: u64) {
        (&mut self.mmap[offset as usize..])
            .write_all(bytes)
            .unwrap();
    }

    pub fn write_entry(&mut self, entry: Entry) {
        let mut e = entry;
        e.offset -= self.base_offset;
        let bytes: Vec<u8> = e.into();
        self.write_at(bytes.as_ref(), e.offset);
    }

    pub fn read_entry(&self, offset: usize) -> Entry {
        let bytes = &self.mmap[offset..offset + 16];
        let mut entry = Entry::from(bytes);
        entry.offset += self.base_offset;
        entry
    }

    pub fn find_entry(&self, offset: u64) -> Option<Entry> {
        let idx = self
            .mmap
            .windows(16)
            .position(|x| Entry::from(x).offset == offset)?;
        let entry = Entry::from(&self.mmap[idx..idx + 16]);
        Some(entry)
    }

    pub fn sync(&self) {
        self.mmap.flush().unwrap();
    }
}

#[cfg(test)]
mod tests {

    use std::{env, path::{Path, PathBuf}};
    use std::fs;
    use std::fs::OpenOptions;
    use std::io::Read;
    use std::io::Seek;
    use std::io::SeekFrom;

    use crate::entry::Entry;

    fn before() -> PathBuf {
        let mut path = env::temp_dir();
        path.push("test");
        fs::create_dir_all(&path).expect("Couldn't create log dir");
        path
    }

    #[test]
    fn write_index() {
        let path = before();
        let mut index = super::Index::new(path.to_path_buf(), 0);
        let entry = Entry::new(0, 10);
        index.write_entry(entry);
    }

    #[test]
    fn read_index() {
        let path = before();
        let mut index = super::Index::new(path.to_path_buf(), 0);
        let entry = Entry::new(0, 10);
        index.write_entry(entry);
        assert_eq!(entry, index.read_entry(0));
    }

    #[test]
    fn relative_offset() {
        let path = before();
        let mut index = super::Index::new(path.to_path_buf(), 100);
        index.write_entry(Entry::new(115, 20));
        let mut path = path;
        path.push("100.index");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();

        file.seek(SeekFrom::Start(15)).unwrap();
        let mut bytes = [0u8; 16];
        file.read(&mut bytes).unwrap();

        let entry = Entry::from(bytes.as_ref());
        assert_eq!(entry.offset, 15);
        assert_eq!(entry.position, 20);
    }

    #[test]
    fn read_offset() {
        let path = before();
        let mut index = super::Index::new(path.to_path_buf(), 100);
        index.write_entry(Entry::new(115, 20));

        let entry = index.read_entry(115);
        assert_eq!(entry, Entry::new(115, 20));
    }
}
