mod compile_accounts;
pub mod program;
pub mod sysvar;

// Re-export result module from mollusk-svm-result crate
pub use mollusk_svm_result as result;
pub use solana_program_runtime::sysvar_cache::SysvarCache;

use crate::sysvar::{DEFAULT_HASH, RENT};

use {
    crate::{
        compile_accounts::{CompiledAccounts, compile_accounts},
        program::ProgramCache,
    },
    agave_feature_set::FeatureSet,
    agave_syscalls::{
        create_program_runtime_environment_v1, create_program_runtime_environment_v2,
    },
    mollusk_svm_error::error::{MolluskError, MolluskPanic},
    mollusk_svm_result::InstructionResult,
    solana_account::Account,
    solana_compute_budget::compute_budget::ComputeBudget,
    solana_compute_budget::compute_budget::{
        SVMTransactionExecutionBudget, SVMTransactionExecutionCost,
    },
    solana_instruction::Instruction,
    solana_program_runtime::invoke_context::{EnvironmentConfig, InvokeContext},
    solana_program_runtime::loaded_programs::ProgramRuntimeEnvironments,
    solana_pubkey::Pubkey,
    solana_svm_callback::InvokeContextCallback,
    solana_svm_feature_set::SVMFeatureSet,
    solana_svm_log_collector::LogCollector,
    solana_svm_timings::ExecuteTimings,
    solana_transaction_context::TransactionContext,
    std::{cell::RefCell, rc::Rc, sync::Arc},
};

/// The Mollusk API, providing a simple interface for testing Solana programs.
///
/// All fields can be manipulated through a handful of helper methods, but
/// users can also directly access and modify them if they desire more control.
pub struct Mollusk {
    pub compute_budget: ComputeBudget,
    budget_ex_budget: SVMTransactionExecutionBudget,
    budget_ex_cost: SVMTransactionExecutionCost,
    runtime_envs: ProgramRuntimeEnvironments,

    pub features: SVMFeatureSet,
    pub feature_set: FeatureSet,
    pub logger: Rc<RefCell<LogCollector>>,
    pub program_cache: ProgramCache,
}

impl Default for Mollusk {
    fn default() -> Self {
        #[rustfmt::skip]
        solana_logger::setup_with_default(
            "solana_rbpf::vm=debug,\
             solana_runtime::message_processor=debug,\
             solana_runtime::system_instruction_processor=trace",
        );

        let feature_set = FeatureSet::all_enabled();
        let simd_0268_active =
            feature_set.is_active(&agave_feature_set::raise_cpi_nesting_limit_to_8::id());
        let simd_0339_active =
            feature_set.is_active(&agave_feature_set::increase_cpi_account_info_limit::id());

        let compute_budget = ComputeBudget::new_with_defaults(simd_0268_active, simd_0339_active);
        let program_cache = ProgramCache::new(&feature_set, &compute_budget);
        let runtime_features = feature_set.runtime_features();
        let execution_budget = compute_budget.to_budget();

        Self {
            compute_budget,
            budget_ex_budget: execution_budget,
            budget_ex_cost: compute_budget.to_cost(),
            runtime_envs: ProgramRuntimeEnvironments {
                program_runtime_v1: Arc::new(
                    create_program_runtime_environment_v1(
                        &runtime_features,
                        &execution_budget,
                        /* reject_deployment_of_broken_elfs */ false,
                        /* debugging_features */ false,
                    )
                    .unwrap(),
                ),
                program_runtime_v2: Arc::new(create_program_runtime_environment_v2(
                    &execution_budget,
                    /* debugging_features */ false,
                )),
            },

            features: runtime_features,
            feature_set,
            logger: Rc::new(RefCell::new(LOGGER)),
            program_cache,
        }
    }
}

const LOGGER: LogCollector = LogCollector {
    messages: Vec::new(),
    bytes_written: 0,
    bytes_limit: None,
    limit_warning: false,
};

struct MolluskInvokeContextCallback;
impl InvokeContextCallback for MolluskInvokeContextCallback {}

impl Mollusk {
    fn get_loader_key(&self, program_id: &Pubkey) -> Pubkey {
        self.program_cache
            .load_program(program_id)
            .or_panic_with(MolluskError::ProgramNotCached(program_id))
            .account_owner()
    }

    #[inline]
    pub fn process_instruction_inner(
        &self,
        sysvar_cache: &SysvarCache,
        instruction: Instruction,
        CompiledAccounts {
            program_id_index,
            instruction_accounts,
            transaction_accounts,
        }: CompiledAccounts,
    ) -> InstructionResult {
        let mut compute_units_consumed = 0;
        let mut transaction_context = TransactionContext::new(
            transaction_accounts,
            RENT,
            self.compute_budget.max_instruction_stack_depth,
            self.compute_budget.max_instruction_trace_length,
        );

        let invoke_result = {
            let mut program_cache = self.program_cache.cache.borrow_mut();
            let mut invoke_context = InvokeContext::new(
                &mut transaction_context,
                &mut program_cache,
                EnvironmentConfig::new(
                    DEFAULT_HASH,
                    5000,
                    &MolluskInvokeContextCallback,
                    &self.features,
                    &self.runtime_envs,
                    &self.runtime_envs,
                    sysvar_cache,
                ),
                Some(self.logger.clone()),
                self.budget_ex_budget,
                self.budget_ex_cost,
            );

            // Configure the next instruction frame for this invocation.
            invoke_context
                .transaction_context
                .configure_next_instruction_for_tests(
                    program_id_index.into(),
                    instruction_accounts,
                    instruction.data,
                )
                .expect("failed to configure next instruction");

            invoke_context
                .process_instruction(&mut compute_units_consumed, &mut ExecuteTimings::default())
        };

        InstructionResult {
            compute_units_consumed: compute_units_consumed as u32,
            raw_result: invoke_result,
            return_data: transaction_context.get_return_data().1.to_vec(),
            messages: self.logger.replace(LOGGER).messages,
        }
    }

    /// Process an instruction using the minified Solana Virtual Machine (SVM)
    /// environment. Simply returns the result.
    pub fn process_instruction(
        &self,
        sysvar_cache: &SysvarCache,
        instruction: Instruction,
        accounts: Vec<(Pubkey, Account)>,
    ) -> InstructionResult {
        let loader_key = self.get_loader_key(&instruction.program_id);
        let compiled_accounts = compile_accounts(&instruction, accounts, loader_key);
        self.process_instruction_inner(sysvar_cache, instruction, compiled_accounts)
    }
}
