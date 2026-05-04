//! # Mocks
//!
//! Fake implementations for testing:
//! - MockRng: Deterministic RNG for tests
//! - MockClock: Controllable clock
//! - MockStore: In-memory world store
//! - MockEventLog: In-memory event log

use std::collections::HashMap;

use sy_api::events::SimEvent;
use sy_core::ports::{
    IEventLog, IRng, ISimClock, IStateHasher, IWorldStore, StateHash, WorldSnapshot,
};
use sy_types::{EventId, RngSeed, SimError, SimResult, SimTime, Tick, WorldMeta};

// ============================================================================
// MockRng
// ============================================================================

/// Mock RNG that produces a deterministic sequence.
pub struct MockRng {
    seed: RngSeed,
    state: u64,
}

impl MockRng {
    pub fn new(seed: RngSeed) -> Self {
        MockRng {
            seed,
            state: seed.as_u64(),
        }
    }
}

impl IRng for MockRng {
    fn seed(&self) -> RngSeed {
        self.seed
    }

    fn state(&self) -> u64 {
        self.state
    }

    fn restore(&mut self, state: u64) {
        self.state = state;
    }

    fn next_u32(&mut self) -> u32 {
        // Simple LCG for testing
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_u64(&mut self) -> u64 {
        let hi = self.next_u32() as u64;
        let lo = self.next_u32() as u64;
        (hi << 32) | lo
    }
}

// ============================================================================
// MockClock
// ============================================================================

/// Mock clock for testing.
pub struct MockClock {
    current_tick: Tick,
}

impl MockClock {
    pub fn new() -> Self {
        MockClock {
            current_tick: Tick::ZERO,
        }
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ISimClock for MockClock {
    fn current_tick(&self) -> Tick {
        self.current_tick
    }

    fn sim_time(&self) -> SimTime {
        SimTime::from_ticks(self.current_tick)
    }

    fn advance(&mut self) -> Tick {
        self.current_tick = self.current_tick.next();
        self.current_tick
    }

    fn set_tick(&mut self, tick: Tick) {
        self.current_tick = tick;
    }

    fn should_tick(&self) -> bool {
        true
    }
}

// ============================================================================
// MockEventLog
// ============================================================================

/// In-memory event log for testing.
pub struct MockEventLog {
    events: Vec<SimEvent>,
    next_event_id: u64,
}

impl MockEventLog {
    pub fn new() -> Self {
        MockEventLog {
            events: Vec::new(),
            next_event_id: 1,
        }
    }
}

impl Default for MockEventLog {
    fn default() -> Self {
        Self::new()
    }
}

impl IEventLog for MockEventLog {
    fn append(&mut self, mut event: SimEvent) -> SimResult<SimEvent> {
        event.event_id = EventId::new(self.next_event_id);
        self.next_event_id += 1;
        self.events.push(event.clone());
        Ok(event)
    }

    fn append_batch(&mut self, events: Vec<SimEvent>) -> SimResult<Vec<SimEvent>> {
        let mut persisted = Vec::with_capacity(events.len());
        for event in events {
            persisted.push(self.append(event)?);
        }
        Ok(persisted)
    }

    fn read_from_event_id(&self, from_id: EventId) -> SimResult<Vec<SimEvent>> {
        Ok(self
            .events
            .iter()
            .filter(|e| e.event_id > from_id)
            .cloned()
            .collect())
    }

    fn read_all_valid(&self) -> SimResult<Vec<SimEvent>> {
        Ok(self.events.clone())
    }

    fn last_event_id(&self) -> EventId {
        if self.next_event_id > 1 {
            EventId::new(self.next_event_id - 1)
        } else {
            EventId::ZERO
        }
    }

    fn last_tick(&self) -> Option<Tick> {
        self.events.last().map(|e| e.tick)
    }

    fn truncate_after(&mut self, event_id: EventId) -> SimResult<()> {
        self.events.retain(|e| e.event_id <= event_id);
        if let Some(last) = self.events.last() {
            self.next_event_id = last.event_id.as_u64() + 1;
        } else {
            self.next_event_id = 1;
        }
        Ok(())
    }

    fn sync(&mut self) -> SimResult<()> {
        Ok(()) // No-op for in-memory
    }

    fn len(&self) -> usize {
        self.events.len()
    }
}

// ============================================================================
// MockWorldStore
// ============================================================================

/// In-memory world store for testing.
pub struct MockWorldStore {
    metas: HashMap<String, WorldMeta>,
    snapshots: HashMap<String, WorldSnapshot>,
}

impl MockWorldStore {
    pub fn new() -> Self {
        MockWorldStore {
            metas: HashMap::new(),
            snapshots: HashMap::new(),
        }
    }
}

impl Default for MockWorldStore {
    fn default() -> Self {
        Self::new()
    }
}

impl IWorldStore for MockWorldStore {
    fn exists(&self, world_id: &str) -> bool {
        self.metas.contains_key(world_id)
    }

    fn list_worlds(&self) -> SimResult<Vec<String>> {
        Ok(self.metas.keys().cloned().collect())
    }

    fn load_meta(&self, world_id: &str) -> SimResult<WorldMeta> {
        self.metas
            .get(world_id)
            .cloned()
            .ok_or_else(|| SimError::PersistenceError(format!("World not found: {}", world_id)))
    }

    fn save_meta(&mut self, meta: &WorldMeta) -> SimResult<()> {
        self.metas.insert(meta.world_id.clone(), meta.clone());
        Ok(())
    }

    fn load_snapshot(&self, world_id: &str) -> SimResult<WorldSnapshot> {
        self.snapshots
            .get(world_id)
            .cloned()
            .ok_or_else(|| SimError::PersistenceError(format!("Snapshot not found: {}", world_id)))
    }

    fn save_snapshot(&mut self, world_id: &str, snapshot: &WorldSnapshot) -> SimResult<()> {
        self.snapshots
            .insert(world_id.to_string(), snapshot.clone());
        Ok(())
    }

    fn delete_world(&mut self, world_id: &str) -> SimResult<()> {
        self.metas.remove(world_id);
        self.snapshots.remove(world_id);
        Ok(())
    }

    fn world_path(&self, world_id: &str) -> String {
        format!("mock://{}", world_id)
    }
}

// ============================================================================
// MockHasher
// ============================================================================

/// Simple hasher for testing.
pub struct MockHasher {
    hash: u64,
}

impl MockHasher {
    pub fn new() -> Self {
        MockHasher { hash: 0 }
    }
}

impl Default for MockHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl IStateHasher for MockHasher {
    fn reset(&mut self) {
        self.hash = 0;
    }

    fn update(&mut self, data: &[u8]) {
        // Simple FNV-like hash for testing
        for &byte in data {
            self.hash = self
                .hash
                .wrapping_mul(1099511628211)
                .wrapping_add(byte as u64);
        }
    }

    fn finalize(&self) -> StateHash {
        StateHash(self.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_rng_deterministic() {
        let mut rng1 = MockRng::new(RngSeed::new(42));
        let mut rng2 = MockRng::new(RngSeed::new(42));

        for _ in 0..10 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn mock_clock_advances() {
        let mut clock = MockClock::new();
        assert_eq!(clock.current_tick(), Tick::ZERO);
        clock.advance();
        assert_eq!(clock.current_tick(), Tick(1));
    }

    #[test]
    fn mock_event_log() {
        use sy_api::events::EventData;

        let mut log = MockEventLog::new();
        let event = SimEvent::new(
            Tick(1),
            EventData::TickProcessed {
                tick: Tick(1),
                sim_time: SimTime::ZERO,
                entities_processed: 0,
            },
        );

        let persisted = log.append(event).unwrap();
        assert_eq!(persisted.event_id, EventId::new(1));
        assert_eq!(log.len(), 1);
        assert_eq!(log.last_event_id(), EventId::new(1));
    }

    #[test]
    fn mock_event_log_read_from_event_id() {
        use sy_api::events::EventData;

        let mut log = MockEventLog::new();

        for i in 1..=5 {
            let event = SimEvent::new(
                Tick(i),
                EventData::TickProcessed {
                    tick: Tick(i),
                    sim_time: SimTime { units: i },
                    entities_processed: 0,
                },
            );
            log.append(event).unwrap();
        }

        let events = log.read_from_event_id(EventId::new(3)).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, EventId::new(4));
    }
}
