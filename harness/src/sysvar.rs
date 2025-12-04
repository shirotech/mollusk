//! Module for working with Solana sysvars.

use {
    solana_clock::{Clock, Slot, UnixTimestamp},
    solana_epoch_rewards::EpochRewards,
    solana_epoch_schedule::EpochSchedule,
    solana_hash::Hash,
    solana_program_runtime::sysvar_cache::SysvarCache,
    solana_rent::{
        DEFAULT_BURN_PERCENT, DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR, Rent,
    },
    solana_slot_hashes::{MAX_ENTRIES as SLOT_HASHES_MAX_ENTRIES, SlotHashes},
    solana_stake_interface::stake_history::{StakeHistory, StakeHistoryEntry},
    solana_sysvar::{self, last_restart_slot::LastRestartSlot},
    solana_sysvar_id::SysvarId,
    std::{
        mem::MaybeUninit,
        time::{SystemTime, UNIX_EPOCH},
    },
};

// Agave's sysvar cache is difficult to work with, so Mollusk offers a wrapper
// around it for modifying its contents.
/// Mollusk sysvars.
pub struct Sysvars {
    pub clock: Clock,
    pub epoch_rewards: EpochRewards,
    pub epoch_schedule: EpochSchedule,
    pub last_restart_slot: LastRestartSlot,
    pub rent: Rent,
    pub slot_hashes: SlotHashes,
    pub stake_history: StakeHistory,
    pub cache: SysvarCache,
}

impl Default for Sysvars {
    fn default() -> Self {
        let clock = Clock::default();
        let epoch_rewards = EpochRewards::default();
        let epoch_schedule = EpochSchedule::without_warmup();
        let last_restart_slot = LastRestartSlot::default();

        let slot_hashes = {
            let mut default_slot_hashes = vec![(0, DEFAULT_HASH); SLOT_HASHES_MAX_ENTRIES];
            default_slot_hashes[0] = (clock.slot, DEFAULT_HASH);
            SlotHashes::new(&default_slot_hashes)
        };

        let mut stake_history = StakeHistory::default();
        stake_history.add(clock.epoch, StakeHistoryEntry::default());

        unsafe {
            let mut sysvars = Self {
                clock,
                epoch_rewards,
                epoch_schedule,
                last_restart_slot,
                rent: RENT,
                slot_hashes,
                stake_history,
                cache: MaybeUninit::zeroed().assume_init(),
            };

            sysvars.cache = (&sysvars).into();
            sysvars
        }
    }
}

impl Sysvars {
    pub fn unix_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as UnixTimestamp
            + 1
    }

    pub fn warp_to_slot(&mut self, slot: Slot) {
        let slot_delta = slot.saturating_sub(self.clock.slot);

        // First update `Clock`.
        let epoch = self.epoch_schedule.get_epoch(slot);
        let leader_schedule_epoch = self.epoch_schedule.get_leader_schedule_epoch(slot);
        let unix_timestamp = Self::unix_timestamp();

        self.clock = Clock {
            slot,
            epoch_start_timestamp: unix_timestamp - 86400,
            epoch,
            leader_schedule_epoch,
            unix_timestamp,
        };

        // Then update `SlotHashes`.
        if slot_delta > SLOT_HASHES_MAX_ENTRIES as u64 {
            let final_hash_slot = slot - SLOT_HASHES_MAX_ENTRIES as u64;

            let slot_hash_entries = (final_hash_slot..slot)
                .rev()
                .map(|slot| (slot, Hash::default()))
                .collect::<Vec<_>>();

            self.slot_hashes = SlotHashes::new(&slot_hash_entries);
        } else {
            let i = if let Some(most_recent_slot_hash) = self.slot_hashes.first() {
                most_recent_slot_hash.0
            } else {
                // By default, this zero is never used, but a user can overwrite
                // `SlotHashes`.
                0
            };
            // Don't include the target slot, since it will become the "current"
            // slot.
            for slot in i..slot {
                self.slot_hashes.add(slot, Hash::default());
            }
        }

        self.cache.set_sysvar_for_tests(&self.clock);
        self.cache.set_sysvar_for_tests(&self.slot_hashes);
    }
}

impl From<&Sysvars> for SysvarCache {
    fn from(mollusk_cache: &Sysvars) -> Self {
        let mut sysvar_cache = SysvarCache::default();
        sysvar_cache.fill_missing_entries(|pubkey, set_sysvar| {
            if pubkey.eq(&Clock::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.clock).unwrap());
            }
            if pubkey.eq(&EpochRewards::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.epoch_rewards).unwrap());
            }
            if pubkey.eq(&EpochSchedule::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.epoch_schedule).unwrap());
            }
            if pubkey.eq(&LastRestartSlot::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.last_restart_slot).unwrap());
            }
            if pubkey.eq(&Rent::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.rent).unwrap());
            }
            if pubkey.eq(&SlotHashes::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.slot_hashes).unwrap());
            }
            if pubkey.eq(&StakeHistory::id()) {
                set_sysvar(&bincode::serialize(&mollusk_cache.stake_history).unwrap());
            }
        });
        sysvar_cache
    }
}

pub const DEFAULT_HASH: Hash = Hash::new_from_array([0; 32]);
pub const RENT: Rent = Rent {
    lamports_per_byte_year: DEFAULT_LAMPORTS_PER_BYTE_YEAR,
    exemption_threshold: DEFAULT_EXEMPTION_THRESHOLD,
    burn_percent: DEFAULT_BURN_PERCENT,
};
