use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::error::{Result, TosumuError};

pub(crate) const DEFAULT_MAX_REGISTERED_SNAPSHOTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapshotRegistryInfo {
    pub(crate) active: u64,
    pub(crate) maximum: u64,
    pub(crate) oldest_generation: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct SnapshotRegistry {
    maximum: usize,
    state: Mutex<RegistryState>,
}

#[derive(Debug, Default)]
struct RegistryState {
    next_registration_id: u64,
    registrations: BTreeMap<u64, u64>,
}

impl SnapshotRegistry {
    pub(crate) fn new(maximum: usize) -> Self {
        Self {
            maximum,
            state: Mutex::new(RegistryState::default()),
        }
    }

    pub(crate) fn register(self: &Arc<Self>, generation: u64) -> Result<SnapshotPin> {
        let mut state = self.lock()?;
        if state.registrations.len() >= self.maximum {
            return Err(TosumuError::SnapshotLimitReached {
                active: usize_to_u64(state.registrations.len()),
                maximum: usize_to_u64(self.maximum),
            });
        }

        let registration_id = state
            .next_registration_id
            .checked_add(1)
            .ok_or(TosumuError::Poisoned)?;
        state.next_registration_id = registration_id;
        state.registrations.insert(registration_id, generation);

        Ok(SnapshotPin {
            registry: Arc::clone(self),
            registration_id,
            generation,
        })
    }

    pub(crate) fn info(&self) -> Result<SnapshotRegistryInfo> {
        let state = self.lock()?;
        Ok(SnapshotRegistryInfo {
            active: usize_to_u64(state.registrations.len()),
            maximum: usize_to_u64(self.maximum),
            oldest_generation: state.registrations.values().copied().min(),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, RegistryState>> {
        self.state.lock().map_err(|_| TosumuError::Poisoned)
    }

    fn unregister(&self, registration_id: u64) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.registrations.remove(&registration_id);
    }
}

impl Default for SnapshotRegistry {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_REGISTERED_SNAPSHOTS)
    }
}

#[derive(Debug)]
pub(crate) struct SnapshotPin {
    registry: Arc<SnapshotRegistry>,
    registration_id: u64,
    generation: u64,
}

impl SnapshotPin {
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn belongs_to(&self, registry: &Arc<SnapshotRegistry>) -> bool {
        Arc::ptr_eq(&self.registry, registry)
    }
}

impl Drop for SnapshotPin {
    fn drop(&mut self) {
        self.registry.unregister(self.registration_id);
    }
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{codes, ErrorStatus};

    #[test]
    fn tracks_the_oldest_registered_generation_until_each_pin_drops() {
        let registry = Arc::new(SnapshotRegistry::default());

        let newer = registry.register(12).unwrap();
        let oldest = registry.register(7).unwrap();
        let same_oldest = registry.register(7).unwrap();

        assert_eq!(newer.generation(), 12);
        assert_eq!(oldest.generation(), 7);
        assert_eq!(
            registry.info().unwrap(),
            SnapshotRegistryInfo {
                active: 3,
                maximum: 64,
                oldest_generation: Some(7),
            }
        );

        drop(oldest);
        assert_eq!(registry.info().unwrap().oldest_generation, Some(7));
        drop(same_oldest);
        assert_eq!(registry.info().unwrap().oldest_generation, Some(12));
        drop(newer);
        assert_eq!(registry.info().unwrap().oldest_generation, None);
    }

    #[test]
    fn rejects_registration_at_the_limit_and_recovers_after_drop() {
        let registry = Arc::new(SnapshotRegistry::new(2));
        let first = registry.register(3).unwrap();
        let _second = registry.register(4).unwrap();

        let error = registry.register(5).unwrap_err();
        let report = error.error_report();
        assert_eq!(report.code, codes::SNAPSHOT_LIMIT_REACHED);
        assert_eq!(report.status, ErrorStatus::Busy);
        assert_eq!(report.detail_u64("active"), Some(2));
        assert_eq!(report.detail_u64("maximum"), Some(2));

        drop(first);
        let replacement = registry.register(5).unwrap();
        assert_eq!(replacement.generation(), 5);
    }

    #[test]
    fn zero_capacity_registry_fails_before_allocating_a_registration() {
        let registry = Arc::new(SnapshotRegistry::new(0));

        let error = registry.register(1).unwrap_err();
        assert!(matches!(
            error,
            TosumuError::SnapshotLimitReached {
                active: 0,
                maximum: 0
            }
        ));
        assert_eq!(registry.info().unwrap().active, 0);
    }
}
