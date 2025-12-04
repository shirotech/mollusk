//! Module for working with Solana programs.

use {
    agave_feature_set::FeatureSet,
    agave_syscalls::create_program_runtime_environment_v1,
    ahash::HashMap,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_program_runtime::{
        invoke_context::{BuiltinFunctionWithContext, InvokeContext},
        loaded_programs::{LoadProgramMetrics, ProgramCacheEntry, ProgramCacheForTxBatch},
        solana_sbpf::program::BuiltinProgram,
    },
    solana_pubkey::Pubkey,
    std::{cell::RefCell, sync::Arc},
};

/// Loader keys, re-exported from `solana_sdk` for convenience.
pub mod loader_keys {
    pub use solana_sdk_ids::{
        bpf_loader::ID as LOADER_V2, bpf_loader_deprecated::ID as LOADER_V1,
        bpf_loader_upgradeable::ID as LOADER_V3, loader_v4::ID as LOADER_V4,
        native_loader::ID as NATIVE_LOADER,
    };
}

pub struct ProgramCache {
    pub cache: RefCell<ProgramCacheForTxBatch>,
    // This stinks, but the `ProgramCacheForTxBatch` doesn't offer a way to
    // access its entries directly. In order to make DX easier for those using
    // `MolluskContext`, we need to track entries added to the cache,
    // so we can populate the account store with program accounts.
    // This saves the developer from having to pre-load the account store with
    // all program accounts they may use, when `Mollusk` has that information
    // already.
    //
    // K: program ID, V: loader key
    pub entries_cache: RefCell<HashMap<Pubkey, Pubkey>>,
    // The function registry (syscalls) to use for verifying and loading
    // program ELFs.
    pub program_runtime_environment: BuiltinProgram<InvokeContext<'static, 'static>>,
}

impl ProgramCache {
    pub fn new(feature_set: &FeatureSet, compute_budget: &ComputeBudget) -> Self {
        let me = Self {
            cache: Default::default(),
            entries_cache: Default::default(),
            program_runtime_environment: create_program_runtime_environment_v1(
                &feature_set.runtime_features(),
                &compute_budget.to_budget(),
                /* reject_deployment_of_broken_elfs */ false,
                /* debugging_features */ false,
            )
            .unwrap(),
        };
        BUILTINS.iter().for_each(|builtin| {
            let program_id = builtin.program_id;
            let entry = builtin.program_cache_entry();
            me.replenish(program_id, entry);
        });
        me
    }

    pub fn has_cache(&self, program_id: &Pubkey) -> bool {
        self.entries_cache.borrow().contains_key(program_id)
    }

    fn replenish(&self, program_id: Pubkey, entry: Arc<ProgramCacheEntry>) {
        self.entries_cache
            .borrow_mut()
            .insert(program_id, entry.account_owner());
        self.cache.borrow_mut().replenish(program_id, entry);
    }

    /// Add a builtin program to the cache.
    pub fn add_builtin(&mut self, builtin: Builtin) {
        let program_id = builtin.program_id;
        let entry = builtin.program_cache_entry();
        self.replenish(program_id, entry);
    }

    /// Add a program to the cache.
    pub fn add_program(&mut self, program_id: &Pubkey, loader_key: &Pubkey, elf: &[u8]) {
        // This might look rough, but it's actually functionally the same as
        // calling `create_program_runtime_environment_v1` on every addition.
        let environment = {
            let config = self.program_runtime_environment.get_config().clone();
            let mut loader = BuiltinProgram::new_loader(config);

            for (_key, (name, value)) in self
                .program_runtime_environment
                .get_function_registry()
                .iter()
            {
                let name = std::str::from_utf8(name).unwrap();
                loader.register_function(name, value).unwrap();
            }

            Arc::new(loader)
        };
        self.replenish(
            *program_id,
            Arc::new(
                ProgramCacheEntry::new(
                    loader_key,
                    environment,
                    0,
                    0,
                    elf,
                    elf.len(),
                    &mut LoadProgramMetrics::default(),
                )
                .unwrap(),
            ),
        );
    }

    /// Load a program from the cache.
    pub fn load_program(&self, program_id: &Pubkey) -> Option<Arc<ProgramCacheEntry>> {
        self.cache.borrow().find(program_id)
    }
}

pub struct Builtin {
    program_id: Pubkey,
    name: &'static str,
    entrypoint: BuiltinFunctionWithContext,
}

impl Builtin {
    fn program_cache_entry(&self) -> Arc<ProgramCacheEntry> {
        Arc::new(ProgramCacheEntry::new_builtin(
            0,
            self.name.len(),
            self.entrypoint,
        ))
    }
}

static BUILTINS: &[Builtin] = &[
    Builtin {
        program_id: solana_system_program::id(),
        name: "system_program",
        entrypoint: solana_system_program::system_processor::Entrypoint::vm,
    },
    Builtin {
        program_id: loader_keys::LOADER_V2,
        name: "solana_bpf_loader_program",
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    Builtin {
        program_id: loader_keys::LOADER_V3,
        name: "solana_bpf_loader_upgradeable_program",
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    #[cfg(feature = "all-builtins")]
    Builtin {
        program_id: loader_keys::LOADER_V1,
        name: "solana_bpf_loader_deprecated_program",
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    #[cfg(feature = "all-builtins")]
    Builtin {
        program_id: loader_keys::LOADER_V4,
        name: "solana_loader_v4_program",
        entrypoint: solana_loader_v4_program::Entrypoint::vm,
    },
    #[cfg(feature = "all-builtins")]
    Builtin {
        program_id: solana_sdk_ids::zk_elgamal_proof_program::id(),
        name: "zk_elgamal_proof_program",
        entrypoint: solana_zk_elgamal_proof_program::Entrypoint::vm,
    },
];
