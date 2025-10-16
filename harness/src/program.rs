//! Module for working with Solana programs.

use {
    agave_feature_set::FeatureSet,
    agave_syscalls::create_program_runtime_environment_v1,
    ahash::{HashMap, HashMapExt},
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_loader_v3_interface::state::UpgradeableLoaderState,
    solana_loader_v4_interface::state::{LoaderV4State, LoaderV4Status},
    solana_program_runtime::{
        invoke_context::{BuiltinFunctionWithContext, InvokeContext},
        loaded_programs::{LoadProgramMetrics, ProgramCacheEntry, ProgramCacheForTxBatch},
        solana_sbpf::program::BuiltinProgram,
    },
    solana_pubkey::Pubkey,
    solana_rent::Rent,
    std::{
        cell::{RefCell, RefMut},
        sync::Arc,
    },
};

/// Loader keys, re-exported from `solana_sdk` for convenience.
pub mod loader_keys {
    pub use solana_sdk_ids::{
        bpf_loader::ID as LOADER_V2, bpf_loader_deprecated::ID as LOADER_V1,
        bpf_loader_upgradeable::ID as LOADER_V3, loader_v4::ID as LOADER_V4,
        native_loader::ID as NATIVE_LOADER,
    };
}

#[cfg(feature = "precompiles")]
pub mod precompile_keys {
    use solana_pubkey::Pubkey;
    pub use solana_sdk_ids::{
        ed25519_program::ID as ED25519_PROGRAM, secp256k1_program::ID as SECP256K1_PROGRAM,
        secp256r1_program::ID as SECP256R1_PROGRAM,
    };

    pub(crate) fn is_precompile(program_id: &Pubkey) -> bool {
        matches!(
            *program_id,
            ED25519_PROGRAM | SECP256K1_PROGRAM | SECP256R1_PROGRAM
        )
    }
}

#[cfg(not(feature = "precompiles"))]
pub mod precompile_keys {
    use solana_pubkey::Pubkey;

    pub(crate) fn is_precompile(_program_id: &Pubkey) -> bool {
        false
    }
}

pub struct ProgramCache {
    cache: RefCell<ProgramCacheForTxBatch>,
    // This stinks, but the `ProgramCacheForTxBatch` doesn't offer a way to
    // access its entries directly. In order to make DX easier for those using
    // `MolluskContext`, we need to track entries added to the cache,
    // so we can populate the account store with program accounts.
    // This saves the developer from having to pre-load the account store with
    // all program accounts they may use, when `Mollusk` has that information
    // already.
    //
    // K: program ID, V: loader key
    entries_cache: RefCell<HashMap<Pubkey, Pubkey>>,
    // The function registry (syscalls) to use for verifying and loading
    // program ELFs.
    pub program_runtime_environment: BuiltinProgram<InvokeContext<'static>>,
}

