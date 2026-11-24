// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560, FR-561, FR-562: the translator from the checked representation of
//! one function to a Cranelift function.
//!
//! Every `Integer` operation is a call to the kernel shim of [`crate::exec`],
//! which runs the operation the interpreter runs with the interpreter's
//! charges; the translator only decides, from checked types at `prepare`
//! (FR-562), that the result fits a signed 64-bit machine integer, so the
//! value can be carried in a register. The decision never depends on a
//! run-time value.

use std::collections::HashMap;
use std::sync::Arc;

use cranelift_codegen::ir::{
    types, AbiParam, Block, InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Value as Clif,
};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::JITModule;
use cranelift_module::{FuncId, Module};
use filament_checked::integer_division::AdmittedIntegerDivision;
use filament_checked::node::{
    Arithmetic, Connective, IntegerDivisionOperator, NodeKind, OrderedKind, RecordField, RecordSlot,
    Visit,
};
use filament_checked::key::DeclarationKey;
use filament_exact::{
    ArithmeticOperator, CardinalityBound, CollectionKind, CollectionType, DecimalType,
    DivisionMember, Integer, IntegerDomain, IntegerInterval, Meter, NodeKey, OrderingOperator,
    Outcome, RationalDomain, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::declaration::CheckedEquality;
use filament_semantic_value::declaration::EqualityOperand;
use filament_semantic_value::location::ExpressionLocation;
use fsl_eval::execution::{PrepareFailure, PrepareMeter};
use fsl_semantics::check::{CheckedGraph, CheckedNode};

use crate::exec::{Shim, Signature, ARGUMENTS_OFFSET, FAILED_OFFSET, RESULT_OFFSET};
use fsl_eval::execution::coverage::{first_outside_body, OutsideCoverage as Outside};
use fsl_eval::value::collections::Shape as IterationShape;

/// Deepest expression this backend translates. Translation recurses on the
/// host stack; a deeper tree is listed unsupported and the interpreter, which
/// evaluates without host recursion, runs it.
const MAX_DEPTH: usize = 192;

/// What one shim call computes.
pub(crate) enum OpKind {
    /// An exact literal stored in the per-call value table.
    Literal(Value),
    PopulationObjects {
        population: DeclarationKey,
        value_type: ValueType,
    },
    NamedAllInstances {
        population: DeclarationKey,
        members: Vec<filament_exact::EffectiveId>,
        value_type: ValueType,
    },
    Arithmetic(Arithmetic),
    Rational {
        operator: ArithmeticOperator,
        domain: RationalDomain,
    },
    RationalNegate(RationalDomain),
    Decimal {
        operator: ArithmeticOperator,
        target: DecimalType,
    },
    DecimalNegate(DecimalType),
    Size,
    Field {
        field: RecordField,
        optional: bool,
        value_type: ValueType,
    },
    Contains {
        item_boolean: bool,
        item_integer: bool,
    },
    BuildStart,
    BuildValue {
        boolean: bool,
        integer: bool,
    },
    ChargeElement,
    BuildRecord {
        declaration: NodeKey,
        slots: Vec<RecordSlot<fsl_semantics::check::NodeId>>,
    },
    BuildTuple {
        declaration: NodeKey,
    },
    BuildCollection {
        collection_type: CollectionType,
    },
    TupleProject {
        declaration: NodeKey,
        position: usize,
    },
    Index,
    DropFirst,
    RemoveOne {
        item_boolean: bool,
        item_integer: bool,
    },
    Flatten {
        result_type: ValueType,
    },
    ConvertCollection {
        target: CollectionType,
    },
    ConvertDecimal {
        target: DecimalType,
    },
    ConvertText {
        target: ValueType,
    },
    /// Convert an equality operand using the checked equality's conversion.
    ConvertScalar {
        operand: EqualityOperand,
        source_boolean: bool,
        source_integer: bool,
        integer_result: bool,
    },
    IterationStart {
        shape: IterationShape,
        value_type: ValueType,
        only_source_filter: bool,
        reduce: bool,
        has_identity: bool,
        identity_boolean: bool,
        identity_integer: bool,
    },
    IterationNext {
        shape: IterationShape,
        value_type: ValueType,
        only_source_filter: bool,
        element_boolean: bool,
        element_integer: bool,
    },
    IterationAccumulator {
        boolean: bool,
        integer: bool,
    },
    IterationAbsorb {
        shape: IterationShape,
        value_type: ValueType,
        only_source_filter: bool,
        result_boolean: bool,
        result_integer: bool,
    },
    IterationFinish {
        shape: IterationShape,
        value_type: ValueType,
        only_source_filter: bool,
        result_boolean: bool,
        result_integer: bool,
    },
    Negate,
    Divide {
        operator: IntegerDivisionOperator,
        selection: Option<AdmittedIntegerDivision>,
    },
    ExactDivide(RationalDomain),
    Order {
        operator: OrderingOperator,
        kind: OrderedKind,
    },
    Coerce(IntegerInterval),
    /// `=` or `!=` (the checked equality), over operands carried as
    /// Booleans (`true`) or Integers (`false`).
    Equality {
        checked: Box<CheckedEquality>,
        left_boolean: bool,
        left_integer: bool,
        right_boolean: bool,
        right_integer: bool,
    },
    Not,
    /// The `boolean.result-retain` charge of a connective's result.
    Retain,
    /// The `function.call` charge of a call.
    ChargeCall,
    FrameStart,
    FrameArgument { boolean: bool, integer: bool },
    FrameEnter,
    FrameLeave,
}

/// One kernel operation of a translated function and where it is.
pub(crate) struct Op {
    pub(crate) kind: OpKind,
    pub(crate) location: ExpressionLocation,
}

/// The register representation of a checked value: a Boolean (`0` or `1`) or
/// an `Integer` with the interval FR-562 derives for it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Ty {
    Boolean,
    Int(i128, i128),
    /// A value carried by an exact-value-table handle.
    Opaque,
}

fn fits_i64((low, high): (i128, i128)) -> bool {
    low >= i128::from(i64::MIN) && high <= i128::from(i64::MAX)
}

fn same_register_type(left: Ty, right: Ty) -> bool {
    matches!(
        (left, right),
        (Ty::Boolean, Ty::Boolean) | (Ty::Int(..), Ty::Int(..)) | (Ty::Opaque, Ty::Opaque)
    )
}

/// What an entry was translated to.
pub(crate) struct Translated {
    /// The `extern "C" fn(*const EntryFrame) -> u32` trampoline of the entry.
    pub(crate) id: FuncId,
    pub(crate) result_is_boolean: bool,
    pub(crate) result_is_integer: bool,
    /// The signature the translator declared for the trampoline.
    pub(crate) signature: Signature,
}

/// Why a function was not translated.
pub(crate) enum Failure {
    /// The function holds a construct this backend does not translate.
    Unsupported(Outside),
    /// `prepare` stops (a limit, the handle, or a backend invariant).
    Prepare(PrepareFailure),
}

