use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use crate::rules::{Command, Write};
use crate::state::{Key, Root, Value, ENTRY_BYTES};
use crate::JournalMode;

const COMMAND: u8 = 1;
const CLOSURE: u8 = 2;
const PREFIX_BYTES: usize = 77;
const BASE_BODY_BYTES: usize = PREFIX_BYTES + 4;
const COMMAND_BYTES: usize = 40;
const FRAME_OVERHEAD: u64 = 8;

#[derive(Clone, Copy)]
pub(crate) struct FrameLayout {
    command: bool,
    body_len: u32,
    writes: u32,
}

impl FrameLayout {
    pub(crate) fn new(command: bool, writes: u64) -> Result<Self, String> {
        let fixed = BASE_BODY_BYTES + if command { COMMAND_BYTES } else { 0 };
        let body_len = writes
            .checked_mul(ENTRY_BYTES as u64)
            .and_then(|size| size.checked_add(fixed as u64))
            .and_then(|size| u32::try_from(size).ok())
            .ok_or("journal frame body exceeds u32")?;
        Ok(Self {
            command,
            body_len,
            writes: u32::try_from(writes).map_err(|_| "journal write count exceeds u32")?,
        })
    }

    pub(crate) fn bytes(self) -> u64 {
        u64::from(self.body_len) + FRAME_OVERHEAD
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Command(Command),
    Closure { seq: u32 },
}

impl Action {
    fn kind(self) -> u8 {
        match self {
            Self::Command(_) => COMMAND,
            Self::Closure { .. } => CLOSURE,
        }
    }

    fn seq(self) -> u32 {
        match self {
            Self::Command(cmd) => cmd.seq,
            Self::Closure { seq } => seq,
        }
    }
}

pub(crate) struct Resolution {
    pub(crate) action: Action,
    pub(crate) tick: u64,
    pub(crate) pre_root: Root,
    pub(crate) post_root: Root,
}

pub(crate) fn encode<P>(
    target: &mut Vec<u8>,
    frame: &Resolution,
    layout: FrameLayout,
    mode: JournalMode,
    writes: &[Write<P>],
) {
    let writes = match mode {
        JournalMode::Delta => writes,
        JournalMode::Root => &[],
    };
    debug_assert_eq!(writes.len(), layout.writes as usize);
    target.extend_from_slice(&layout.body_len.to_le_bytes());
    let body_start = target.len();
    target.push(frame.action.kind());
    target.extend_from_slice(&frame.tick.to_le_bytes());
    target.extend_from_slice(&frame.action.seq().to_le_bytes());
    target.extend_from_slice(&frame.pre_root);
    target.extend_from_slice(&frame.post_root);
    if let Action::Command(cmd) = frame.action {
        for index in cmd.keys {
            target.extend_from_slice(&index.to_le_bytes());
        }
        target.extend_from_slice(&cmd.amount.to_le_bytes());
    }
    target.extend_from_slice(&layout.writes.to_le_bytes());
    for write in writes {
        target.extend_from_slice(&write.key);
        target.extend_from_slice(&write.new);
    }
    debug_assert_eq!(target.len() - body_start, layout.body_len as usize);
    let crc = crc32fast::hash(&target[body_start..]);
    target.extend_from_slice(&crc.to_le_bytes());
}

pub(crate) struct JournalReader {
    source: BufReader<File>,
    file_len: u64,
    offset: u64,
    body: Vec<u8>,
}

pub(crate) struct Decoded<'a> {
    pub(crate) frame: Resolution,
    entries: &'a [u8],
}

impl Decoded<'_> {
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&Key, &Value)> + Clone {
        self.entries.chunks_exact(ENTRY_BYTES).map(|entry| {
            (
                entry[..16].try_into().unwrap(),
                entry[16..].try_into().unwrap(),
            )
        })
    }
}