impl ProgramCache {
    pub fn new(feature_set: &FeatureSet, compute_budget: &ComputeBudget) -> Self {
        let me = Self {
            cache: RefCell::new(ProgramCacheForTxBatch::default()),
            entries_cache: RefCell::new(HashMap::new()),
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

    pub(crate) fn cache(&self) -> RefMut<'_, ProgramCacheForTxBatch> {
        self.cache.borrow_mut()
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
        program_id: loader_keys::LOADER_V4,
        name: "solana_loader_v4_program",
        entrypoint: solana_loader_v4_program::Entrypoint::vm,
    },
    #[cfg(feature = "all-builtins")]
    Builtin {
        program_id: solana_sdk_ids::stake::id(),
        name: "solana_stake_program",
        entrypoint: solana_stake_program::stake_instruction::Entrypoint::vm,
    },
    /* ... */
];

/// Create a key and account for a builtin program.
pub fn create_keyed_account_for_builtin_program(
    program_id: &Pubkey,
    name: &str,
) -> (Pubkey, Account) {
    let data = name.as_bytes().to_vec();
    let lamports = Rent::default().minimum_balance(data.len());
    let account = Account {
        lamports,
        data,
        owner: loader_keys::NATIVE_LOADER,
        executable: true,
        ..Default::default()
    };
    (*program_id, account)
}

/// Get the key and account for the system program.
pub fn keyed_account_for_system_program() -> (Pubkey, Account) {
    create_keyed_account_for_builtin_program(&BUILTINS[0].program_id, BUILTINS[0].name)
}

/// Get the key and account for the BPF Loader v2 program.
pub fn keyed_account_for_bpf_loader_v2_program() -> (Pubkey, Account) {
    create_keyed_account_for_builtin_program(&BUILTINS[1].program_id, BUILTINS[1].name)
}

/// Get the key and account for the BPF Loader v3 (Upgradeable) program.
pub fn keyed_account_for_bpf_loader_v3_program() -> (Pubkey, Account) {
    create_keyed_account_for_builtin_program(&BUILTINS[2].program_id, BUILTINS[2].name)
}

/* ... */

/// Create a BPF Loader 1 (deprecated) program account.
pub fn create_program_account_loader_v1(elf: &[u8]) -> Account {
    let lamports = Rent::default().minimum_balance(elf.len());
    Account {
        lamports,
        data: elf.to_vec(),
        owner: loader_keys::LOADER_V1,
        executable: true,
        ..Default::default()
    }
}

/// Create a BPF Loader 2 program account.
pub fn create_program_account_loader_v2(elf: &[u8]) -> Account {
    let lamports = Rent::default().minimum_balance(elf.len());
    Account {
        lamports,
        data: elf.to_vec(),
        owner: loader_keys::LOADER_V2,
        executable: true,
        ..Default::default()
    }
}

/// Create a BPF Loader v3 (Upgradeable) program account.
pub fn create_program_account_loader_v3(program_id: &Pubkey) -> Account {
    let programdata_address =
        Pubkey::find_program_address(&[program_id.as_ref()], &loader_keys::LOADER_V3).0;
    let data = bincode::serialize(&UpgradeableLoaderState::Program {
        programdata_address,
    })
    .unwrap();
    let lamports = Rent::default().minimum_balance(data.len());
    Account {
        lamports,
        data,
        owner: loader_keys::LOADER_V3,
        executable: true,
        ..Default::default()
    }
}

/// Create a BPF Loader v3 (Upgradeable) program data account.
pub fn create_program_data_account_loader_v3(elf: &[u8]) -> Account {
    let data = {
        let elf_offset = UpgradeableLoaderState::size_of_programdata_metadata();
        let data_len = elf_offset + elf.len();
        let mut data = vec![0; data_len];
        bincode::serialize_into(
            &mut data[0..elf_offset],
            &UpgradeableLoaderState::ProgramData {
                slot: 0,
                upgrade_authority_address: None,
            },
        )
        .unwrap();
        data[elf_offset..].copy_from_slice(elf);
        data
    };
    let lamports = Rent::default().minimum_balance(data.len());
    Account {
        lamports,
        data,
        owner: loader_keys::LOADER_V3,
        executable: false,
        ..Default::default()
    }
}

/// Create a BPF Loader v3 (Upgradeable) program and program data account.
///
/// Returns a tuple, where the first element is the program account and the
/// second element is the program data account.
pub fn create_program_account_pair_loader_v3(
    program_id: &Pubkey,
    elf: &[u8],
) -> (Account, Account) {
    (
        create_program_account_loader_v3(program_id),
        create_program_data_account_loader_v3(elf),
    )
}

/// Create a BPF Loader 4 program account.
pub fn create_program_account_loader_v4(elf: &[u8]) -> Account {
    let data = unsafe {
        let elf_offset = LoaderV4State::program_data_offset();
        let data_len = elf_offset + elf.len();
        let mut data = vec![0u8; data_len];
        *std::mem::transmute::<&mut [u8; LoaderV4State::program_data_offset()], &mut LoaderV4State>(
            (&mut data[0..elf_offset]).try_into().unwrap(),
        ) = LoaderV4State {
            slot: 0,
            authority_address_or_next_version: Pubkey::new_from_array([2; 32]),
            status: LoaderV4Status::Deployed,
        };
        data[elf_offset..].copy_from_slice(elf);
        data
    };
    let lamports = Rent::default().minimum_balance(data.len());
    Account {
        lamports,
        data,
        owner: loader_keys::LOADER_V3,
        executable: false,
        ..Default::default()
    }
}
