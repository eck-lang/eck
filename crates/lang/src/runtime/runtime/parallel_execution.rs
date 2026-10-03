//! Bounded parallel range execution with isolated locals and ordered writes.

use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicI64, Ordering},
};

use rayon::{ThreadPool, ThreadPoolBuilder};

use super::Runtime;
use crate::RuntimeError;
use crate::analysis::ExecutionAnalysis;
use crate::containers::array::set_element;
use crate::ir::{LocalVariableSlot, TypedBlock};
use crate::semantic::Value;

/// Runtime profitability and resource limits, independent of static safety.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionOptions {
    /// Maximum number of reusable CPU worker threads. One forces sequential execution.
    pub workers: usize,
    /// Minimum estimated total work; zero forces safe ranges through the worker path.
    pub parallelization_threshold: u64,
}

impl Default for ExecutionOptions {
    /// Uses the available CPU budget and avoids scheduling small interpreted loops.
    fn default() -> Self {
        Self {
            workers: std::thread::available_parallelism().map_or(1, usize::from),
            parallelization_threshold: crate::analysis::DEFAULT_PARALLELIZATION_THRESHOLD,
        }
    }
}

/// Retains one parallel budget and its successful pool or startup failure.
struct CachedWorkerPool {
    workers: usize,
    pool: Result<Arc<ThreadPool>, String>,
}

/// Reusable execution budget and its most recently selected parallel pool.
///
/// A single-worker budget never starts a pool. Source `parallelization.cores` overrides the default
/// budget but never the compiler's safety decision. Startup failure runs sequentially.
pub struct Executor {
    pub(super) options: ExecutionOptions,
    pool: Mutex<Option<CachedWorkerPool>>,
}

impl Executor {
    /// Creates reusable workers; explicit worker counts also work on single-CPU CI.
    pub fn new(options: ExecutionOptions) -> Result<Self, RuntimeError> {
        if options.workers == 0 {
            return Err(RuntimeError::Message(
                "execution requires at least one worker".into(),
            ));
        }
        Ok(Self {
            options,
            pool: Mutex::new(None),
        })
    }

    /// Reuses the last parallel budget; serial loops leave the cached pool untouched.
    ///
    /// One cache entry bounds retained threads when source directives change budgets.
    /// Invocations hold an Arc so concurrent budget changes cannot drop active workers.
    fn worker_pool(&self, workers: usize) -> Option<Arc<ThreadPool>> {
        let mut cached = self.pool.lock().ok()?;
        if cached
            .as_ref()
            .is_none_or(|cached| cached.workers != workers)
        {
            let pool = ThreadPoolBuilder::new()
                .num_threads(workers)
                .thread_name(|index| format!("eck-worker-{index}"))
                .build()
                .map(Arc::new)
                .map_err(|error| error.to_string());
            *cached = Some(CachedWorkerPool { workers, pool });
        }
        cached.as_ref()?.pool.as_ref().ok().cloned()
    }

    /// Executes a resolved program using this executor's reusable resource budget.
    pub fn execute(
        &self,
        program: &crate::ir::TypedProgram,
        registry: &crate::semantic::Registry,
    ) -> Result<(), RuntimeError> {
        super::execute_with_executor(program, registry, self)
    }
}

/// Shares production workers across calls while allowing tests their own executors.
pub(super) fn default_executor() -> &'static Executor {
    static EXECUTOR: OnceLock<Executor> = OnceLock::new();
    EXECUTOR.get_or_init(|| {
        Executor::new(ExecutionOptions::default()).unwrap_or_else(|_| Executor {
            options: ExecutionOptions {
                workers: 1,
                parallelization_threshold: u64::MAX,
            },
            pool: Mutex::new(None),
        })
    })
}

/// Immutable invocation state shared by the caller; worker runtimes omit it.
pub(super) struct ExecutionSession<'program> {
    pub analysis: &'program ExecutionAnalysis,
    pub identity: Option<(u64, u64)>,
    pub executor: &'program Executor,
    #[cfg(test)]
    pub observer: Option<&'program (dyn Fn(i64, std::thread::ThreadId) + Sync)>,
}

/// One prepared element store whose computation ran without mutating caller state.
pub(crate) struct PendingArrayWrite {
    pub slot: LocalVariableSlot,
    pub index: usize,
    pub element: Value,
}

/// Buffers only captured array mutations; local containers remain worker-owned.
pub(crate) struct ArrayWriteJournal {
    pub external_slots: Vec<LocalVariableSlot>,
    pub writes: Vec<PendingArrayWrite>,
    pub iteration_start: usize,
}

/// Stores one chunk's prefix of effects and its first logical failure.
struct ChunkOutcome {
    writes: Vec<PendingArrayWrite>,
    error: Option<RuntimeError>,
}