impl JournalReader {
    pub(crate) fn new(mut source: File, offset: u64) -> Result<Self, String> {
        let file_len = source.metadata().map_err(|e| e.to_string())?.len();
        if offset > file_len {
            return Err("checkpoint journal offset exceeds the journal length".into());
        }
        source
            .seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            source: BufReader::new(source),
            file_len,
            offset,
            body: Vec::new(),
        })
    }

    pub(crate) fn next(
        &mut self,
        keys_count: u64,
        expected: FrameLayout,
        tick: u64,
        seq: u32,
    ) -> Result<Decoded<'_>, String> {
        let is_command = expected.command;
        let name = if is_command { "command" } else { "closure" };
        if self.offset == self.file_len {
            return Err(format!("missing {name} at tick {tick}, seq {seq}"));
        }
        if self.file_len - self.offset < FRAME_OVERHEAD {
            return Err(format!("truncated {name} at tick {tick}, seq {seq}"));
        }
        let mut size = [0; 4];
        self.source
            .read_exact(&mut size)
            .map_err(|e| e.to_string())?;
        let size = u32::from_le_bytes(size);
        if size != expected.body_len {
            return Err(format!(
                "invalid {name} body length at tick {tick}, seq {seq}"
            ));
        }
        if self.file_len - self.offset < expected.bytes() {
            return Err(format!("truncated {name} at tick {tick}, seq {seq}"));
        }
        self.body.resize(size as usize, 0);
        self.source
            .read_exact(&mut self.body)
            .map_err(|e| e.to_string())?;
        let mut crc = [0; 4];
        self.source
            .read_exact(&mut crc)
            .map_err(|e| e.to_string())?;
        if u32::from_le_bytes(crc) != crc32fast::hash(&self.body) {
            return Err(format!("journal CRC mismatch at tick {tick}, seq {seq}"));
        }
        self.offset += expected.bytes();
        let body = &self.body;
        let actual_tick = u64::from_le_bytes(body[1..9].try_into().unwrap());
        let actual_seq = u32::from_le_bytes(body[9..13].try_into().unwrap());
        let expected_kind = if is_command { COMMAND } else { CLOSURE };
        if body[0] != expected_kind || actual_tick != tick || actual_seq != seq {
            return Err(format!("expected {name} at tick {tick}, seq {seq}"));
        }
        let pre_root = body[13..45].try_into().unwrap();
        let post_root = body[45..PREFIX_BYTES].try_into().unwrap();
        let (action, count_at) = if is_command {
            let keys: [u64; 4] = std::array::from_fn(|i| {
                let start = PREFIX_BYTES + 8 * i;
                u64::from_le_bytes(body[start..start + 8].try_into().unwrap())
            });
            if keys
                .iter()
                .enumerate()
                .any(|(i, key)| *key >= keys_count || keys[..i].contains(key))
            {
                return Err(format!("invalid command keys at tick {tick}, seq {seq}"));
            }
            let count_at = PREFIX_BYTES + COMMAND_BYTES;
            let amount = u64::from_le_bytes(body[count_at - 8..count_at].try_into().unwrap());
            if !(1..=100).contains(&amount) {
                return Err(format!("invalid command amount at tick {tick}, seq {seq}"));
            }
            (Action::Command(Command { seq, keys, amount }), count_at)
        } else {
            (Action::Closure { seq }, PREFIX_BYTES)
        };
        let writes = u32::from_le_bytes(body[count_at..count_at + 4].try_into().unwrap());
        if writes != expected.writes {
            return Err(format!("invalid write count at tick {tick}, seq {seq}"));
        }
        Ok(Decoded {
            frame: Resolution {
                action,
                tick,
                pre_root,
                post_root,
            },
            entries: &body[count_at + 4..],
        })
    }

    pub(crate) fn finish(self) -> Result<u64, String> {
        if self.offset != self.file_len {
            return Err(format!(
                "extra journal bytes after final closure at offset {}",
                self.offset
            ));
        }
        Ok(self.offset)
    }
}
