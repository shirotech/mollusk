//! # Mollusk
//!
//! Mollusk is a lightweight test harness for Solana programs. It provides a
//! simple interface for testing Solana program executions in a minified
//! Solana Virtual Machine (SVM) environment.
//!
//! It does not create any semblance of a validator runtime, but instead
//! provisions a program execution pipeline directly from lower-level SVM
//! components.
//!
//! In summary, the main processor - `process_instruction` - creates minified
//! instances of Agave's program cache, transaction context, and invoke
//! context. It uses these components to directly execute the provided
//! program's ELF using the BPF Loader.
//!
//! Because it does not use AccountsDB, Bank, or any other large Agave
//! components, the harness is exceptionally fast. However, it does require
//! the user to provide an explicit list of accounts to use, since it has
//! nowhere to load them from.
//!
//! The test environment can be further configured by adjusting the compute
//! budget, feature set, or sysvars. These configurations are stored directly
//! on the test harness (the `Mollusk` struct), but can be manipulated through
//! a handful of helpers.
//!
//! Four main API methods are offered:
//!
//! * `process_instruction`: Process an instruction and return the result.
//! * `process_and_validate_instruction`: Process an instruction and perform a
//!   series of checks on the result, panicking if any checks fail.
//! * `process_instruction_chain`: Process a chain of instructions and return
//!   the result.
//! * `process_and_validate_instruction_chain`: Process a chain of instructions
//!   and perform a series of checks on each result, panicking if any checks
//!   fail.
//!
//! ## Single Instructions
//!
//! Both `process_instruction` and `process_and_validate_instruction` deal with
//! single instructions. The former simply processes the instruction and
//! returns the result, while the latter processes the instruction and then
//! performs a series of checks on the result. In both cases, the result is
//! also returned.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm::Mollusk,
//!     solana_sdk::{
//!         account::Account,
//!         instruction::{AccountMeta, Instruction},
//!         pubkey::Pubkey,
//!     },
//! };
//!
//! let program_id = Pubkey::new_unique();
//! let key1 = Pubkey::new_unique();
//! let key2 = Pubkey::new_unique();
//!
//! let instruction = Instruction::new_with_bytes(
//!     program_id,
//!     &[],
//!     vec![
//!         AccountMeta::new(key1, false),
//!         AccountMeta::new_readonly(key2, false),
//!     ],
//! );
//!
//! let accounts = vec![
//!     (key1, Account::default()),
//!     (key2, Account::default()),
//! ];
//!
//! let mollusk = Mollusk::new(&program_id, "my_program");
//!
//! // Execute the instruction and get the result.
//! let result = mollusk.process_instruction(&instruction, &accounts);
//! ```
//!
//! To apply checks via `process_and_validate_instruction`, developers can use
//! the `Check` enum, which provides a set of common checks.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm::{Mollusk, result::Check},
//!     solana_sdk::{
//!         account::Account,
//!         instruction::{AccountMeta, Instruction},
//!         pubkey::Pubkey
//!         system_instruction,
//!         system_program,
//!     },
//! };
//!
//! let sender = Pubkey::new_unique();
//! let recipient = Pubkey::new_unique();
//!
//! let base_lamports = 100_000_000u64;
//! let transfer_amount = 42_000u64;
//!
//! let instruction = system_instruction::transfer(&sender, &recipient, transfer_amount);
//! let accounts = [
//!     (
//!         sender,
//!         Account::new(base_lamports, 0, &system_program::id()),
//!     ),
//!     (
//!         recipient,
//!         Account::new(base_lamports, 0, &system_program::id()),
//!     ),
//! ];
//! let checks = vec![
//!     Check::success(),
//!     Check::compute_units(system_processor::DEFAULT_COMPUTE_UNITS),
//!     Check::account(&sender)
//!         .lamports(base_lamports - transfer_amount)
//!         .build(),
//!     Check::account(&recipient)
//!         .lamports(base_lamports + transfer_amount)
//!         .build(),
//! ];
//!
//! Mollusk::default().process_and_validate_instruction(
//!     &instruction,
//!     &accounts,
//!     &checks,
//! );
//! ```
//!
//! Note: `Mollusk::default()` will create a new `Mollusk` instance without
//! adding any provided BPF programs. It will still contain a subset of the
//! default builtin programs. For more builtin programs, you can add them
//! yourself or use the `all-builtins` feature.
//!
//! ## Instruction Chains
//!
//! Both `process_instruction_chain` and
//! `process_and_validate_instruction_chain` deal with chains of instructions.
//! The former processes each instruction in the chain and returns the final
//! result, while the latter processes each instruction in the chain and then
//! performs a series of checks on each result. In both cases, the final result
//! is also returned.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm::Mollusk,
//!     solana_sdk::{account::Account, pubkey::Pubkey, system_instruction},
//! };
//!
//! let mollusk = Mollusk::default();
//!
//! let alice = Pubkey::new_unique();
//! let bob = Pubkey::new_unique();
//! let carol = Pubkey::new_unique();
//! let dave = Pubkey::new_unique();
//!
//! let starting_lamports = 500_000_000;
//!
//! let alice_to_bob = 100_000_000;
//! let bob_to_carol = 50_000_000;
//! let bob_to_dave = 50_000_000;
//!
//! mollusk.process_instruction_chain(
//!     &[
//!         system_instruction::transfer(&alice, &bob, alice_to_bob),
//!         system_instruction::transfer(&bob, &carol, bob_to_carol),
//!         system_instruction::transfer(&bob, &dave, bob_to_dave),
//!     ],
//!     &[
//!         (alice, system_account_with_lamports(starting_lamports)),
//!         (bob, system_account_with_lamports(starting_lamports)),
//!         (carol, system_account_with_lamports(starting_lamports)),
//!         (dave, system_account_with_lamports(starting_lamports)),
//!     ],
//! );
//! ```
//!
//! Just like with `process_and_validate_instruction`, developers can use the
//! `Check` enum to apply checks via `process_and_validate_instruction_chain`.
//! Notice that `process_and_validate_instruction_chain` takes a slice of
//! tuples, where each tuple contains an instruction and a slice of checks.
//! This allows the developer to apply specific checks to each instruction in
//! the chain. The result returned by the method is the final result of the
//! last instruction in the chain.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm::{Mollusk, result::Check},
//!     solana_sdk::{account::Account, pubkey::Pubkey, system_instruction},
//! };
//!
//! let mollusk = Mollusk::default();
//!
//! let alice = Pubkey::new_unique();
//! let bob = Pubkey::new_unique();
//! let carol = Pubkey::new_unique();
//! let dave = Pubkey::new_unique();
//!
//! let starting_lamports = 500_000_000;
//!
//! let alice_to_bob = 100_000_000;
//! let bob_to_carol = 50_000_000;
//! let bob_to_dave = 50_000_000;
//!
//! mollusk.process_and_validate_instruction_chain(
//!     &[
//!         (
//!             // 0: Alice to Bob
//!             &system_instruction::transfer(&alice, &bob, alice_to_bob),
//!             &[
//!                 Check::success(),
//!                 Check::account(&alice)
//!                     .lamports(starting_lamports - alice_to_bob) // Alice pays
//!                     .build(),
//!                 Check::account(&bob)
//!                     .lamports(starting_lamports + alice_to_bob) // Bob receives
//!                     .build(),
//!                 Check::account(&carol)
//!                     .lamports(starting_lamports) // Unchanged
//!                     .build(),
//!                 Check::account(&dave)
//!                     .lamports(starting_lamports) // Unchanged
//!                     .build(),
//!             ],
//!         ),
//!         (
//!             // 1: Bob to Carol
//!             &system_instruction::transfer(&bob, &carol, bob_to_carol),
//!             &[
//!                 Check::success(),
//!                 Check::account(&alice)
//!                     .lamports(starting_lamports - alice_to_bob) // Unchanged
//!                     .build(),
//!                 Check::account(&bob)
//!                     .lamports(starting_lamports + alice_to_bob - bob_to_carol) // Bob pays
//!                     .build(),
//!                 Check::account(&carol)
//!                     .lamports(starting_lamports + bob_to_carol) // Carol receives
//!                     .build(),
//!                 Check::account(&dave)
//!                     .lamports(starting_lamports) // Unchanged
//!                     .build(),
//!             ],
//!         ),
//!         (
//!             // 2: Bob to Dave
//!             &system_instruction::transfer(&bob, &dave, bob_to_dave),
//!             &[
//!                 Check::success(),
//!                 Check::account(&alice)
//!                     .lamports(starting_lamports - alice_to_bob) // Unchanged
//!                     .build(),
//!                 Check::account(&bob)
//!                     .lamports(starting_lamports + alice_to_bob - bob_to_carol - bob_to_dave) // Bob pays
//!                     .build(),
//!                 Check::account(&carol)
//!                     .lamports(starting_lamports + bob_to_carol) // Unchanged
//!                     .build(),
//!                 Check::account(&dave)
//!                     .lamports(starting_lamports + bob_to_dave) // Dave receives
//!                     .build(),
//!             ],
//!         ),
//!     ],
//!     &[
//!         (alice, system_account_with_lamports(starting_lamports)),
//!         (bob, system_account_with_lamports(starting_lamports)),
//!         (carol, system_account_with_lamports(starting_lamports)),
//!         (dave, system_account_with_lamports(starting_lamports)),
//!     ],
//! );
//! ```
//!
//! It's important to understand that instruction chains _should not_ be
//! considered equivalent to Solana transactions. Mollusk does not impose
//! constraints on instruction chains, such as loaded account keys or size.
//! Developers should recognize that instruction chains are primarily used for
//! testing program execution.
//!
//! ## Stateful Testing with MolluskContext
//!
//! For complex testing scenarios that involve multiple instructions or require
//! persistent state between calls, `MolluskContext` provides a stateful wrapper
//! around `Mollusk`. It automatically manages an account store and provides the
//! same API methods but without requiring explicit account management.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm::{Mollusk, account_store::AccountStore},
//!     solana_account::Account,
//!     solana_instruction::Instruction,
//!     solana_pubkey::Pubkey,
//!     solana_system_interface::instruction as system_instruction,
//!     std::collections::HashMap,
//! };
//!
//! // Simple in-memory account store implementation
//! #[derive(Default)]
//! struct InMemoryAccountStore {
//!     accounts: HashMap<Pubkey, Account>,
//! }
//!
//! impl AccountStore for InMemoryAccountStore {
//!     fn get_account(&self, pubkey: &Pubkey) -> Option<Account> {
//!         self.accounts.get(pubkey).cloned()
//!     }
//!
//!     fn store_account(&mut self, pubkey: Pubkey, account: Account) {
//!         self.accounts.insert(pubkey, account);
//!     }
//! }
//!
//! let mollusk = Mollusk::default();
//! let context = mollusk.with_context(InMemoryAccountStore::default());
//!
//! let alice = Pubkey::new_unique();
//! let bob = Pubkey::new_unique();
//!
//! // Execute instructions without managing accounts manually
//! let instruction1 = system_instruction::transfer(&alice, &bob, 1_000_000);
//! let result1 = context.process_instruction(&instruction1);
//!
//! let instruction2 = system_instruction::transfer(&bob, &alice, 500_000);
//! let result2 = context.process_instruction(&instruction2);
//!
//! // Account state is automatically preserved between calls
//! ```
//!
//! The `MolluskContext` API provides the same core methods as `Mollusk`:
//!
//! * `process_instruction`: Process an instruction with automatic account
//!   management
//! * `process_instruction_chain`: Process a chain of instructions
//! * `process_and_validate_instruction`: Process and validate an instruction
//! * `process_and_validate_instruction_chain`: Process and validate an
//!   instruction chain
//!
//! All methods return `ContextResult` instead of `InstructionResult`, which
//! omits the `resulting_accounts` field since accounts are managed by the
//! context's account store.
//!
//! Note that `HashMap<Pubkey, Account>` implements `AccountStore` directly,
//! so you can use it as a simple in-memory account store without needing
//! to implement your own.
//!
//! ## Fixtures
//!
//! Mollusk also supports working with multiple kinds of fixtures, which can
//! help expand testing capabilities. Note this is all gated behind either the
//! `fuzz` or `fuzz-fd` feature flags.
//!
//! A fixture is a structured representation of a test case, containing the
//! input data, the expected output data, and any additional context required
//! to run the test. One fixture maps to one instruction.
//!
//! A classic use case for such fixtures is the act of testing two versions of
//! a program against each other, to ensure the new version behaves as
//! expected. The original version's test suite can be used to generate a set
//! of fixtures, which can then be used as inputs to test the new version.
//! Although you could also simply replace the program ELF file in the test
//! suite to achieve a similar result, fixtures provide exhaustive coverage.
//!
//! ### Generating Fixtures from Mollusk Tests
//!
//! Mollusk is capable of generating fixtures from any defined test case. If
//! the `EJECT_FUZZ_FIXTURES` environment variable is set during a test run,
//! Mollusk will serialize every invocation of `process_instruction` into a
//! fixture, using the provided inputs, current Mollusk configurations, and
//! result returned. `EJECT_FUZZ_FIXTURES_JSON` can also be set to write the
//! fixtures in JSON format.
//!
//! ```ignore
//! EJECT_FUZZ_FIXTURES="./fuzz-fixtures" cargo test-sbf ...
//! ```
//!
//! Note that Mollusk currently supports two types of fixtures: Mollusk's own
//! fixture layout and the fixture layout used by the Firedancer team. Both of
//! these layouts stem from Protobuf definitions.
//!
//! These layouts live in separate crates, but a snippet of the Mollusk input
//! data for a fixture can be found below:
//!
//! ```rust,ignore
//! /// Instruction context fixture.
//! pub struct Context {
//!     /// The compute budget to use for the simulation.
//!     pub compute_budget: ComputeBudget,
//!     /// The feature set to use for the simulation.
//!     pub feature_set: FeatureSet,
//!     /// The runtime sysvars to use for the simulation.
//!     pub sysvars: Sysvars,
//!     /// The program ID of the program being invoked.
//!     pub program_id: Pubkey,
//!     /// Accounts to pass to the instruction.
//!     pub instruction_accounts: Vec<AccountMeta>,
//!     /// The instruction data.
//!     pub instruction_data: Vec<u8>,
//!     /// Input accounts with state.
//!     pub accounts: Vec<(Pubkey, Account)>,
//! }
//! ```
//!
//! ### Loading and Executing Fixtures
//!
//! Mollusk can also execute fixtures, just like it can with instructions. The
//! `process_fixture` method will process a fixture and return the result, while
//! `process_and_validate_fixture` will process a fixture and compare the result
//! against the fixture's effects.
//!
//! An additional method, `process_and_partially_validate_fixture`, allows
//! developers to compare the result against the fixture's effects using a
//! specific subset of checks, rather than the entire set of effects. This
//! may be useful if you wish to ignore certain effects, such as compute units
//! consumed.
//!
//! ```rust,ignore
//! use {
//!     mollusk_svm::{Mollusk, fuzz::check::FixtureCheck},
//!     solana_sdk::{account::Account, pubkey::Pubkey, system_instruction},
//!     std::{fs, path::Path},
//! };
//!
//! let mollusk = Mollusk::default();
//!
//! for file in fs::read_dir(Path::new("fixtures-dir"))? {
//!     let fixture = Fixture::load_from_blob_file(&entry?.file_name());
//!
//!     // Execute the fixture and apply partial checks.
//!     mollusk.process_and_partially_validate_fixture(
//!        &fixture,
//!        &[
//!            FixtureCheck::ProgramResult,
//!            FixtureCheck::ReturnData,
//!            FixtureCheck::all_resulting_accounts(),
//!         ],
//!     );
//! }
//! ```
//!
//! Fixtures can be loaded from files or decoded from raw blobs. These
//! capabilities are provided by the respective fixture crates.