impl Runtime<'_> {
    /// Applies profitability and native-range guards to a compile-time safety proof.
    pub(super) fn try_parallel_range(
        &mut self,
        slot: LocalVariableSlot,
        start: &Value,
        end: &Value,
        range_plan: &crate::ir::TypedRangePlan,
        body: &TypedBlock,
        span: crate::syntax::Span,
    ) -> Result<bool, RuntimeError> {
        let Some(session) = self.parallel_execution else {
            return Ok(false);
        };
        if self.configuration.parallelization_level() == Some(0)
            || session.identity != Some(self.registry.execution_identity())
        {
            return Ok(false);
        }
        let Some(loop_analysis) = session.analysis.loop_analysis(span) else {
            return Ok(false);
        };
        let crate::analysis::Parallelism::IndependentIterations {
            written_array_slots,
            ..
        } = &loop_analysis.parallelism
        else {
            return Ok(false);
        };
        let workers = self
            .configuration
            .execution_workers()
            .unwrap_or(session.executor.options.workers);
        let integer = self.registry.default_integer()?;
        let integer_type = crate::semantic::ValueType::plain(integer);
        if start.scalar_type() != Some(integer_type)
            || end.scalar_type() != Some(integer_type)
            || range_plan.current_type != integer_type
            || !self
                .registry
                .result_transform_is_identity(integer, &self.configuration)?
            || workers < 2
        {
            return Ok(false);
        }
        let (Some(start), Some(end)) = (
            start.downcast_ref::<i64>().copied(),
            end.downcast_ref::<i64>().copied(),
        ) else {
            return Ok(false);
        };
        let count = i128::from(end) - i128::from(start);
        if count <= 0 {
            return Ok(false);
        }
        let threshold = self
            .configuration
            .parallelization_threshold()
            .unwrap_or(session.executor.options.parallelization_threshold);
        let total_work = loop_analysis
            .cost_per_iteration
            .saturating_mul(count as u64);
        if total_work.units() < threshold {
            return Ok(false);
        }
        let Some(pool) = session.executor.worker_pool(workers) else {
            return Ok(false);
        };
        self.execute_parallel_range(slot, start, end, body, written_array_slots, &pool)?;
        Ok(true)
    }

    /// Computes bounded batches concurrently and commits their serial prefix.
    ///
    /// Workers share immutable registry/IR and COW snapshots, never mutable
    /// locals. Only the caller applies writes after every worker has joined.
    /// A failing iteration's already-completed stores are retained; later
    /// speculative stores are discarded. Cancellation cannot skip an earlier
    /// iteration that might fail, so error selection is deterministic.
    pub(super) fn execute_parallel_range(
        &mut self,
        slot: LocalVariableSlot,
        start: i64,
        end: i64,
        body: &TypedBlock,
        external_slots: &[LocalVariableSlot],
        pool: &ThreadPool,
    ) -> Result<(), RuntimeError> {
        #[cfg(test)]
        let session = self
            .parallel_execution
            .expect("parallel caller has invocation state");
        let integer = self.registry.default_integer()?;
        let loop_plan = self.compile_loop_body_execution_plan(slot, body)?;
        let mut batch_start = start;
        let worker_count = pool.current_num_threads();
        // Bound speculative work and journal memory independently of range size.
        const ITERATIONS_PER_WORKER_BATCH: u64 = 1024;
        while batch_start < end {
            let remaining = (i128::from(end) - i128::from(batch_start)) as u64;
            let batch_length =
                remaining.min(ITERATIONS_PER_WORKER_BATCH.saturating_mul(worker_count as u64));
            let first_failure = AtomicI64::new(end);
            let captures = &self.local_values;
            let configuration = &self.configuration;
            let registry = self.registry;
            let outcomes: Vec<ChunkOutcome> = pool.broadcast(|worker_context| {
                let worker_index = worker_context.index();
                let offset = batch_length * worker_index as u64 / worker_count as u64;
                let next_offset = batch_length * (worker_index + 1) as u64 / worker_count as u64;
                let chunk_start = (i128::from(batch_start) + i128::from(offset)) as i64;
                let chunk_end = (i128::from(batch_start) + i128::from(next_offset)) as i64;
                let mut worker = Runtime {
                    registry,
                    configuration: configuration.clone(),
                    local_values: captures.clone(),
                    loop_control: None,
                    parallel_execution: None,
                    pending_writes: Some(ArrayWriteJournal {
                        external_slots: external_slots.to_vec(),
                        writes: Vec::new(),
                        iteration_start: 0,
                    }),
                };
                let mut value_stack = Vec::with_capacity(loop_plan.value_stack_capacity);
                let mut error = None;
                for current in chunk_start..chunk_end {
                    if current >= first_failure.load(Ordering::Relaxed) {
                        break;
                    }
                    let journal = worker
                        .pending_writes
                        .as_mut()
                        .expect("worker owns a journal");
                    journal.iteration_start = journal.writes.len();
                    #[cfg(test)]
                    if let Some(observer) = session.observer {
                        observer(current, std::thread::current().id());
                    }
                    worker.store_local_value(slot, Value::new(integer, current));
                    if let Err(failure) = worker.execute_loop_body(&loop_plan, &mut value_stack) {
                        first_failure.fetch_min(current, Ordering::Relaxed);
                        error = Some(failure);
                        break;
                    }
                    worker.loop_control.take();
                }
                ChunkOutcome {
                    writes: worker
                        .pending_writes
                        .take()
                        .expect("worker journal exists")
                        .writes,
                    error,
                }
            });
            for outcome in outcomes {
                for write in outcome.writes {
                    let array = self.local_values[write.slot.0].as_mut().ok_or_else(|| {
                        RuntimeError::Message("array binding is not initialized".into())
                    })?;
                    set_element(array, write.index, write.element)?;
                }
                if let Some(error) = outcome.error {
                    return Err(error);
                }
            }
            batch_start = (i128::from(batch_start) + i128::from(batch_length)) as i64;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "parallel_execution.tests.rs"]
mod tests;
