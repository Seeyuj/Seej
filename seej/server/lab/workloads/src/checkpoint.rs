use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::partitioned::Records;
use crate::state::{key_at, Key, Root, Value, ENTRY_BYTES, PARTITION_SIZE};
use crate::{Config, Layout, StateLayout};

const MAGIC: &[u8; 8] = b"SLABC1\0\0";
const HEADER_BYTES: usize = 64;
pub(crate) const CHECKPOINT_OVERHEAD: u64 = HEADER_BYTES as u64 + 4;

pub(crate) trait Snapshot {
    fn root(&self) -> Root;
    fn count(&self) -> u64;
    fn entries(&self) -> impl Iterator<Item = (&Key, &Value)>;
    fn full_scan(&self) -> u128;
    fn checked_acc_root(&self, expected_sum: u64) -> Result<Root, String>;
}

pub(crate) enum Entries {
    Btree(BTreeMap<Key, Value>),
    Partitioned(Records),
}

pub(crate) struct Checkpoint {
    pub(crate) root: Root,
    pub(crate) journal_offset: u64,
    pub(crate) entries: Entries,
}

pub(crate) fn write(
    dir: &Path,
    tick: u64,
    offset: u64,
    state: &impl Snapshot,
) -> Result<u64, String> {
    let tmp = dir.join("checkpoint.tmp");
    let file = File::create_new(&tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    let bytes =
        write_contents(file, tick, offset, state).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, dir.join("checkpoint.bin"))
        .map_err(|e| format!("rename {}: {e}", tmp.display()))?;
    Ok(bytes)
}

fn write_contents(
    file: File,
    tick: u64,
    offset: u64,
    state: &impl Snapshot,
) -> std::io::Result<u64> {
    let mut writer = BufWriter::new(file);
    let mut crc = crc32fast::Hasher::new();
    let mut header = [0; HEADER_BYTES];
    header[..8].copy_from_slice(MAGIC);
    header[8..16].copy_from_slice(&tick.to_le_bytes());
    header[16..48].copy_from_slice(&state.root());
    header[48..56].copy_from_slice(&offset.to_le_bytes());
    header[56..].copy_from_slice(&state.count().to_le_bytes());
    writer.write_all(&header)?;
    crc.update(&header);
    let mut entry = [0; ENTRY_BYTES];
    for (key, value) in state.entries() {
        entry[..16].copy_from_slice(key);
        entry[16..].copy_from_slice(value);
        writer.write_all(&entry)?;
        crc.update(&entry);
    }
    writer.write_all(&crc.finalize().to_le_bytes())?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(writer.get_ref().metadata()?.len())
}

pub(crate) fn read(dir: &Path, cfg: &Config, layout: &Layout) -> Result<Checkpoint, String> {
    let path = dir.join("checkpoint.bin");
    let file = File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    read_contents(file, cfg, layout).map_err(|err| format!("{}: {err}", path.display()))
}

fn read_contents(file: File, cfg: &Config, layout: &Layout) -> Result<Checkpoint, String> {
    if file.metadata().map_err(|e| e.to_string())?.len() != layout.checkpoint_bytes {
        return Err("invalid checkpoint length".into());
    }
    let mut reader = BufReader::new(file);
    let mut crc = crc32fast::Hasher::new();
    let mut header = [0; HEADER_BYTES];
    reader.read_exact(&mut header).map_err(|e| e.to_string())?;
    if &header[..8] != MAGIC {
        return Err("invalid checkpoint magic".into());
    }
    let tick = u64::from_le_bytes(header[8..16].try_into().unwrap());
    if tick != cfg.checkpoint_at {
        return Err("checkpoint tick does not match config".into());
    }
    let root = header[16..48].try_into().unwrap();
    let journal_offset = u64::from_le_bytes(header[48..56].try_into().unwrap());
    if journal_offset != tick * layout.tick_bytes {
        return Err("invalid checkpoint journal offset".into());
    }
    let count = u64::from_le_bytes(header[56..].try_into().unwrap());
    if count != cfg.keys {
        return Err("checkpoint count does not match config".into());
    }
    crc.update(&header);
    let mut entries = match cfg.layout {
        StateLayout::Btree => Entries::Btree(BTreeMap::new()),
        StateLayout::Partitioned => {
            Entries::Partitioned(Records::new(layout.partitions, PARTITION_SIZE as usize))
        }
    };
    let mut entry = [0; ENTRY_BYTES];
    for i in 0..count {
        reader.read_exact(&mut entry).map_err(|e| e.to_string())?;
        let key = key_at(i);
        if entry[..16] != key {
            return Err(format!("invalid checkpoint key at index {i}"));
        }
        crc.update(&entry);
        match &mut entries {
            Entries::Btree(entries) => {
                entries.insert(key, entry[16..].try_into().unwrap());
            }
            Entries::Partitioned(records) => records.push_sorted(entry),
        }
    }
    let mut checksum = [0; 4];
    reader
        .read_exact(&mut checksum)
        .map_err(|e| e.to_string())?;
    if u32::from_le_bytes(checksum) != crc.finalize() {
        return Err("checkpoint CRC mismatch".into());
    }
    Ok(Checkpoint {
        root,
        journal_offset,
        entries,
    })
}