impl From<PrepareFailure> for Failure {
    fn from(failure: PrepareFailure) -> Self {
        Self::Prepare(failure)
    }
}

fn fault(invariant: &'static str) -> Failure {
    Failure::Prepare(PrepareFailure::Fault(
        filament_semantic_value::catalog::InternalFault::new("FR-560", invariant),
    ))
}

fn untranslated(node: CheckedNode<'_>) -> Outside {
    let form = match node.kind() {
        NodeKind::Local(_) => "name",
        NodeKind::Literal(_) => "literal",
        NodeKind::Let { .. } => "let",
        NodeKind::If { .. } => "if",
        NodeKind::Call { .. } | NodeKind::ImportedCall { .. } | NodeKind::Dispatch { .. } => "call",
        _ => "operator",
    };
    Outside {
        construct: filament_eval::UntranslatableConstruct::ExpressionForm(form.to_owned()),
        location: node.location().clone(),
    }
}

/// The interval of a declared parameter or result type, when it is a type
/// this backend carries in a register.
pub(crate) fn type_of(value_type: &ValueType) -> Option<Ty> {
    match value_type {
        ValueType::Boolean => Some(Ty::Boolean),
        ValueType::Integer => Some(Ty::Int(i128::from(i64::MIN), i128::from(i64::MAX))),
        ValueType::Int(interval) => {
            let bound = |value: &Integer| {
                value.to_string().parse::<i128>().unwrap_or_else(|_| {
                    if value.is_negative() {
                        i128::MIN
                    } else {
                        i128::MAX
                    }
                })
            };
            let (lo, hi) = (bound(interval.lower()), bound(interval.upper()));
            Some(Ty::Int(lo, hi))
        }
        ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Text(_)
        | ValueType::Reference(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Option(_) => Some(Ty::Opaque),
        _ => None,
    }
}

/// Whether a parameter of `value_type` is carried in a register, and as a
/// Boolean (`Some(true)`) or an Integer (`Some(false)`); `None` for a type
/// this backend does not carry (an unread parameter of one is not an error).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RegisterKind {
    Boolean,
    Integer,
    ExactValue,
}

pub(crate) fn register_kind(value_type: &ValueType) -> Option<RegisterKind> {
    match type_of(value_type)? {
        Ty::Boolean => Some(RegisterKind::Boolean),
        Ty::Int(low, high) if fits_i64((low, high)) => Some(RegisterKind::Integer),
        Ty::Int(..) => None,
        Ty::Opaque => Some(RegisterKind::ExactValue),
    }
}

/// `value` as a signed 64-bit integer, when it fits.
pub(crate) fn integer_to_i64(value: &Integer) -> Option<i64> {
    let magnitude = if value.is_negative() {
        value.neg().to_u64()?
    } else {
        value.to_u64()?
    };
    let wide = i128::from(magnitude);
    i64::try_from(if value.is_negative() { -wide } else { wide }).ok()
}

enum State {
    InProgress,
    Done(FuncId, Ty),
    Unsupported,
}

/// The translator of one prepared value: the functions it has translated, the
/// operation table their shim calls index, and the prepare meter.
pub(crate) struct Compiler<'a, 'm> {
    module: &'a mut JITModule,
    shim: FuncId,
    graph: &'a CheckedGraph,
    coverage: &'a fsl_eval::execution::BackendCoverage,
    ops: &'a mut Vec<Op>,
    meter: &'a mut PrepareMeter<'m>,
    state: HashMap<usize, State>,
}

struct Emitter<'a, 'b, 'm> {
    builder: FunctionBuilder<'b>,
    module: &'a mut JITModule,
    shim: FuncId,
    context: Clif,
    exit: Block,
    ops: &'a mut Vec<Op>,
    state: &'a HashMap<usize, State>,
    slots: HashMap<usize, (Clif, Ty)>,
    meter: &'a mut PrepareMeter<'m>,
    /// A backend invariant a code-generation step could not uphold.
    refused: Option<&'static str>,
}

impl<'a, 'm> Compiler<'a, 'm> {
    pub(crate) fn new(
        module: &'a mut JITModule,
        shim: &Shim,
        graph: &'a CheckedGraph,
        coverage: &'a fsl_eval::execution::BackendCoverage,
        ops: &'a mut Vec<Op>,
        meter: &'a mut PrepareMeter<'m>,
    ) -> Self {
        Self {
            module,
            shim: shim.id(),
            graph,
            coverage,
            ops,
            meter,
            state: HashMap::new(),
        }
    }

    /// The index of the function whose identity is `identity`.
    pub(crate) fn index_of(&self, identity: &filament_exact::NodeKey) -> Option<usize> {
        (0..)
            .map_while(|index| self.graph.function_identity_at(index))
            .position(|candidate| candidate == *identity)
    }

    /// Translate the function at `index` and its callees, then the entry
    /// trampoline that adapts it to the one entry signature (FR-568).
    pub(crate) fn entry(&mut self, index: usize) -> Result<Translated, Failure> {
        let (block, ty) = self.function(index, 0)?;
        self.entry_for(index, block, ty)
    }

    /// Translate a state clause body with no bound inputs. Clause callers only
    /// use this route after coverage analysis proves that the implicit `self`
    /// and all other clause bindings are absent from the expression.
    pub(crate) fn clause_entry(
        &mut self,
        index: usize,
        parameters: &[Option<Ty>],
        body: CheckedNode<'_>,
    ) -> Result<Translated, Failure> {
        let mut pending = vec![body];
        while let Some(node) = pending.pop() {
            if let NodeKind::Call {
                function: callee, ..
            } = node.kind()
            {
                match self.function(*callee, 1) {
                    Ok(_) => {}
                    Err(Failure::Unsupported(_)) => {
                        let key = self
                            .graph
                            .node_key(node)
                            .ok_or_else(|| fault("jit-call-site-has-node-key"))?;
                        return Err(Failure::Unsupported(Outside {
                            construct: filament_eval::UntranslatableConstruct::Operation(
                                key.clone(),
                            ),
                            location: node.location().clone(),
                        }));
                    }
                    Err(other) => return Err(other),
                }
            }
            pending.extend(node.children().into_iter().rev());
        }
        let (block, ty) = self.build(index, parameters, body)?;
        self.entry_for(index, block, ty)
    }