pub mod account_store;
mod compile_accounts;
pub mod epoch_stake;
pub mod file;
pub mod program;
pub mod sysvar;

// Re-export result module from mollusk-svm-result crate
pub use mollusk_svm_result as result;
#[cfg(any(feature = "fuzz", feature = "fuzz-fd"))]
use mollusk_svm_result::Compare;
use solana_clock::Clock;
use solana_compute_budget::compute_budget::{
    SVMTransactionExecutionBudget, SVMTransactionExecutionCost,
};
#[cfg(feature = "precompiles")]
use solana_precompile_error::PrecompileError;
use solana_program_runtime::sysvar_cache::SysvarCache;
use solana_rent::{
    Rent, DEFAULT_BURN_PERCENT, DEFAULT_EXEMPTION_THRESHOLD, DEFAULT_LAMPORTS_PER_BYTE_YEAR,
};
use solana_svm_feature_set::SVMFeatureSet;

use crate::sysvar::Sysvars;

use {
    crate::{compile_accounts::CompiledAccounts, epoch_stake::EpochStake, program::ProgramCache},
    agave_feature_set::FeatureSet,
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    mollusk_svm_result::{CheckContext, Config, InstructionResult},
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_hash::Hash,
    solana_instruction::Instruction,
    solana_program_runtime::invoke_context::{EnvironmentConfig, InvokeContext},
    solana_pubkey::Pubkey,
    solana_svm_callback::InvokeContextCallback,
    solana_svm_timings::ExecuteTimings,
    solana_transaction_context::TransactionContext,
};

