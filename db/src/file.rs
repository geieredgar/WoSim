use std::{
    io::{self, Read, Seek, Write},
    ops::{Deref, DerefMut},
};

use crate::{
    cursor::{reallocate, PageLookup},
    lock::Lock,
    page::{PageNr, PAGE_SIZE},
    reference::DatabaseRef,
};

#[derive(Clone, Default, Copy)]
#[repr(C)]
pub struct FileHeader {
    pub root: PageNr,
    pub len: u64,
}

impl FileHeader {
    fn pages(&self) -> usize {
        (self.len as usize + PAGE_SIZE - 1) / PAGE_SIZE
    }
}

pub struct File {
    header: FileHeader,
    database: DatabaseRef,
}

impl File {
    pub fn new(database: DatabaseRef) -> Self {
        Self {
            header: FileHeader::default(),
            database,
        }
    }

    pub(crate) fn from_header(header: FileHeader, database: DatabaseRef) -> Self {
        Self { header, database }
    }

    pub fn deserialize(reader: &mut impl Read, database: DatabaseRef) -> io::Result<Self> {
        let mut bytes = [0; 4];
        reader.read_exact(&mut bytes)?;
        let root = u32::from_ne_bytes(bytes);
        let mut bytes = [0; 8];
        reader.read_exact(&mut bytes)?;
        let len = u64::from_ne_bytes(bytes);
        Ok(Self {
            header: FileHeader { root, len },
            database,
        })
    }

    pub fn serialize(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(&self.header.root.to_ne_bytes())?;
        writer.write_all(&self.header.len.to_ne_bytes())?;
        Ok(())
    }

    pub(crate) fn header(&self) -> FileHeader {
        self.header
    }

    pub fn read(&self) -> ReadFileGuard<'_> {
        ReadFileGuard {
            header: &self.header,
            lock: self.database.lock(),
            pos: 0,
            lookup: PageLookup::Invalid,
        }
    }

    pub fn write(&mut self) -> WriteFileGuard<'_> {
        WriteFileGuard {
            header: &mut self.header,
            lock: self.database.lock(),
            pos: 0,
            lookup: PageLookup::Invalid,
        }
    }
}

impl<'a, H: DerefMut<Target = FileHeader>> FileGuard<'a, H> {
    pub fn set_len(&mut self, size: u64) {
        let current_pages = self.header.pages();
        self.header.len = size;
        let new_pages = self.header.pages();
        reallocate(&mut self.header.root, current_pages, new_pages, &self.lock);
        self.lookup = PageLookup::Invalid;
    }
}

impl<'a, H: Deref<Target = FileHeader>> Seek for FileGuard<'a, H> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(pos) => self.pos = pos,
            io::SeekFrom::End(offset) => self.pos = (self.header.len as i64 + offset) as u64,
            io::SeekFrom::Current(offset) => self.pos = ((self.pos as i64) + offset) as u64,
        }
        Ok(self.pos)
    }
}

impl<'a, H: Deref<Target = FileHeader>> Read for FileGuard<'a, H> {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let len = buf
            .len()
            .min((self.header.len as usize).saturating_sub(self.pos as usize));
        buf = buf.split_at_mut(len).0;
        while !buf.is_empty() {
            let index = self.pos as usize / PAGE_SIZE;
            let offset = self.pos as usize % PAGE_SIZE;
            let n = buf.len().min(PAGE_SIZE - offset);
            let (a, b) = buf.split_at_mut(n);
            let page = self
                .lookup
                .get(self.header.root, self.header.pages(), index, &self.lock);
            a.copy_from_slice(&page[offset..offset + n]);
            self.pos += n as u64;
            buf = b;
        }
        Ok(len)
    }
}

