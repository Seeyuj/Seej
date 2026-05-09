#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use sy_api::{
    commands::EntityProperties,
    events::{EventData, SimEvent},
};
use sy_api::persistence::IEventLog;
use sy_infra::store::FileEventLog;
use sy_types::{EventId, RngSeed, SimTime, Tick, ZoneId};

#[derive(Arbitrary, Debug)]
struct Input {
    tick: u16,
    entities_processed: u16,
    rng_state_after: Option<u64>,
    batch_len: u8,
}

fuzz_target!(|input: Input| {
    let batch_len = usize::from(input.batch_len % 16) + 1;
    let tick = Tick(u64::from(input.tick));

    let events = (0..batch_len)
        .map(|offset| {
            SimEvent::new(
                tick,
                event_for(
                    input.tick,
                    input.entities_processed,
                    input.rng_state_after,
                    offset,
                ),
            )
        })
        .collect::<Vec<_>>();

    let dir = tempfile::tempdir().expect("tempdir must be available for WAL fuzzing");
    let path = dir.path().join("events.wal");
    let mut log = FileEventLog::new(&path).expect("new WAL must initialize");
    let persisted = log
        .append_batch(events)
        .expect("valid generated WAL batch must append");
    drop(log);

    let recovered = FileEventLog::new(&path)
        .expect("valid generated WAL must reopen")
        .read_all_valid()
        .expect("valid generated WAL must decode");

    assert_eq!(recovered.len(), persisted.len());
    for (index, event) in recovered.iter().enumerate() {
        assert_eq!(event.event_id, EventId::new((index + 1) as u64));
        assert_eq!(event.tick, persisted[index].tick);
    }
});

fn event_for(
    tick: u16,
    entities_processed: u16,
    rng_state_after: Option<u64>,
    offset: usize,
) -> EventData {
    match offset % 4 {
        0 => EventData::TickProcessed {
            tick: Tick(u64::from(tick)),
            sim_time: SimTime::from_ticks(Tick(u64::from(tick))),
            entities_processed: u32::from(entities_processed),
            rng_state_after,
        },
        1 => EventData::WorldCreated {
            world_id: format!("fuzz-world-{}", tick),
            name: "fuzz".to_string(),
            seed: RngSeed::new(u64::from(tick)),
        },
        2 => EventData::ZoneCreated {
            zone_id: ZoneId::new(u32::from(tick)),
            name: Some(format!("zone-{}", tick)),
        },
        _ => EventData::EntitySpawned {
            entity_id: sy_types::EntityId::new((offset + 1) as u64),
            kind: sy_types::EntityKind::Resource,
            position: sy_types::WorldPos::origin(),
            properties: EntityProperties {
                name: Some("resource".to_string()),
                amount: Some(u32::from(entities_processed)),
                health: Some(100),
            },
        },
    }
}