pub(crate) const DEFAULT_LOADER_KEY: Pubkey = solana_sdk_ids::bpf_loader_upgradeable::id();

/// The Mollusk API, providing a simple interface for testing Solana programs.
///
/// All fields can be manipulated through a handful of helper methods, but
/// users can also directly access and modify them if they desire more control.
pub struct Mollusk {
    pub config: Config,

    pub compute_budget: ComputeBudget,
    budget_ex_budget: SVMTransactionExecutionBudget,
    budget_ex_cost: SVMTransactionExecutionCost,

    pub epoch_stake: EpochStake,
    pub features: SVMFeatureSet,
    pub feature_set: FeatureSet,
    pub program_cache: ProgramCache,
    pub sysvars_cache: SysvarCache,

    /// This field stores the slot only to be able to convert to and from FD
    /// fixtures and a Mollusk instance, since FD fixtures have a
    /// "slot context". However, this field is functionally irrelevant for
    /// instruction execution, since all slot-based information for on-chain
    /// programs comes from the sysvars.
    #[cfg(feature = "fuzz-fd")]
    pub slot: u64,
}

impl Default for Mollusk {
    fn default() -> Self {
        #[rustfmt::skip]
        solana_logger::setup_with_default(
            "solana_rbpf::vm=debug,\
             solana_runtime::message_processor=debug,\
             solana_runtime::system_instruction_processor=trace",
        );
        let compute_budget = ComputeBudget::new_with_defaults(true);

        #[cfg(feature = "fuzz")]
        let feature_set = {
            // Omit "test features" (they have the same u64 ID).
            let mut fs = FeatureSet::all_enabled();
            fs.active_mut()
                .remove(&agave_feature_set::disable_sbpf_v0_execution::id());
            fs.active_mut()
                .remove(&agave_feature_set::reenable_sbpf_v0_execution::id());
            fs
        };
        #[cfg(not(feature = "fuzz"))]
        let feature_set = FeatureSet::all_enabled();
        let program_cache = ProgramCache::new(&feature_set, &compute_budget);
        Self {
            config: Config::default(),

            compute_budget,
            budget_ex_budget: compute_budget.to_budget(),
            budget_ex_cost: compute_budget.to_cost(),

            epoch_stake: EpochStake::default(),
            features: feature_set.runtime_features(),
            feature_set,
            program_cache,
            sysvars_cache: Sysvars::default().setup_sysvar_cache(&[]),

            #[cfg(feature = "fuzz-fd")]
            slot: 0,
        }
    }
}