impl<'a, H: DerefMut<Target = FileHeader>> Write for FileGuard<'a, H> {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        let size = self.pos + buf.len() as u64;
        if size > self.header.len as u64 {
            self.set_len(size)
        }
        let pages = self.header.pages();
        while !buf.is_empty() {
            let index = self.pos as usize / PAGE_SIZE;
            let offset = self.pos as usize % PAGE_SIZE;
            let n = buf.len().min(PAGE_SIZE - offset);
            let (a, b) = buf.split_at(n);
            let page = unsafe {
                self.lookup
                    .get_mut(&mut self.header.root, pages, index, &self.lock)
                    .as_mut()
                    .unwrap()
            };
            page[offset..offset + n].copy_from_slice(a);
            self.pos += n as u64;
            buf = b;
        }
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct FileGuard<'a, H: Deref<Target = FileHeader>> {
    header: H,
    pos: u64,
    lookup: PageLookup,
    lock: Lock<'a>,
}

pub type ReadFileGuard<'a> = FileGuard<'a, &'a FileHeader>;
pub type WriteFileGuard<'a> = FileGuard<'a, &'a mut FileHeader>;

/*

impl Drop for File {
    fn drop(&mut self) {
        let lock = self.database.lock();
        if !lock.is_closing() {
            reallocate(self.header.root, self.header.pages(), 0, &lock)
        }
    }
}

impl Seek for File {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(pos) => self.pos = pos,
            io::SeekFrom::End(offset) => self.pos = self.header.len as u64 + offset,
            io::SeekFrom::Current(offset) => self.pos += offset,
        }
        if let Some(cursor) = self.cursor.as_mut() {
            if self.pos < self.header.len as u64 {
                unsafe { cursor.seek(self.pos as u32 / PAGE_SIZE_U32, &self.database.lock()) }
            } else {
                self.cursor = None
            }
        }
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();
        let end = self.cursor.pos() as usize * PAGE_SIZE + self.page_offset + len;
        if end > self.header.len as usize {
            self.resize(end)
        }
        while !buf.is_empty() {
            let mut n = self.page_remaining();
            if n == 0 {
                self.cursor.seek(self.cursor.pos() + 1);
                self.page_offset = 0;
                n = PAGE_SIZE;
            }
            n = n.min(buf.len());
            let (a, b) = buf.split_at(n);
            self.cursor.page_mut()[self.page_offset..self.page_offset + n].copy_from_slice(a);
            self.page_offset += n;
            buf = b;
        }
        self.header.root = self.cursor.root();
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct FileReader<'a> {
    file: &'a File,
    pos: usize,
}

impl<'a> FileReader<'a> {
    fn new(header: &'a FileHeader, lock: Lock<'a>) -> Self {
        Self {
            header,
            cursor: Cursor::new(header.root, header.pages(), lock),
            page_offset: 0,
        }
    }
}

impl<'a> FileWriter<'a> {
    fn new(header: &'a mut FileHeader, lock: Lock<'a>) -> Self {
        let cursor = Cursor::new(header.root, header.pages(), lock);
        Self {
            header,
            cursor,
            page_offset: 0,
        }
    }

    fn resize(&mut self, len: usize) {
        assert!(len < 2usize.pow(u32::BITS));
        self.header.len = len as u32;
        self.cursor.resize(self.header.pages());
        self.header.root = self.cursor.root();
    }
}

impl<'a> Write for FileWriter<'a> {
    fn write(&mut self, mut buf: &[u8]) -> std::io::Result<usize> {}

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> Read for FileReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = buf.len().min(self.remaining());
        let mut buf = buf.split_at_mut(len).0;
        while !buf.is_empty() {
            let mut n = self.page_remaining();
            if n == 0 {
                let pos = self.cursor().pos();
                self.cursor.seek(pos + 1);
                self.page_offset = 0;
                n = PAGE_SIZE;
            }
            n = n.min(buf.len());
            let (a, b) = buf.split_at_mut(n);
            a.copy_from_slice(&self.cursor.page()[self.page_offset..self.page_offset + n]);
            buf = b;
        }
        Ok(len)
    }
}

*/