    fn entry_for(&mut self, index: usize, block: FuncId, ty: Ty) -> Result<Translated, Failure> {
        let pointer = self.module.target_config().pointer_type();
        let mut signature = self.module.make_signature();
        signature.params.push(AbiParam::new(pointer));
        signature.returns.push(AbiParam::new(types::I32));
        let id = self
            .module
            .declare_function(
                &format!("qsl_entry_{index}"),
                cranelift_module::Linkage::Export,
                &signature,
            )
            .map_err(|_| fault("jit-declare-entry"))?;
        let frontend = self.module.target_config();
        let mut context = self.module.make_context();
        let recorded = Signature::of(&signature, pointer);
        context.func.signature = signature;
        let mut builder_context = FunctionBuilderContext::new();
        {
            let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
            let body = builder.create_block();
            builder.append_block_params_for_function_params(body);
            builder.switch_to_block(body);
            let frame = builder.block_params(body)[0];
            let flags = MemFlagsData::trusted();
            let arguments = builder.ins().load(pointer, flags, frame, ARGUMENTS_OFFSET);
            let callee = self.module.declare_func_in_func(block, builder.func);
            let call = builder.ins().call(callee, &[frame, arguments]);
            let result = builder.inst_results(call)[0];
            builder.ins().store(flags, result, frame, RESULT_OFFSET);
            let failed = builder.ins().load(types::I64, flags, frame, FAILED_OFFSET);
            let status = builder.ins().ireduce(types::I32, failed);
            builder.ins().return_(&[status]);
            builder.seal_all_blocks();
            builder.finalize(frontend);
        }
        self.module
            .define_function(id, &mut context)
            .map_err(|_| fault("jit-define-entry"))?;
        self.module.clear_context(&mut context);
        Ok(Translated {
            id,
            result_is_boolean: ty == Ty::Boolean,
            result_is_integer: matches!(ty, Ty::Int(..)),
            signature: recorded,
        })
    }

    /// Translate the function at `index` (once) after the functions it calls.
    fn function(&mut self, index: usize, depth: usize) -> Result<(FuncId, Ty), Failure> {
        match self.state.get(&index) {
            Some(State::Done(id, ty)) => return Ok((*id, *ty)),
            Some(State::Unsupported) | Some(State::InProgress) => {
                return Err(self.placeholder(index));
            }
            None => {}
        }
        let graph = self.graph;
        let (Some(function), Some(identity)) = (
            graph.function_state(index),
            graph.function_identity_at(index),
        ) else {
            return Err(fault("jit-function-index"));
        };
        let Some(callable) = graph.callable_by_identity(identity) else {
            self.state.insert(index, State::Unsupported);
            return Err(self.placeholder(index));
        };
        if depth > MAX_DEPTH {
            self.state.insert(index, State::Unsupported);
            return Err(Failure::Unsupported(untranslated(function.body)));
        }
        let outside = first_outside_body(graph, callable.parameters, function.body, self.coverage)
            .map_err(|fault| Failure::Prepare(PrepareFailure::Fault(fault)))?;
        if let Some(outside) = outside {
            self.state.insert(index, State::Unsupported);
            return Err(Failure::Unsupported(outside));
        }
        self.state.insert(index, State::InProgress);
        // Callees first, so a call site finds them translated.
        let mut pending = vec![function.body];
        while let Some(node) = pending.pop() {
            if let NodeKind::Call {
                function: callee, ..
            } = node.kind()
            {
                match self.function(*callee, depth + 1) {
                    Ok(_) => {}
                    Err(Failure::Unsupported(_)) => {
                        self.state.insert(index, State::Unsupported);
                        let key = self
                            .graph
                            .node_key(node)
                            .ok_or_else(|| fault("jit-call-site-has-node-key"))?;
                        return Err(Failure::Unsupported(Outside {
                            construct: filament_eval::UntranslatableConstruct::Operation(
                                key.clone(),
                            ),
                            location: node.location().clone(),
                        }));
                    }
                    Err(other) => return Err(other),
                }
            }
            pending.extend(node.children().into_iter().rev());
        }
        let parameters: Vec<Option<Ty>> = callable
            .parameters
            .iter()
            .map(|(_, t)| type_of(t))
            .collect();
        match self.build(index, &parameters, function.body) {
            Ok((id, ty)) => {
                self.state.insert(index, State::Done(id, ty));
                Ok((id, ty))
            }
            Err(failure) => {
                self.state.insert(index, State::Unsupported);
                Err(failure)
            }
        }
    }

    fn placeholder(&self, index: usize) -> Failure {
        match self.graph.function_state(index) {
            Some(function) => Failure::Unsupported(untranslated(function.body)),
            None => fault("jit-function-index"),
        }
    }

    fn build(
        &mut self,
        index: usize,
        parameters: &[Option<Ty>],
        body: CheckedNode<'_>,
    ) -> Result<(FuncId, Ty), Failure> {
        let module = &mut *self.module;
        let pointer = module.target_config().pointer_type();
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(pointer));
        signature.params.push(AbiParam::new(pointer));
        signature.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function(
                &format!("qsl_f{index}"),
                cranelift_module::Linkage::Export,
                &signature,
            )
            .map_err(|_| fault("jit-declare-function"))?;
        let frontend = module.target_config();
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut builder_context = FunctionBuilderContext::new();
        let ty = {
            let mut builder = FunctionBuilder::new(&mut context.func, &mut builder_context);
            let entry = builder.create_block();
            builder.append_block_params_for_function_params(entry);
            builder.switch_to_block(entry);
            let (ctx_value, args) = (
                builder.block_params(entry)[0],
                builder.block_params(entry)[1],
            );
            let exit = builder.create_block();
            let mut emitter = Emitter {
                builder,
                module: &mut *module,
                shim: self.shim,
                context: ctx_value,
                exit,
                ops: &mut *self.ops,
                state: &self.state,
                slots: HashMap::new(),
                meter: &mut *self.meter,
                refused: None,
            };
            for (position, ty) in parameters.iter().enumerate() {
                if let Some(ty) = ty {
                    let offset =
                        i32::try_from(position * 8).map_err(|_| fault("jit-parameter-offset"))?;
                    let value = emitter.builder.ins().load(
                        types::I64,
                        MemFlagsData::trusted(),
                        args,
                        offset,
                    );
                    emitter.slots.insert(position, (value, *ty));
                }
            }
            let (result, ty) = emitter.emit(body, 0)?;
            if let Some(invariant) = emitter.refused {
                return Err(fault(invariant));
            }
            emitter.builder.ins().return_(&[result]);
            emitter.builder.switch_to_block(exit);
            let zero = emitter.builder.ins().iconst(types::I64, 0);
            emitter.builder.ins().return_(&[zero]);
            emitter.builder.seal_all_blocks();
            emitter.builder.finalize(frontend);
            ty
        };
        module
            .define_function(id, &mut context)
            .map_err(|_| fault("jit-define-function"))?;
        let code_bytes = match context.compiled_code() {
            Some(code) => {
                u64::try_from(code.code_buffer().len()).map_err(|_| fault("jit-code-size"))?
            }
            None => return Err(fault("jit-code-missing")),
        };
        module.clear_context(&mut context);
        self.meter.charge_block(code_bytes)?;
        Ok((id, ty))
    }
}

