// SPDX-License-Identifier: GPL-3.0-or-later
use std::{fs::File, io::{self, Read, Seek, SeekFrom}, path::Path};
use same_file::Handle;
const CHUNK: usize = 64 * 1024;
const MAX_LINE: usize = 1024 * 1024;
const ANCHOR: usize = 64;

pub struct Batch { pub reset: bool, pub bytes: Vec<u8>, pub size: u64 }
pub struct Tail { file: Option<File>, identity: Option<Handle>, pub offset: u64, anchor: Vec<u8>, needs_reset: bool }
impl Tail {
    pub fn new() -> Self { Self { file: None, identity: None, offset: 0, anchor: Vec::new(), needs_reset: false } }
    pub fn poll(&mut self, path: &Path) -> io::Result<Batch> {
        // Compare file identities, not just sizes; keep the actual reader handle alive.
        let candidate = File::open(path)?;
        let id = Handle::from_file(candidate.try_clone()?)?;
        let size = candidate.metadata()?.len();
        let mut reset = self.identity.as_ref() != Some(&id) || size < self.offset;
        if !reset && !self.anchor.is_empty() {
            let f = self.file.as_mut().ok_or_else(|| io::Error::other("reader missing"))?;
            f.seek(SeekFrom::Start(self.offset - self.anchor.len() as u64))?;
            let mut check = vec![0; self.anchor.len()];
            match f.read_exact(&mut check) {
                Ok(()) => reset = check != self.anchor,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => reset = true,
                Err(e) => return Err(e),
            }
        }
        if reset {
            self.file = Some(candidate);
            self.identity = Some(id);
            self.offset = 0;
            self.anchor.clear();
            self.needs_reset = true;
        }
        let f = self.file.as_mut().ok_or_else(|| io::Error::other("reader missing"))?;
        f.seek(SeekFrom::Start(self.offset))?;
        let mut bytes = vec![0; CHUNK.min(size.saturating_sub(self.offset) as usize)];
        let n = f.read(&mut bytes)?;
        bytes.truncate(n);
        self.offset += n as u64;
        self.anchor.extend_from_slice(&bytes);
        if self.anchor.len() > ANCHOR { self.anchor.drain(..self.anchor.len()-ANCHOR); }
        let reset = std::mem::take(&mut self.needs_reset);
        Ok(Batch { reset, bytes, size })
    }
}

pub struct Framer { partial: Vec<u8>, offset: u64, start: u64, dropping: bool }
pub enum Line { Complete(u64, Vec<u8>), Oversized }
impl Framer {
    pub fn new() -> Self { Self { partial: Vec::new(), offset: 0, start: 0, dropping: false } }
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Line> {
        let mut lines = Vec::new();
        for &b in bytes {
            self.offset += 1;
            if b == b'\n' {
                if !self.dropping { lines.push(Line::Complete(self.start, std::mem::take(&mut self.partial))); }
                self.dropping = false;
                self.start = self.offset;
            } else if !self.dropping {
                if self.partial.len() == MAX_LINE {
                    self.partial.clear(); self.dropping = true; lines.push(Line::Oversized);
                } else { self.partial.push(b); }
            }
        }
        lines
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn split_line_and_utf8_are_buffered() {
        let mut f = Framer::new();
        assert!(f.push(&[b'a', 0xe4]).is_empty());
        let r = f.push(&[0xb8, 0xad, b'\n', b'b', b'\n']);
        assert!(matches!(&r[0], Line::Complete(0, x) if x == "a中".as_bytes()));
        assert!(matches!(&r[1], Line::Complete(5, x) if x == b"b"));
    }
    #[test]
    fn oversized_line_recovers_at_newline() {
        let mut f = Framer::new();
        assert!(matches!(f.push(&vec![b'x'; MAX_LINE + 1]).as_slice(), [Line::Oversized]));
        assert!(matches!(f.push(b"\nok\n").as_slice(), [Line::Complete(_, x)] if x == b"ok"));
    }
    #[test]
    fn missing_path_then_reconnect_keeps_offset_and_partial_line() {
        let d=tempfile::tempdir().unwrap();let path=d.path().join("EE.log");let backup=d.path().join("held.log");
        std::fs::write(&path,b"first\npar").unwrap();
        let mut tail=Tail::new();let mut framer=Framer::new();
        let batch=tail.poll(&path).unwrap();assert!(batch.reset);
        assert!(matches!(framer.push(&batch.bytes).as_slice(),[Line::Complete(0,x)] if x==b"first"));
        std::fs::rename(&path,&backup).unwrap();assert!(tail.poll(&path).is_err());
        std::fs::OpenOptions::new().append(true).open(&backup).unwrap().write_all(b"tial\n").unwrap();
        std::fs::rename(&backup,&path).unwrap();let batch=tail.poll(&path).unwrap();assert!(!batch.reset);
        assert!(matches!(framer.push(&batch.bytes).as_slice(),[Line::Complete(6,x)] if x==b"partial"));
    }
    #[test]
    fn append_truncate_regrow_and_replacement() {
        let d = tempfile::tempdir().unwrap(); let p = d.path().join("EE.log");
        let mut t = Tail::new();
        assert!(t.poll(&p).is_err());
        std::fs::write(&p, b"old\n").unwrap();
        assert!(t.poll(&p).unwrap().reset);
        std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"next\n").unwrap();
        assert_eq!(t.poll(&p).unwrap().bytes, b"next\n");
        std::fs::write(&p, b"replacement after truncation\n").unwrap();
        let b = t.poll(&p).unwrap(); assert!(b.reset); assert_eq!(b.bytes, b"replacement after truncation\n");
        std::fs::rename(&p, d.path().join("EE.old.log")).unwrap();
        std::fs::write(&p, b"brand new file much larger than the previous one\n").unwrap();
        assert!(t.poll(&p).unwrap().reset);
    }
}