impl CheckContext for Mollusk {
    fn is_rent_exempt(&self, lamports: u64, space: usize, owner: Pubkey) -> bool {
        owner.eq(&Pubkey::default()) && lamports == 0 || RENT.is_exempt(lamports, space)
    }
}

struct MolluskInvokeContextCallback<'a> {
    #[cfg_attr(not(feature = "precompiles"), allow(dead_code))]
    feature_set: &'a FeatureSet,
    epoch_stake: &'a EpochStake,
}

impl InvokeContextCallback for MolluskInvokeContextCallback<'_> {
    fn get_epoch_stake(&self) -> u64 {
        self.epoch_stake.values().sum()
    }

    fn get_epoch_stake_for_vote_account(&self, vote_address: &Pubkey) -> u64 {
        self.epoch_stake.get(vote_address).copied().unwrap_or(0)
    }

    #[cfg(feature = "precompiles")]
    fn is_precompile(&self, program_id: &Pubkey) -> bool {
        agave_precompiles::is_precompile(program_id, |feature_id| {
            self.feature_set.is_active(feature_id)
        })
    }

    #[cfg(not(feature = "precompiles"))]
    fn is_precompile(&self, _program_id: &Pubkey) -> bool {
        false
    }

    #[cfg(feature = "precompiles")]
    fn process_precompile(
        &self,
        program_id: &Pubkey,
        data: &[u8],
        instruction_datas: Vec<&[u8]>,
    ) -> Result<(), PrecompileError> {
        if let Some(precompile) = agave_precompiles::get_precompile(program_id, |feature_id| {
            self.feature_set.is_active(feature_id)
        }) {
            precompile.verify(data, &instruction_datas, self.feature_set)
        } else {
            Err(PrecompileError::InvalidPublicKey)
        }
    }

    #[cfg(not(feature = "precompiles"))]
    fn process_precompile(
        &self,
        _program_id: &Pubkey,
        _data: &[u8],
        _instruction_datas: Vec<&[u8]>,
    ) -> Result<(), solana_precompile_error::PrecompileError> {
        panic!("precompiles feature not enabled");
    }
}