impl Emitter<'_, '_, '_> {
    fn unsupported(&self, node: CheckedNode<'_>) -> Failure {
        Failure::Unsupported(untranslated(node))
    }

    fn emit(&mut self, node: CheckedNode<'_>, depth: usize) -> Result<(Clif, Ty), Failure> {
        if depth > MAX_DEPTH {
            return Err(self.unsupported(node));
        }
        self.meter.charge_node()?;
        match node.kind() {
            NodeKind::Literal(literal) => match &literal.0 {
                Value::Boolean(value) => {
                    let v = self.builder.ins().iconst(types::I64, i64::from(*value));
                    Ok((v, Ty::Boolean))
                }
                value @ Value::Integer(_) => {
                    let Value::Integer(integer) = value else {
                        unreachable!("the matched integer literal is an Integer")
                    };
                    let exact = integer_to_i64(integer).ok_or_else(|| self.unsupported(node))?;
                    let value = self.builder.ins().iconst(types::I64, exact);
                    Ok((value, Ty::Int(i128::from(exact), i128::from(exact))))
                }
                value => {
                    if type_of(node.value_type()) != Some(Ty::Opaque) {
                        return Err(self.unsupported(node));
                    }
                    let zero = self.builder.ins().iconst(types::I64, 0);
                    let handle = self.shim_call(node, OpKind::Literal(value.clone()), zero, zero);
                    Ok((handle, Ty::Opaque))
                }
            },
            NodeKind::Record { declaration, slots } => {
                let zero = self.builder.ins().iconst(types::I64, 0);
                let builder = self.shim_call(node, OpKind::BuildStart, zero, zero);
                for slot in slots {
                    if let RecordSlot::Present(child) = slot {
                        let (value, ty) = self.emit(node.at(*child), depth + 1)?;
                        let (boolean, integer) = match ty {
                            Ty::Boolean => (true, false),
                            Ty::Int(..) => (false, true),
                            Ty::Opaque => (false, false),
                        };
                        let _ = self.shim_call(
                            node,
                            OpKind::BuildValue { boolean, integer },
                            builder,
                            value,
                        );
                    }
                }
                let result = self.shim_call(
                    node,
                    OpKind::BuildRecord {
                        declaration: declaration.clone(),
                        slots: slots.clone(),
                    },
                    builder,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::Tuple {
                declaration,
                arguments,
            } => {
                let zero = self.builder.ins().iconst(types::I64, 0);
                let builder = self.shim_call(node, OpKind::BuildStart, zero, zero);
                for child in arguments {
                    let (value, ty) = self.emit(node.at(*child), depth + 1)?;
                    let (boolean, integer) = match ty {
                        Ty::Boolean => (true, false),
                        Ty::Int(..) => (false, true),
                        Ty::Opaque => (false, false),
                    };
                    let _ = self.shim_call(
                        node,
                        OpKind::BuildValue { boolean, integer },
                        builder,
                        value,
                    );
                }
                let result = self.shim_call(
                    node,
                    OpKind::BuildTuple {
                        declaration: declaration.clone(),
                    },
                    builder,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                let zero = self.builder.ins().iconst(types::I64, 0);
                let builder = self.shim_call(node, OpKind::BuildStart, zero, zero);
                for child in elements {
                    let _ = self.shim_call(node, OpKind::ChargeElement, zero, zero);
                    let (value, ty) = self.emit(node.at(*child), depth + 1)?;
                    let (boolean, integer) = match ty {
                        Ty::Boolean => (true, false),
                        Ty::Int(..) => (false, true),
                        Ty::Opaque => (false, false),
                    };
                    let _ = self.shim_call(
                        node,
                        OpKind::BuildValue { boolean, integer },
                        builder,
                        value,
                    );
                }
                let result = self.shim_call(
                    node,
                    OpKind::BuildCollection {
                        collection_type: collection_type.clone(),
                    },
                    builder,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::Local(slot) => self
                .slots
                .get(slot)
                .copied()
                .ok_or_else(|| self.unsupported(node)),
            NodeKind::Let {
                slot, value, body, ..
            } => {
                let bound = self.emit(node.at(*value), depth + 1)?;
                let previous = self.slots.insert(*slot, bound);
                let result = self.emit(node.at(*body), depth + 1);
                match previous {
                    Some(previous) => self.slots.insert(*slot, previous),
                    None => self.slots.remove(slot),
                };
                result
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => self.emit_if(node, (*condition, *then, *otherwise), depth),
            NodeKind::Arithmetic(operator, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let (Ty::Int(al, ah), Ty::Int(bl, bh)) = (left_ty, right_ty) else {
                    return Err(self.unsupported(node));
                };
                let (low, high) = match operator {
                    Arithmetic::Add => (al.saturating_add(bl), ah.saturating_add(bh)),
                    Arithmetic::Subtract => (al.saturating_sub(bh), ah.saturating_sub(bl)),
                    Arithmetic::Multiply => {
                        let products = [
                            al.saturating_mul(bl),
                            al.saturating_mul(bh),
                            ah.saturating_mul(bl),
                            ah.saturating_mul(bh),
                        ];
                        (
                            products.into_iter().min().unwrap_or(0),
                            products.into_iter().max().unwrap_or(0),
                        )
                    }
                };
                if !fits_i64((low, high)) {
                    return Err(self.unsupported(node));
                }
                let result = self.shim_call(node, OpKind::Arithmetic(*operator), a, b);
                Ok((result, Ty::Int(low, high)))
            }
            NodeKind::Rational {
                operator,
                left,
                right,
                domain,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if left_ty != Ty::Opaque || right_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Rational {
                    operator: *operator,
                    domain: domain.clone(),
                };
                let result = self.shim_call(node, kind, a, b);
                Ok((result, Ty::Opaque))
            }
            NodeKind::RationalNegate(operand, domain) => {
                let (value, value_ty) = self.emit(node.at(*operand), depth + 1)?;
                if value_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result =
                    self.shim_call(node, OpKind::RationalNegate(domain.clone()), value, zero);
                Ok((result, Ty::Opaque))
            }
            NodeKind::Decimal {
                operator,
                left,
                right,
                target,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if left_ty != Ty::Opaque || right_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let result = self.shim_call(
                    node,
                    OpKind::Decimal {
                        operator: *operator,
                        target: target.clone(),
                    },
                    a,
                    b,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::DecimalNegate(operand, target) => {
                let (value, value_ty) = self.emit(node.at(*operand), depth + 1)?;
                if value_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result =
                    self.shim_call(node, OpKind::DecimalNegate(target.clone()), value, zero);
                Ok((result, Ty::Opaque))
            }
            NodeKind::Size(operand) => {
                let (value, value_ty) = self.emit(node.at(*operand), depth + 1)?;
                if value_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let result_type =
                    type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::Size, value, zero);
                Ok((result, result_type))
            }
            NodeKind::Field {
                operand,
                field,
                optional,
            } => {
                let (record, record_ty) = self.emit(node.at(*operand), depth + 1)?;
                if record_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let value_type = node.value_type().clone();
                if type_of(&value_type).is_none() {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::Field {
                        field: field.clone(),
                        optional: *optional,
                        value_type,
                    },
                    record,
                    zero,
                );
                let result_type =
                    type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                Ok((result, result_type))
            }
            NodeKind::Contains(collection, item) => {
                let (collection, collection_ty) = self.emit(node.at(*collection), depth + 1)?;
                let (item_value, item_ty) = self.emit(node.at(*item), depth + 1)?;
                if collection_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let (item_boolean, item_integer) = match item_ty {
                    Ty::Boolean => (true, false),
                    Ty::Int(..) => (false, true),
                    Ty::Opaque => (false, false),
                };
                let result = self.shim_call(
                    node,
                    OpKind::Contains {
                        item_boolean,
                        item_integer,
                    },
                    collection,
                    item_value,
                );
                Ok((result, Ty::Boolean))
            }
            NodeKind::TupleProject {
                operand,
                declaration,
                position,
            } => {
                let (tuple, tuple_ty) = self.emit(node.at(*operand), depth + 1)?;
                if tuple_ty != Ty::Opaque || type_of(node.value_type()).is_none() {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::TupleProject {
                        declaration: declaration.clone(),
                        position: *position,
                    },
                    tuple,
                    zero,
                );
                Ok((result, type_of(node.value_type()).unwrap_or(Ty::Opaque)))
            }
            NodeKind::Index {
                collection,
                index,
                kind:
                    filament_checked::node::IndexedKind::Sequence
                    | filament_checked::node::IndexedKind::OrderedSet,
            } => {
                let (collection, collection_ty) = self.emit(node.at(*collection), depth + 1)?;
                let (index_value, index_ty) = self.emit(node.at(*index), depth + 1)?;
                if collection_ty != Ty::Opaque || !matches!(index_ty, Ty::Int(..)) {
                    return Err(self.unsupported(node));
                }
                let result_type =
                    type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                let result = self.shim_call(node, OpKind::Index, collection, index_value);
                Ok((result, result_type))
            }
            NodeKind::DropFirst(collection) => {
                let (collection, collection_ty) = self.emit(node.at(*collection), depth + 1)?;
                if collection_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::DropFirst, collection, zero);
                Ok((result, Ty::Opaque))
            }
            NodeKind::RemoveOne(collection, item) => {
                let (collection, collection_ty) = self.emit(node.at(*collection), depth + 1)?;
                let (item, item_ty) = self.emit(node.at(*item), depth + 1)?;
                if collection_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let (item_boolean, item_integer) = match item_ty {
                    Ty::Boolean => (true, false),
                    Ty::Int(..) => (false, true),
                    Ty::Opaque => (false, false),
                };
                let result = self.shim_call(
                    node,
                    OpKind::RemoveOne {
                        item_boolean,
                        item_integer,
                    },
                    collection,
                    item,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::Flatten(collection) => {
                let (collection, collection_ty) = self.emit(node.at(*collection), depth + 1)?;
                if collection_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::Flatten {
                        result_type: node.value_type().clone(),
                    },
                    collection,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::ConvertCollection { target, operand } => {
                let (collection, collection_ty) = self.emit(node.at(*operand), depth + 1)?;
                if collection_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::ConvertCollection {
                        target: target.clone(),
                    },
                    collection,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::ConvertDecimal(operand, target) => {
                let (value, value_ty) = self.emit(node.at(*operand), depth + 1)?;
                if value_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::ConvertDecimal {
                        target: target.clone(),
                    },
                    value,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::ConvertText(target, operand) => {
                let (source, source_ty) = self.emit(node.at(*operand), depth + 1)?;
                if source_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let result_ty = type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::ConvertText {
                        target: target.clone(),
                    },
                    source,
                    zero,
                );
                Ok((result, result_ty))
            }
            NodeKind::ConvertScalar(operand, source) => {
                let (source, source_ty) = self.emit(node.at(*source), depth + 1)?;
                if source_ty == Ty::Boolean {
                    return Err(self.unsupported(node));
                }
                let result_ty = type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                if !matches!(result_ty, Ty::Int(..) | Ty::Opaque) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::ConvertScalar {
                        operand: operand.clone(),
                        source_boolean: source_ty == Ty::Boolean,
                        source_integer: matches!(source_ty, Ty::Int(..)),
                        integer_result: matches!(result_ty, Ty::Int(..)),
                    },
                    source,
                    zero,
                );
                Ok((result, result_ty))
            }
            NodeKind::Query {
                visit,
                slot,
                source,
                body,
                ..
            } => {
                if matches!(visit, Visit::Forall | Visit::Exists) {
                    return Err(self.unsupported(node));
                }
                let source_node = node.at(*source);
                let (source_value, source_ty, collection_type) = match source_node.kind() {
                    NodeKind::NamedPopulation { population, members } if members.len() == 1 => {
                        let ValueType::Population(maximum) = source_node.value_type() else {
                            return Err(self.unsupported(source_node));
                        };
                        let collection_type = Arc::new(CollectionType::new(
                            CollectionKind::Set,
                            ValueType::Reference(members[0].clone()),
                            maximum.map(CardinalityBound::at_most),
                        ));
                        let value_type = ValueType::Collection(Arc::clone(&collection_type));
                        let zero = self.builder.ins().iconst(types::I64, 0);
                        let result = self.shim_call(
                            node,
                            OpKind::NamedAllInstances {
                                population: population.clone(),
                                members: members.clone(),
                                value_type,
                            },
                            zero,
                            zero,
                        );
                        (result, Ty::Opaque, collection_type)
                    }
                    NodeKind::NamedPopulation { .. } => return Err(self.unsupported(source_node)),
                    _ => {
                        let ValueType::Collection(collection_type) = source_node.value_type() else {
                            return Err(self.unsupported(node));
                        };
                        let (value, ty) = self.emit(source_node, depth + 1)?;
                        (value, ty, collection_type.clone())
                    }
                };
                let element_ty = type_of(collection_type.element())
                    .ok_or_else(|| self.unsupported(node))?;
                let element_boolean = element_ty == Ty::Boolean;
                let element_integer = matches!(element_ty, Ty::Int(..));
                let only_source_filter = matches!(
                    source_node.kind(),
                    NodeKind::Query {
                        visit: Visit::Filter,
                        ..
                    }
                );
                if source_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let shape = IterationShape::Query(*visit);
                let zero = self.builder.ins().iconst(types::I64, 0);
                let iteration = self.shim_call(
                    node,
                    OpKind::IterationStart {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter,
                        reduce: false,
                        has_identity: false,
                        identity_boolean: false,
                        identity_integer: false,
                    },
                    source_value,
                    zero,
                );
                let header = self.builder.create_block();
                let visit_block = self.builder.create_block();
                let finish_block = self.builder.create_block();
                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);
                let item = self.shim_call(
                    node,
                    OpKind::IterationNext {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter,
                        element_boolean,
                        element_integer,
                    },
                    iteration,
                    zero,
                );
                let available = self
                    .builder
                    .ins()
                    .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, item, 0);
                self.builder
                    .ins()
                    .brif(available, visit_block, &[], finish_block, &[]);
                self.builder.switch_to_block(visit_block);
                let previous = self.slots.insert(*slot, (item, element_ty));
                let body_result = self.emit(node.at(*body), depth + 1);
                match previous {
                    Some(previous) => self.slots.insert(*slot, previous),
                    None => self.slots.remove(slot),
                };
                let (body_value, body_ty) = body_result?;
                let _ = self.shim_call(
                    node,
                    OpKind::IterationAbsorb {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter,
                        result_boolean: body_ty == Ty::Boolean,
                        result_integer: matches!(body_ty, Ty::Int(..)),
                    },
                    iteration,
                    body_value,
                );
                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(finish_block);
                let result_ty = type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                let result = self.shim_call(
                    node,
                    OpKind::IterationFinish {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter,
                        result_boolean: result_ty == Ty::Boolean,
                        result_integer: matches!(result_ty, Ty::Int(..)),
                    },
                    iteration,
                    zero,
                );
                Ok((result, result_ty))
            }
            NodeKind::PopulationObjects { population } => {
                let value_type = node.value_type().clone();
                if !matches!(value_type, ValueType::Collection(_)) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::PopulationObjects {
                        population: population.clone(),
                        value_type,
                    },
                    zero,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::AllInstances { population } => {
                let NodeKind::NamedPopulation {
                    population,
                    members,
                } = node.at(*population).kind()
                else {
                    return Err(self.unsupported(node));
                };
                let value_type = node.value_type().clone();
                if !matches!(value_type, ValueType::Collection(_)) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(
                    node,
                    OpKind::NamedAllInstances {
                        population: population.clone(),
                        members: members.clone(),
                        value_type,
                    },
                    zero,
                    zero,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::Fold {
                accumulator,
                binder,
                source,
                step,
                identity,
                ..
            } => {
                let source_node = node.at(*source);
                let ValueType::Collection(collection_type) = source_node.value_type() else {
                    return Err(self.unsupported(node));
                };
                let element_ty = type_of(collection_type.element())
                    .ok_or_else(|| self.unsupported(node))?;
                let element_boolean = element_ty == Ty::Boolean;
                let element_integer = matches!(element_ty, Ty::Int(..));
                let (source_value, source_ty) = self.emit(source_node, depth + 1)?;
                if source_ty != Ty::Opaque {
                    return Err(self.unsupported(node));
                }
                let identity_value = identity
                    .map(|identity| self.emit(node.at(identity), depth + 1))
                    .transpose()?;
                let (identity_value, identity_ty) = identity_value
                    .map_or((self.builder.ins().iconst(types::I64, 0), None), |(value, ty)| {
                        (value, Some(ty))
                    });
                let identity_boolean = identity_ty == Some(Ty::Boolean);
                let identity_integer = matches!(identity_ty, Some(Ty::Int(..)));
                let mut accumulator_ty = type_of(node.at(*step).value_type())
                    .ok_or_else(|| self.unsupported(node))?;
                if let (Some(Ty::Int(identity_low, identity_high)), Ty::Int(_, _)) =
                    (identity_ty, accumulator_ty)
                {
                    if let NodeKind::Arithmetic(Arithmetic::Add, left, right) =
                        node.at(*step).kind()
                    {
                        let accumulator_left = matches!(
                            node.at(*left).kind(),
                            NodeKind::Local(slot) if *slot == *accumulator
                        );
                        let accumulator_right = matches!(
                            node.at(*right).kind(),
                            NodeKind::Local(slot) if *slot == *accumulator
                        );
                        let matching_binder = (accumulator_left
                            && matches!(node.at(*right).kind(), NodeKind::Local(slot) if *slot == *binder))
                            || (accumulator_right
                                && matches!(node.at(*left).kind(), NodeKind::Local(slot) if *slot == *binder));
                        if matching_binder
                        {
                            let maximum = collection_type
                                .bound()
                                .map(|bound| i128::from(bound.maximum()))
                                .ok_or_else(|| self.unsupported(node))?;
                            let Ty::Int(element_low, element_high) = element_ty else {
                                return Err(self.unsupported(node));
                            };
                            let low = identity_low
                                .checked_add(element_low.min(0).checked_mul(maximum).ok_or_else(|| self.unsupported(node))?)
                                .ok_or_else(|| self.unsupported(node))?;
                            let high = identity_high
                                .checked_add(element_high.max(0).checked_mul(maximum).ok_or_else(|| self.unsupported(node))?)
                                .ok_or_else(|| self.unsupported(node))?;
                            if !fits_i64((low, high)) {
                                return Err(self.unsupported(node));
                            }
                            accumulator_ty = Ty::Int(low, high);
                        }
                    }
                }
                if identity_ty.is_some_and(|ty| !same_register_type(ty, accumulator_ty)) {
                    return Err(self.unsupported(node));
                }
                let shape = IterationShape::Fold;
                let reduce = identity.is_none();
                let iteration = self.shim_call(
                    node,
                    OpKind::IterationStart {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter: false,
                        reduce,
                        has_identity: identity.is_some(),
                        identity_boolean,
                        identity_integer,
                    },
                    source_value,
                    identity_value,
                );
                let zero = self.builder.ins().iconst(types::I64, 0);
                let header = self.builder.create_block();
                let visit_block = self.builder.create_block();
                let finish_block = self.builder.create_block();
                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);
                let item = self.shim_call(
                    node,
                    OpKind::IterationNext {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter: false,
                        element_boolean,
                        element_integer,
                    },
                    iteration,
                    zero,
                );
                let available = self
                    .builder
                    .ins()
                    .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, item, 0);
                self.builder
                    .ins()
                    .brif(available, visit_block, &[], finish_block, &[]);
                self.builder.switch_to_block(visit_block);
                let accumulator_value = self.shim_call(
                    node,
                    OpKind::IterationAccumulator {
                        boolean: accumulator_ty == Ty::Boolean,
                        integer: matches!(accumulator_ty, Ty::Int(..)),
                    },
                    iteration,
                    zero,
                );
                let prior_accumulator = self
                    .slots
                    .insert(*accumulator, (accumulator_value, accumulator_ty));
                let prior_binder = self.slots.insert(*binder, (item, element_ty));
                let step_result = self.emit(node.at(*step), depth + 1);
                match prior_accumulator {
                    Some(previous) => self.slots.insert(*accumulator, previous),
                    None => self.slots.remove(accumulator),
                };
                match prior_binder {
                    Some(previous) => self.slots.insert(*binder, previous),
                    None => self.slots.remove(binder),
                };
                let (step_value, step_ty) = step_result?;
                let _ = self.shim_call(
                    node,
                    OpKind::IterationAbsorb {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter: false,
                        result_boolean: step_ty == Ty::Boolean,
                        result_integer: matches!(step_ty, Ty::Int(..)),
                    },
                    iteration,
                    step_value,
                );
                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(finish_block);
                let result_ty = type_of(node.value_type()).ok_or_else(|| self.unsupported(node))?;
                let result = self.shim_call(
                    node,
                    OpKind::IterationFinish {
                        shape,
                        value_type: node.value_type().clone(),
                        only_source_filter: false,
                        result_boolean: result_ty == Ty::Boolean,
                        result_integer: matches!(result_ty, Ty::Int(..)),
                    },
                    iteration,
                    zero,
                );
                Ok((result, result_ty))
            }
            NodeKind::Negate(operand) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                let Ty::Int(low, high) = ty else {
                    return Err(self.unsupported(node));
                };
                let interval = (high.saturating_neg(), low.saturating_neg());
                if !fits_i64(interval) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::Negate, a, zero);
                Ok((result, Ty::Int(interval.0, interval.1)))
            }
            NodeKind::IntegerDivision {
                operator,
                selection,
                left,
                right,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if !matches!((left_ty, right_ty), (Ty::Int(..), Ty::Int(..))) {
                    return Err(self.unsupported(node));
                }
                if !matches!(operator, IntegerDivisionOperator::Mod) && selection.is_none() {
                    return Err(self.unsupported(node));
                }
                let (Ty::Int(al, ah), Ty::Int(bl, bh)) = (left_ty, right_ty) else {
                    return Err(self.unsupported(node));
                };
                let Some((low, high)) = division_interval(*operator, *selection, (al, ah), (bl, bh)) else {
                    return Err(self.unsupported(node));
                };
                if !fits_i64((low, high)) {
                    return Err(self.unsupported(node));
                }
                let kind = OpKind::Divide {
                    operator: *operator,
                    selection: *selection,
                };
                let result = self.shim_call(node, kind, a, b);
                Ok((result, Ty::Int(low, high)))
            }
            NodeKind::Divide {
                left,
                right,
                domain,
            } => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                if !matches!((left_ty, right_ty), (Ty::Int(..), Ty::Int(..))) {
                    return Err(self.unsupported(node));
                }
                let result = self.shim_call(
                    node,
                    OpKind::ExactDivide(domain.clone()),
                    a,
                    b,
                );
                Ok((result, Ty::Opaque))
            }
            NodeKind::Order(operator, kind, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let supported = match kind {
                    OrderedKind::Integers => {
                        matches!((left_ty, right_ty), (Ty::Int(..), Ty::Int(..)))
                    }
                    OrderedKind::Rationals | OrderedKind::Decimals | OrderedKind::Texts => {
                        left_ty == Ty::Opaque && right_ty == Ty::Opaque
                    }
                    _ => false,
                };
                if !supported {
                    return Err(self.unsupported(node));
                }
                let result = self.shim_call(
                    node,
                    OpKind::Order {
                        operator: *operator,
                        kind: *kind,
                    },
                    a,
                    b,
                );
                Ok((result, Ty::Boolean))
            }
            NodeKind::Coerce(operand, target) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                if !matches!(ty, Ty::Int(..)) {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::Coerce(target.clone()), a, zero);
                let result_type = type_of(node.value_type()).unwrap_or(ty);
                Ok((result, result_type))
            }
            NodeKind::Equality(_, checked, left, right) => {
                let (a, left_ty) = self.emit(node.at(*left), depth + 1)?;
                let (b, right_ty) = self.emit(node.at(*right), depth + 1)?;
                let left_boolean = left_ty == Ty::Boolean;
                let left_integer = matches!(left_ty, Ty::Int(..));
                let right_boolean = right_ty == Ty::Boolean;
                let right_integer = matches!(right_ty, Ty::Int(..));
                let kind = OpKind::Equality {
                    checked: checked.clone(),
                    left_boolean,
                    left_integer,
                    right_boolean,
                    right_integer,
                };
                let result = self.shim_call(node, kind, a, b);
                Ok((result, Ty::Boolean))
            }
            NodeKind::Not(operand) => {
                let (a, ty) = self.emit(node.at(*operand), depth + 1)?;
                if ty != Ty::Boolean {
                    return Err(self.unsupported(node));
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let result = self.shim_call(node, OpKind::Not, a, zero);
                Ok((result, Ty::Boolean))
            }
            NodeKind::Connective(connective, _, left, right) => {
                self.emit_connective(node, (*connective, *left, *right), depth)
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let Some(State::Done(callee, result_ty)) = self.state.get(function) else {
                    return Err(self.unsupported(node));
                };
                let (callee, result_ty) = (*callee, *result_ty);
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    values.push(self.emit(node.at(*argument), depth + 1)?);
                }
                let zero = self.builder.ins().iconst(types::I64, 0);
                let charged = self.shim_call(node, OpKind::ChargeCall, zero, zero);
                let _ = charged;
                let arguments_builder = self.shim_call(node, OpKind::FrameStart, zero, zero);
                for (value, ty) in &values {
                    let _ = self.shim_call(
                        node,
                        OpKind::FrameArgument {
                            boolean: *ty == Ty::Boolean,
                            integer: matches!(ty, Ty::Int(..)),
                        },
                        arguments_builder,
                        *value,
                    );
                }
                let _ = self.shim_call(node, OpKind::FrameEnter, arguments_builder, zero);
                let Ok(size) = u32::try_from(values.len().max(1) * 8) else {
                    return Err(self.unsupported(node));
                };
                let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                ));
                for (position, (value, _)) in values.iter().enumerate() {
                    let Ok(offset) = i32::try_from(position * 8) else {
                        return Err(self.unsupported(node));
                    };
                    self.builder
                        .ins()
                        .stack_store(types::I64, *value, slot, offset);
                }
                let pointer = self.module.target_config().pointer_type();
                let address = self.builder.ins().stack_addr(pointer, slot, 0);
                let function_ref = self.module.declare_func_in_func(callee, self.builder.func);
                let call = self
                    .builder
                    .ins()
                    .call(function_ref, &[self.context, address]);
                let result = self.builder.inst_results(call)[0];
                self.leave_on_failure();
                let _ = self.shim_call(node, OpKind::FrameLeave, arguments_builder, zero);
                Ok((result, result_ty))
            }
            _ => Err(self.unsupported(node)),
        }
    }

    fn emit_connective(
        &mut self,
        node: CheckedNode<'_>,
        (connective, left, right): (
            Connective,
            fsl_semantics::check::NodeId,
            fsl_semantics::check::NodeId,
        ),
        depth: usize,
    ) -> Result<(Clif, Ty), Failure> {
        let (selector, left_ty) = self.emit(node.at(left), depth + 1)?;
        if left_ty != Ty::Boolean {
            return Err(self.unsupported(node));
        }
        let (skip, evaluate, merge) = (
            self.builder.create_block(),
            self.builder.create_block(),
            self.builder.create_block(),
        );
        self.builder.append_block_param(merge, types::I64);
        // The left operand alone decides `and` when false, `or` when true and
        // `implies` when false; the decided result is retained (charged) like
        // any other connective result.
        let (decided, when_decided) = match connective {
            Connective::And => (false, false),
            Connective::Or => (true, true),
            Connective::Implies => (true, false),
        };
        if when_decided {
            self.builder.ins().brif(selector, skip, &[], evaluate, &[]);
        } else {
            self.builder.ins().brif(selector, evaluate, &[], skip, &[]);
        }
        self.builder.switch_to_block(skip);
        let constant = self.builder.ins().iconst(types::I64, i64::from(decided));
        let zero = self.builder.ins().iconst(types::I64, 0);
        let retained = self.shim_call(node, OpKind::Retain, constant, zero);
        self.builder.ins().jump(merge, &[retained.into()]);
        self.builder.switch_to_block(evaluate);
        let (value, right_ty) = self.emit(node.at(right), depth + 1)?;
        if right_ty != Ty::Boolean {
            return Err(self.unsupported(node));
        }
        let zero = self.builder.ins().iconst(types::I64, 0);
        let retained = self.shim_call(node, OpKind::Retain, value, zero);
        self.builder.ins().jump(merge, &[retained.into()]);
        self.builder.switch_to_block(merge);
        let joined = self.builder.block_params(merge)[0];
        Ok((joined, Ty::Boolean))
    }

    fn emit_if(
        &mut self,
        node: CheckedNode<'_>,
        (condition, then, otherwise): (
            fsl_semantics::check::NodeId,
            fsl_semantics::check::NodeId,
            fsl_semantics::check::NodeId,
        ),
        depth: usize,
    ) -> Result<(Clif, Ty), Failure> {
        let (selector, condition_ty) = self.emit(node.at(condition), depth + 1)?;
        if condition_ty != Ty::Boolean {
            return Err(self.unsupported(node));
        }
        let (then_block, else_block, merge) = (
            self.builder.create_block(),
            self.builder.create_block(),
            self.builder.create_block(),
        );
        self.builder.append_block_param(merge, types::I64);
        self.builder
            .ins()
            .brif(selector, then_block, &[], else_block, &[]);
        self.builder.switch_to_block(then_block);
        let (taken, then_ty) = self.emit(node.at(then), depth + 1)?;
        self.builder.ins().jump(merge, &[taken.into()]);
        self.builder.switch_to_block(else_block);
        let (other, else_ty) = self.emit(node.at(otherwise), depth + 1)?;
        self.builder.ins().jump(merge, &[other.into()]);
        self.builder.switch_to_block(merge);
        let joined = self.builder.block_params(merge)[0];
        let ty = match (then_ty, else_ty) {
            (Ty::Boolean, Ty::Boolean) => Ty::Boolean,
            (Ty::Int(a, b), Ty::Int(c, d)) => Ty::Int(a.min(c), b.max(d)),
            (Ty::Opaque, Ty::Opaque) => Ty::Opaque,
            _ => return Err(self.unsupported(node)),
        };
        Ok((joined, ty))
    }

    /// Call the kernel shim for `kind` at `node`, then leave through `exit`
    /// when the shim recorded a stop (FR-561: the status is passed up
    /// unchanged).
    fn shim_call(&mut self, node: CheckedNode<'_>, kind: OpKind, a: Clif, b: Clif) -> Clif {
        let Ok(index) = i64::try_from(self.ops.len()) else {
            self.refused = Some("jit-operation-index");
            return a;
        };
        self.ops.push(Op {
            kind,
            location: node.location().clone(),
        });
        let function = self
            .module
            .declare_func_in_func(self.shim, self.builder.func);
        let index = self.builder.ins().iconst(types::I64, index);
        let call = self
            .builder
            .ins()
            .call(function, &[self.context, index, a, b]);
        let result = self.builder.inst_results(call)[0];
        self.leave_on_failure();
        result
    }

    /// Return from the function when the call just made recorded a stop.
    fn leave_on_failure(&mut self) {
        let failed = self.builder.ins().load(
            types::I64,
            MemFlagsData::trusted(),
            self.context,
            FAILED_OFFSET,
        );
        let next = self.builder.create_block();
        self.builder.ins().brif(failed, self.exit, &[], next, &[]);
        self.builder.switch_to_block(next);
    }
}

/// A sound interval for the kernel's `div`, `rem` or `mod` of operands in
/// the given intervals, whose divisor interval excludes 0.
///
/// The quotient is monotone in each operand while the divisor keeps one
/// sign, so its extremes are at the operand interval ends and are found by
/// running the kernel there with an unlimited meter. The remainder is not
/// monotone (`a mod 5` over `a` in `[0, 10]` takes 0 at both ends and 4 in
/// between), so `rem` and `mod` take the interval every admitted law keeps
/// them in: strictly smaller than the divisor in magnitude. FR-562's table
/// text, which takes the extremes at the ends for all three, is unsound for
/// the remainders; this follows the soundness the rule exists for.
fn division_interval(
    operator: IntegerDivisionOperator,
    selection: Option<AdmittedIntegerDivision>,
    (al, ah): (i128, i128),
    (bl, bh): (i128, i128),
) -> Option<(i128, i128)> {
    if operator == IntegerDivisionOperator::Rem && selection.is_none() {
        return None;
    }
    if matches!(
        operator,
        IntegerDivisionOperator::Rem | IntegerDivisionOperator::Mod
    ) {
        let bound = bl.saturating_abs().max(bh.saturating_abs()).saturating_sub(1);
        return Some((-bound, bound));
    }
    let mut meter = Meter::new(ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    });
    let mut low = i128::MAX;
    let mut high = i128::MIN;
    for dividend in [al, ah] {
        for divisor in [bl, bh] {
            let result = divide(
                operator,
                selection.as_ref(),
                &Integer::from(dividend),
                &Integer::from(divisor),
                &mut meter,
            )?;
            let Outcome::Completed(result) = result else {
                return None;
            };
            let value = integer_to_i64(&result)?;
            low = low.min(value.into());
            high = high.max(value.into());
        }
    }
    Some((low, high))
}

/// The kernel call the interpreter makes for `operator` (S6a,
/// `IntegerDivision`), or `None` when `div` or `rem` has no admitted
/// selection.
pub(crate) fn divide(
    operator: IntegerDivisionOperator,
    selection: Option<&AdmittedIntegerDivision>,
    dividend: &Integer,
    divisor: &Integer,
    meter: &mut Meter,
) -> Option<Outcome<Integer>> {
    let member = match operator {
        IntegerDivisionOperator::Mod => {
            return Some(filament_eval::modulo(
                dividend,
                divisor,
                &IntegerDomain::Mathematical,
                meter,
            ))
        }
        IntegerDivisionOperator::Div => DivisionMember::Quotient,
        IntegerDivisionOperator::Rem => DivisionMember::Remainder,
    };
    Some(fsl_semantics::value::divide(
        selection?,
        member,
        dividend,
        divisor,
        &IntegerDomain::Mathematical,
        meter,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A remainder with no admitted division selection has no interval, so the
    /// entry is listed unsupported at prepare instead of faulting at run time
    /// (`jit-division-selection-missing`); `mod` needs none.
    #[test]
    fn a_remainder_without_a_selection_has_no_interval() {
        assert_eq!(
            division_interval(IntegerDivisionOperator::Rem, None, (0, 10), (1, 5)),
            None
        );
        assert_eq!(
            division_interval(IntegerDivisionOperator::Mod, None, (0, 10), (1, 5)),
            Some((-4, 4))
        );
    }
}