impl Mollusk {
    /// Create a new Mollusk instance containing the provided program.
    ///
    /// Attempts to load the program's ELF file from the default search paths.
    /// Once loaded, adds the program to the program cache and returns the
    /// newly created Mollusk instance.
    ///
    /// # Default Search Paths
    ///
    /// The following locations are checked in order:
    ///
    /// - `tests/fixtures`
    /// - The directory specified by the `BPF_OUT_DIR` environment variable
    /// - The directory specified by the `SBF_OUT_DIR` environment variable
    /// - The current working directory
    pub fn new(program_id: &Pubkey, program_name: &str) -> Self {
        let mut mollusk = Self::default();
        mollusk.add_program(program_id, program_name, &DEFAULT_LOADER_KEY);
        mollusk
    }

    /// Add a program to the test environment.
    ///
    /// If you intend to CPI to a program, this is likely what you want to use.
    pub fn add_program(&mut self, program_id: &Pubkey, program_name: &str, loader_key: &Pubkey) {
        let elf = file::load_program_elf(program_name);
        self.add_program_with_elf_and_loader(program_id, &elf, loader_key);
    }

    /// Add a program to the test environment using a provided ELF under a
    /// specific loader.
    ///
    /// If you intend to CPI to a program, this is likely what you want to use.
    pub fn add_program_with_elf_and_loader(
        &mut self,
        program_id: &Pubkey,
        elf: &[u8],
        loader_key: &Pubkey,
    ) {
        self.program_cache.add_program(program_id, loader_key, elf);
    }

    pub fn update_clock(&mut self, clock: &Clock) {
        self.sysvars_cache.set_sysvar_for_tests(clock);
    }

    /// Process an instruction using the minified Solana Virtual Machine (SVM)
    /// environment. Simply returns the result.
    pub fn process_instruction(
        &self,
        instruction: &Instruction,
        accounts: Vec<(Pubkey, Account)>,
    ) -> InstructionResult {
        let mut compute_units_consumed = 0;
        let mut timings = ExecuteTimings::default();

        let loader_key = if crate::program::precompile_keys::is_precompile(&instruction.program_id)
        {
            crate::program::loader_keys::NATIVE_LOADER
        } else {
            self.program_cache
                .load_program(&instruction.program_id)
                .or_panic_with(MolluskError::ProgramNotCached(&instruction.program_id))
                .account_owner()
        };

        let CompiledAccounts {
            program_id_index,
            instruction_accounts,
            transaction_accounts,
        } = crate::compile_accounts::compile_accounts(instruction, &accounts, loader_key);

        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            RENT,
            self.compute_budget.max_instruction_stack_depth,
            self.compute_budget.max_instruction_trace_length,
        );

        let invoke_result = {
            let mut program_cache = self.program_cache.cache();
            let callback = MolluskInvokeContextCallback {
                epoch_stake: &self.epoch_stake,
                feature_set: &self.feature_set,
            };
            let mut invoke_context = InvokeContext::new(
                &mut transaction_context,
                &mut program_cache,
                EnvironmentConfig::new(
                    Hash::default(),
                    5000,
                    &callback,
                    &self.features,
                    &self.sysvars_cache,
                ),
                None,
                self.budget_ex_budget,
                self.budget_ex_cost,
            );

            // Configure the next instruction frame for this invocation.
            invoke_context
                .transaction_context
                .configure_next_instruction_for_tests(
                    program_id_index,
                    instruction_accounts,
                    &instruction.data,
                )
                .expect("failed to configure next instruction");

            let result = if invoke_context.is_precompile(&instruction.program_id) {
                invoke_context.process_precompile(
                    &instruction.program_id,
                    &instruction.data,
                    std::iter::once(instruction.data.as_ref()),
                )
            } else {
                invoke_context.process_instruction(&mut compute_units_consumed, &mut timings)
            };

            result
        };

        let return_data = transaction_context.get_return_data().1.to_vec();

        let program_result = invoke_result.is_ok();
        let resulting_accounts = if program_result {
            accounts
                .into_iter()
                .map(|(pubkey, account)| {
                    transaction_context
                        .find_index_of_account(&pubkey)
                        .map(|index| {
                            let resulting_account = transaction_context
                                .accounts()
                                .try_borrow(index)
                                .unwrap()
                                .clone()
                                .into();
                            (pubkey, resulting_account)
                        })
                        .unwrap_or((pubkey, account))
                })
                .collect()
        } else {
            accounts
        };

        InstructionResult {
            compute_units_consumed,
            program_result,
            raw_result: invoke_result,
            return_data,
            resulting_accounts,
        }
    }
}

const RENT: Rent = Rent {
    lamports_per_byte_year: DEFAULT_LAMPORTS_PER_BYTE_YEAR,
    exemption_threshold: DEFAULT_EXEMPTION_THRESHOLD,
    burn_percent: DEFAULT_BURN_PERCENT,
};
