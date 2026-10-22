//! One value runner. User calls, operands and generators suspend in flat vectors.
use super::prepared::ReadSlot;
use super::{
    Budget, ConstructedValue, DeclaredType, Frame, KeyAxis, KeyedEntry, Located, Location,
    PreparedArgument, PreparedArguments, PreparedGenerator, PreparedKind, PreparedValue,
    QueryError, RuntimeValue, ValueBinaryOperator, ValueConstructor, ValueKey, ValueQueryHost,
    access_declaration, admit_container, admit_diagnostic, allocation, annotate, binary, boolean,
    constructed_value_type, copy_axes, copy_constructed_value, copy_declaration, entries,
    equal_keys, error, expose, extremum_position, finish_keyed, insert_set, integer, integer_range,
    key_positions, key_set_type, key_value, metadata_vec, optional_position, push_metadata,
    record_declaration, reduce_ref, scalar, sort_value, source_axis, string_match_ref, sum,
    take_one, take_position, tally_value, unary, underlying, unique_value, value_keys, zip_exact,
};

use super::identity::HashIndex;
use std::rc::Rc;

pub(crate) struct Body<'a, Q, K> {
    pub declaration: crate::functions::DeclarationRef<'a, Q, K>,
    pub bindings: Vec<(&'a str, PreparedValue<'a, Q>)>,
    pub value: PreparedValue<'a, Q>,
}

#[derive(Clone)]
pub(crate) enum NativeStep {
    Record(usize),
    Tuple(usize),
    List(usize),
    // Constructed steps form a suffix after the last native wrapper. The owner
    // stays native; matching and scalar reads borrow this selected subtree.
    Constructed(usize),
    Declaration(std::sync::Arc<DeclaredType<()>>),
}
impl NativeStep {
    pub fn index(&self) -> usize {
        match self {
            Self::Record(i) | Self::Tuple(i) | Self::List(i) | Self::Constructed(i) => *i,
            Self::Declaration(_) => unreachable!("declaration is not a child selector"),
        }
    }
}
pub(crate) struct NativeView<N> {
    pub owner: std::sync::Arc<N>,
    pub path: Vec<NativeStep>,
}
impl<N> NativeView<N> {
    pub fn new(owner: std::sync::Arc<N>) -> Self {
        Self {
            owner,
            path: Vec::new(),
        }
    }
    pub fn child(
        &self,
        step: NativeStep,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        let len = self.path.len()
            - usize::from(matches!(self.path.last(), Some(NativeStep::Declaration(_))));
        let mut path = metadata_vec(len + 1, budget, location)?;
        path.extend_from_slice(&self.path[..len]);
        path.push(step);
        Ok(Self {
            owner: std::sync::Arc::clone(&self.owner),
            path,
        })
    }
    pub fn share(&self, budget: &mut Budget, location: &Location) -> Result<Self, QueryError> {
        let mut path = metadata_vec(self.path.len(), budget, location)?;
        path.extend_from_slice(&self.path);
        budget.charge(1, location.clone())?;
        Ok(Self {
            owner: std::sync::Arc::clone(&self.owner),
            path,
        })
    }
    pub fn retain_declaration(
        &mut self,
        declaration: std::sync::Arc<DeclaredType<()>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        budget.charge(1, location.clone())?;
        if let Some(NativeStep::Declaration(old)) = self.path.last_mut() {
            *old = declaration;
        } else {
            push_metadata(
                &mut self.path,
                NativeStep::Declaration(declaration),
                budget,
                location,
            )?;
        }
        Ok(())
    }
    pub fn native<'v>(
        &'v self,
        record: impl Fn(&'v N) -> Option<&'v crate::NativeCapture<N>>,
        tuple: impl Fn(&'v N) -> Option<&'v [N]>,
        list: impl Fn(&'v N) -> Option<&'v [N]>,
    ) -> &'v N {
        let mut native = self.owner.as_ref();
        for step in &self.path {
            native = match step {
                NativeStep::Record(i) => {
                    &record(native).expect("retained record view").fields[*i].1
                }
                NativeStep::Tuple(i) => &tuple(native).expect("retained tuple view")[*i],
                NativeStep::List(i) => &list(native).expect("retained list view")[*i],
                _ => break,
            };
        }
        native
    }
    pub fn constructed<'v>(
        &'v self,
        constructed: impl Fn(&'v N) -> Option<&'v ConstructedValue>,
        record: impl Fn(&'v N) -> Option<&'v crate::NativeCapture<N>>,
        tuple: impl Fn(&'v N) -> Option<&'v [N]>,
        list: impl Fn(&'v N) -> Option<&'v [N]>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Option<ConstructedRef<'v>>, QueryError> {
        budget.charge(self.path.len(), location.clone())?;
        let native = self.native(record, tuple, list);
        let Some(value) = constructed(native) else {
            return Ok(None);
        };
        let mut data = ConstructedRef {
            value,
            declaration: super::declaration(value),
        };
        budget.charge(self.path.len(), location.clone())?;
        for step in &self.path {
            match step {
                NativeStep::Constructed(i) => data = data.child(*i, budget, location)?,
                NativeStep::Declaration(d) => data.declaration = Some(d),
                _ => {}
            }
        }
        Ok(Some(data))
    }
}

pub(crate) enum Data<N> {
    Constructed(RuntimeValue),
    Native(NativeView<N>),
}
pub(crate) struct Value<'a, N, K> {
    pub data: Data<N>,
    pub contract: Option<&'a DeclaredType<K>>,
}
impl<'a, N, K> Value<'a, N, K> {
    pub fn constructed(value: RuntimeValue) -> Self {
        Self {
            data: Data::Constructed(value),
            contract: None,
        }
    }
    pub fn share(&mut self, budget: &mut Budget, location: &Location) -> Result<Self, QueryError> {
        let data = match &mut self.data {
            Data::Constructed(value) => Data::Constructed(value.share(budget, location)?),
            Data::Native(value) => Data::Native(value.share(budget, location)?),
        };
        Ok(Self {
            data,
            contract: self.contract,
        })
    }
    fn scalar(&self, location: &Location) -> Result<&ConstructedValue, QueryError> {
        match &self.data {
            Data::Constructed(value) => Ok(value.as_ref()),
            Data::Native(_) => Err(error(
                location,
                "native handles require an explicit scalar projection; computational values cannot be rendered as data",
            )),
        }
    }
    pub fn materialize(
        self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<ConstructedValue, QueryError> {
        match self.data {
            Data::Constructed(value) => {
                let value = value.materialize(budget, location)?;
                if let Some(contract) = self.contract {
                    if crate::runtime_types::contains_native(contract, budget, location)? {
                        Ok(value)
                    } else {
                        annotate(
                            value,
                            crate::runtime_types::constructed_declaration(
                                contract, budget, location,
                            )?,
                            budget,
                            location,
                        )
                    }
                } else {
                    Ok(value)
                }
            }
            Data::Native(_) => Err(error(
                location,
                "native handles require an explicit scalar projection; computational values cannot be rendered as data",
            )),
        }
    }
    pub fn materialize_with<Q, H: Host<'a, Q, Native = N, Kind = K> + ?Sized>(
        self,
        host: &H,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<ConstructedValue, QueryError> {
        host.materialize(self, budget, location)
    }
}

type Slots<'a, N, K> = [(&'a str, Value<'a, N, K>)];
pub(crate) struct ConstructedRef<'v> {
    pub value: &'v ConstructedValue,
    pub declaration: Option<&'v DeclaredType<()>>,
}
impl<'v> ConstructedRef<'v> {
    pub fn child(
        &self,
        index: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Self, QueryError> {
        let data = self.value.data();
        let declaration = match (self.declaration.map(underlying), data) {
            (Some(DeclaredType::Dictionary(_, child)), _) => Some(child.as_ref()),
            (Some(DeclaredType::Tuple(children)), _) => children.get(index),
            (Some(DeclaredType::Record(fields)), ConstructedValue::Record(values)) => Some(
                record_declaration(fields, &values[index].0, budget, location)?,
            ),
            (
                Some(
                    DeclaredType::Array(child)
                    | DeclaredType::Sequence(child)
                    | DeclaredType::Keyed(child),
                ),
                _,
            ) => Some(child.as_ref()),
            _ => None,
        };
        let value = match data {
            ConstructedValue::Record(fields) => &fields[index].1,
            ConstructedValue::Dictionary(entries) => &entries[index].1,
            ConstructedValue::Array(values) | ConstructedValue::Tuple(values) => &values[index],
            ConstructedValue::Set { elements, .. } => &elements[index],
            ConstructedValue::KeyedArray { entries, .. } => &entries[index].value,
            _ => unreachable!("selected constructed child"),
        };
        Ok(Self {
            value,
            declaration: super::declaration(value).or(declaration),
        })
    }
    pub fn materialize(
        &self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<ConstructedValue, QueryError> {
        let value = copy_constructed_value(self.value, budget, location.clone())?;
        match self.declaration {
            Some(d) => annotate(
                value,
                copy_declaration(d, budget, location)?,
                budget,
                location,
            ),
            None => Ok(value),
        }
    }
}
pub(crate) type DictionaryEntry<'a, N, K> = (Value<'a, N, K>, Value<'a, N, K>);

pub(crate) trait Host<'a, Q> {
    type Native;
    type Kind: PartialEq;
    type Error: From<QueryError>;
    fn native_constructed<'v>(&self, _value: &'v Self::Native) -> Option<&'v ConstructedValue> {
        None
    }
    fn constructed<'v>(
        &self,
        value: &'v Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Option<ConstructedRef<'v>>, QueryError> {
        match &value.data {
            Data::Constructed(data) => Ok(Some(ConstructedRef {
                value: data.as_ref(),
                declaration: data.declaration(),
            })),
            Data::Native(view) => view.constructed(
                |v| self.native_constructed(v),
                |v| self.native_record(v),
                |v| self.native_tuple(v),
                |v| self.native_list(v),
                budget,
                location,
            ),
        }
    }
    fn scalar<'v>(
        &self,
        value: &'v Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<&'v ConstructedValue, QueryError> {
        self.constructed(value, budget, location)?
            .map(|v| v.value)
            .ok_or_else(|| error(location, "native handles require an explicit scalar projection; computational values cannot be rendered as data"))
    }
    fn occurs(
        &self,
        value: &Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<bool, QueryError> {
        Ok(self
            .constructed(value, budget, location)?
            .is_none_or(|v| v.value.occurs()))
    }
    fn normalize_in_place(
        &self,
        value: &mut Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        if matches!(value.data, Data::Native(_))
            && let Some(data) = self.constructed(value, budget, location)?
        {
            value.data =
                Data::Constructed(RuntimeValue::Owned(data.materialize(budget, location)?));
        }
        Ok(())
    }
    fn normalize(
        &self,
        mut value: Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, QueryError> {
        self.normalize_in_place(&mut value, budget, location)?;
        Ok(value)
    }
    fn materialize(
        &self,
        value: Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<ConstructedValue, QueryError> {
        self.normalize(value, budget, location)?
            .materialize(budget, location)
    }
    fn native_record<'v>(
        &self,
        _value: &'v Self::Native,
    ) -> Option<&'v crate::NativeCapture<Self::Native>> {
        None
    }
    fn native_tuple<'v>(&self, _value: &'v Self::Native) -> Option<&'v [Self::Native]> {
        None
    }
    fn native_list<'v>(&self, _value: &'v Self::Native) -> Option<&'v [Self::Native]> {
        None
    }
    fn native_data<'v>(
        &self,
        value: &'v NativeView<Self::Native>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<&'v Self::Native, QueryError> {
        budget.charge(value.path.len(), location.clone())?;
        let data = value.native(
            |v| self.native_record(v),
            |v| self.native_tuple(v),
            |v| self.native_list(v),
        );
        Ok(data)
    }
    fn inspect_pattern(
        &mut self,
        _value: &Value<'a, Self::Native, Self::Kind>,
        _budget: &mut Budget,
        _location: &Location,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn check_native_context(
        &mut self,
        _value: &Self::Native,
        _expected: &DeclaredType<Self::Kind>,
        _budget: &mut Budget,
        _location: &Location,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
    fn apply(
        &mut self,
        expression: &'a PreparedValue<'a, Q>,
        values: Vec<Value<'a, Self::Native, Self::Kind>>,
        budget: &mut Budget,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        apply(expression, values, self, budget)
    }
    fn collection_hash(
        &mut self,
        value: &Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<u64, Self::Error> {
        Ok(super::identity::hash_admitted(
            self.scalar(value, budget, location)?,
            budget,
            location,
        )?)
    }
    fn collection_equal(
        &mut self,
        left: &Value<'a, Self::Native, Self::Kind>,
        right: &Value<'a, Self::Native, Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<bool, Self::Error> {
        Ok(super::key_equal(
            self.scalar(left, budget, location)?,
            self.scalar(right, budget, location)?,
            budget,
            location,
        )?)
    }
    fn finish_array(
        &mut self,
        values: Vec<Value<'a, Self::Native, Self::Kind>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        constructed_array(values, self, budget, location)
    }
    fn finish_record(
        &mut self,
        fields: Vec<(String, ResultValue<'a, Self, Q>)>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        constructed_record(fields, self, budget, location)
    }
    fn finish_dictionary(
        &mut self,
        entries: Vec<DictionaryEntry<'a, Self::Native, Self::Kind>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        constructed_dictionary(entries, self, budget, location)
    }
    fn finish_set(
        &mut self,
        values: Vec<Value<'a, Self::Native, Self::Kind>>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        constructed_set(values, self, budget, location)
    }
    fn retains_native_admission(&self) -> bool {
        false
    }
    fn current_input(
        &mut self,
        _budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        Err(QueryError::query(range.clone(), "constructed current input is unavailable").into())
    }
    fn predicate_subject(
        &mut self,
        _budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        Err(QueryError::query(
            range.clone(),
            "unnamed argument is only available in a predicate",
        )
        .into())
    }
    fn outer(
        &mut self,
        slot: usize,
        name: &str,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error>;
    #[expect(
        clippy::too_many_arguments,
        reason = "A native callback borrows the active request, explicit scope and argument continuations"
    )]
    fn native(
        &mut self,
        query: &'a Q,
        arguments: &'a [PreparedArgument<'a, Q>],
        locals: &mut Slots<'a, Self::Native, Self::Kind>,
        bodies: &'a [Body<'a, Q, Self::Kind>],
        depth: usize,
        function: Option<usize>,
        projected: bool,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error>;
    fn native_call(
        &mut self,
        _index: usize,
        _arguments: Vec<Value<'a, Self::Native, Self::Kind>>,
        _ranges: &[&std::ops::Range<usize>],
        _budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        Err(QueryError::query(
            range.clone(),
            "native function was not prepared in this request",
        )
        .into())
    }
    fn admit_native(
        &mut self,
        value: &Self::Native,
        expected: &DeclaredType<Self::Kind>,
        budget: &mut Budget,
        location: Location,
    ) -> Result<(), Self::Error>;
    fn sequence_items(
        &mut self,
        _value: NativeView<Self::Native>,
        _element: &DeclaredType<Self::Kind>,
        _budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<ResultValue<'a, Self, Q>>, Self::Error> {
        Err(error(location, "native positional sequences are unsupported").into())
    }
    fn finish_sequence(
        &mut self,
        values: Vec<Value<'a, Self::Native, Self::Kind>>,
        element: &DeclaredType<Self::Kind>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, Self::Native, Self::Kind>, Self::Error> {
        Ok(constructed_sequence(
            values, element, self, budget, location,
        )?)
    }
    fn native_contract(
        &mut self,
        _value: &Self::Native,
        _budget: &mut Budget,
        location: &Location,
    ) -> Result<DeclaredType<Self::Kind>, Self::Error> {
        Err(error(location, "loop state needs an established native contract").into())
    }
    fn subjects(
        &mut self,
        _value: NativeView<Self::Native>,
        _budget: &mut Budget,
        location: &Location,
    ) -> Result<Vec<Self::Native>, Self::Error> {
        Err(error(location, "in source needs an array or supported set").into())
    }
    fn check(
        &mut self,
        value: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError>;
    fn failure(&mut self, _calls: &[(usize, std::ops::Range<usize>)], _omitted: usize) {}
}

pub(crate) fn constructed_array<'a, Q, H: Host<'a, Q> + ?Sized>(
    values: Vec<Value<'a, H::Native, H::Kind>>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<Value<'a, H::Native, H::Kind>, H::Error> {
    let mut output = metadata_vec(values.len(), budget, location)?;
    for value in values {
        output.push(host.materialize(value, budget, location)?);
    }
    Ok(Value::constructed(RuntimeValue::Owned(
        ConstructedValue::Array(output),
    )))
}
pub(crate) fn constructed_record<'a, Q, H: Host<'a, Q> + ?Sized>(
    fields: Vec<(String, ResultValue<'a, H, Q>)>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let mut output = metadata_vec(fields.len(), budget, location)?;
    for (label, value) in fields {
        output.push((label, host.materialize(value, budget, location)?));
    }
    Ok(Value::constructed(RuntimeValue::Owned(
        ConstructedValue::Record(output),
    )))
}
pub(crate) fn constructed_dictionary<'a, Q, H: Host<'a, Q> + ?Sized>(
    entries: Vec<DictionaryEntry<'a, H::Native, H::Kind>>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<Value<'a, H::Native, H::Kind>, H::Error> {
    let mut output = metadata_vec(entries.len(), budget, location)?;
    for (key, value) in entries {
        output.push((
            host.materialize(key, budget, location)?,
            host.materialize(value, budget, location)?,
        ));
    }
    Ok(Value::constructed(RuntimeValue::Owned(
        super::finish_dictionary(output, budget, location)?,
    )))
}
pub(crate) fn constructed_set<'a, Q, H: Host<'a, Q> + ?Sized>(
    values: Vec<Value<'a, H::Native, H::Kind>>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<Value<'a, H::Native, H::Kind>, H::Error> {
    let mut output = metadata_vec(values.len(), budget, location)?;
    let mut kind = None;
    let mut hashes = HashIndex::default();
    for value in values {
        insert_set(
            &mut output,
            &mut hashes,
            &mut kind,
            host.materialize(value, budget, location)?,
            false,
            budget,
            location,
        )?;
    }
    super::check_set(kind, &output, budget, location)?;
    Ok(Value::constructed(RuntimeValue::Owned(
        ConstructedValue::Set {
            elements: output.into(),
            element_type: kind,
        },
    )))
}

struct Activation<'a, N, K> {
    function: Option<usize>,
    range: std::ops::Range<usize>,
    base: usize,
    locals: Vec<(&'a str, Value<'a, N, K>)>,
    names: &'a [&'a str],
    return_contract: Option<&'a DeclaredType<K>>,
}
type ResultValue<'a, H, Q> = Value<'a, <H as Host<'a, Q>>::Native, <H as Host<'a, Q>>::Kind>;
type ResultSource<'a, H, Q> =
    GeneratorSource<'a, <H as Host<'a, Q>>::Native, <H as Host<'a, Q>>::Kind>;

enum Pending<'a, Q, N, K> {
    SelectionSource(&'a PreparedValue<'a, Q>, usize),
    SelectionNext(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        ConstructedValue,
        Vec<bool>,
        usize,
    ),
    Eval(&'a PreparedValue<'a, Q>, usize, bool),
    Check(&'a PreparedValue<'a, Q>, usize),
    Apply(
        &'a PreparedValue<'a, Q>,
        usize,
        Vec<&'a PreparedValue<'a, Q>>,
        Vec<Value<'a, N, K>>,
    ),
    Initialize(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        bool,
        Vec<Value<'a, N, K>>,
    ),
    Loop(Loop<'a, Q, K>),
    Break(&'a PreparedValue<'a, Q>, usize),
    Next(&'a PreparedValue<'a, Q>, usize, Vec<Value<'a, N, K>>),
    Branch(&'a PreparedValue<'a, Q>, usize, usize, bool),
    Block(&'a PreparedValue<'a, Q>, usize, usize, bool),
    Match(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        bool,
        Option<Value<'a, N, K>>,
    ),
    Guard(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        bool,
        usize,
        Value<'a, N, K>,
    ),
    Require(&'a PreparedValue<'a, Q>, usize, bool),
    Default(&'a PreparedValue<'a, Q>, usize, bool),
    Fallback(&'a PreparedValue<'a, Q>, usize, Value<'a, N, K>),
    Call(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        bool,
        Vec<Value<'a, N, K>>,
    ),
    Invoke {
        expression: &'a PreparedValue<'a, Q>,
        function: usize,
        arguments: Vec<Value<'a, N, K>>,
        ranges: Vec<&'a std::ops::Range<usize>>,
        tail: bool,
        return_contract: Option<&'a DeclaredType<K>>,
    },
    AccumulatorItems(&'a PreparedValue<'a, Q>, usize),
    AccumulatorInitial(&'a PreparedValue<'a, Q>, usize, Value<'a, N, K>),
    AccumulatorStep(&'a PreparedValue<'a, Q>, usize, Accumulator<'a, N, K>),
    Set(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Vec<Value<'a, N, K>>,
        HashIndex,
    ),
    DictionaryKey(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Vec<DictionaryEntry<'a, N, K>>,
        HashIndex,
    ),
    DictionaryValue(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Vec<DictionaryEntry<'a, N, K>>,
        HashIndex,
        Value<'a, N, K>,
        u64,
    ),
    DeclaredAxes(&'a PreparedValue<'a, Q>, usize, usize, Vec<KeyAxis>),
    KeyedKey(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Option<Vec<KeyAxis>>,
        Vec<Located<KeyedEntry>>,
    ),
    KeyedValue(
        &'a PreparedValue<'a, Q>,
        usize,
        usize,
        Option<Vec<KeyAxis>>,
        Vec<Located<KeyedEntry>>,
        Vec<ValueKey>,
    ),
    Let(usize, usize),
    Return(usize),
    Predicate(&'a PreparedValue<'a, Q>, usize, usize),
    PredicateResult(&'a PreparedValue<'a, Q>, usize),
    Clauses(Rc<Comprehension<'a, Q, N, K>>, usize, usize),
    Source(Rc<Comprehension<'a, Q, N, K>>, usize, usize),
    Candidate(
        Rc<Comprehension<'a, Q, N, K>>,
        usize,
        usize,
        GeneratorSource<'a, N, K>,
        Vec<usize>,
        usize,
    ),
    Filter(Rc<Comprehension<'a, Q, N, K>>, usize, usize),
    Restore(usize),
    Head(Rc<Comprehension<'a, Q, N, K>>),
    KeyHead(Rc<Comprehension<'a, Q, N, K>>, Vec<ValueKey>),
    DictionaryHead(Rc<Comprehension<'a, Q, N, K>>, Value<'a, N, K>, u64),
    FinishComprehension(Rc<Comprehension<'a, Q, N, K>>),
    EmptyAxes(Rc<Comprehension<'a, Q, N, K>>, usize, Vec<KeyAxis>),
}
struct Accumulator<'a, N, K> {
    source: GeneratorSource<'a, N, K>,
    index: usize,
    snapshots: Vec<Value<'a, N, K>>,
}
struct Loop<'a, Q, K> {
    expression: &'a PreparedValue<'a, Q>,
    depth: usize,
    tail: bool,
    saved: usize,
    contracts: Vec<StateContract<'a, K>>,
}
enum StateContract<'a, K> {
    Borrowed(&'a DeclaredType<K>),
    Constructed(std::sync::Arc<DeclaredType<()>>),
    Native(DeclaredType<K>),
}

enum GeneratorSource<'a, N, K> {
    Constructed(RuntimeValue),
    Native(Vec<Value<'a, N, K>>),
}
impl<'a, N, K> GeneratorSource<'a, N, K> {
    fn len(&self) -> usize {
        match self {
            Self::Constructed(value) => match value.as_ref() {
                ConstructedValue::Array(values) => values.len(),
                ConstructedValue::Set { elements, .. } => elements.len(),
                ConstructedValue::KeyedArray { entries, .. } => entries.len(),
                _ => unreachable!("admitted generator source"),
            },
            Self::Native(values) => values.len(),
        }
    }
    fn child(
        &mut self,
        position: usize,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Value<'a, N, K>, QueryError> {
        match self {
            Self::Constructed(value) => Ok(Value::constructed(
                value
                    .share(budget, location)?
                    .child(position, budget, location)?,
            )),
            Self::Native(values) => values[position].share(budget, location),
        }
    }
}
struct Comprehension<'a, Q, N, K> {
    clauses: &'a [Located<PreparedGenerator<'a, Q>>],
    key: Option<&'a PreparedValue<'a, Q>>,
    head: &'a PreparedValue<'a, Q>,
    empty: Option<&'a [PreparedValue<'a, Q>]>,
    general_keys: bool,
    depth: usize,
    output: std::cell::RefCell<Output<'a, N, K>>,
}
enum Output<'a, N, K> {
    Array(Vec<Value<'a, N, K>>),
    Set(Vec<Value<'a, N, K>>, HashIndex),
    Keyed(Option<Vec<KeyAxis>>, Vec<Located<KeyedEntry>>),
    Dictionary(Vec<DictionaryEntry<'a, N, K>>, HashIndex),
}

pub(crate) fn execute<'a, Q, H: Host<'a, Q>>(
    entry: &'a PreparedValue<'a, Q>,
    names: &'a [&'a str],
    bodies: &'a [Body<'a, Q, H::Kind>],
    host: &mut H,
    budget: &mut Budget,
    limit: usize,
    offset: usize,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let location = Location::Query(entry.range.clone());
    budget.within_evaluation(0, location.clone(), |budget| {
        let mut pending = metadata_vec(1, budget, &location)?;
        pending.push(Pending::Eval(entry, 1, false));
        let mut frames = metadata_vec(1, budget, &location)?;
        frames.push(Activation {
            function: None,
            range: entry.range.clone(),
            base: 0,
            locals: Vec::new(),
            names,
            return_contract: None,
        });
        let mut result: Option<ResultValue<'a, H, Q>> = None;
        let outcome = (|| {
            while let Some(next) = pending.pop() {
                let frame_depth = frames.len();
                let frame = frames.last().expect("active expression scope");
                let location = match &next {
                    Pending::Eval(e, ..)
                    | Pending::Check(e, ..)
                    | Pending::Apply(e, ..)
                    | Pending::Initialize(e, ..)
                    | Pending::Break(e, ..)
                    | Pending::Next(e, ..)
                    | Pending::Block(e, ..)
                    | Pending::Match(e, ..)
                    | Pending::Guard(e, ..)
                    | Pending::Branch(e, ..)
                    | Pending::Require(e, ..)
                    | Pending::Default(e, ..)
                    | Pending::Fallback(e, ..)
                    | Pending::Call(e, ..)
                    | Pending::AccumulatorItems(e, ..)
                    | Pending::AccumulatorInitial(e, ..)
                    | Pending::AccumulatorStep(e, ..)
                    | Pending::Invoke { expression: e, .. }
                    | Pending::Set(e, ..)
                    | Pending::DictionaryKey(e, ..)
                    | Pending::DictionaryValue(e, ..)
                    | Pending::DeclaredAxes(e, ..)
                    | Pending::KeyedKey(e, ..)
                    | Pending::KeyedValue(e, ..) => Location::Query(e.range.clone()),
                    Pending::Loop(state) => Location::Query(state.expression.range.clone()),
                    _ => Location::Query(frame.range.clone()),
                };
                let depth = match &next {
                    Pending::SelectionSource(_, depth)
                    | Pending::SelectionNext(_, depth, ..)
                    | Pending::Eval(_, depth, _)
                    | Pending::Check(_, depth)
                    | Pending::Apply(_, depth, ..)
                    | Pending::Initialize(_, depth, ..)
                    | Pending::Break(_, depth)
                    | Pending::Next(_, depth, ..)
                    | Pending::Predicate(_, depth, ..)
                    | Pending::Block(_, depth, ..)
                    | Pending::Match(_, depth, ..)
                    | Pending::Guard(_, depth, ..)
                    | Pending::Branch(_, depth, ..)
                    | Pending::Require(_, depth, _)
                    | Pending::Default(_, depth, _)
                    | Pending::Fallback(_, depth, _)
                    | Pending::Call(_, depth, ..)
                    | Pending::AccumulatorItems(_, depth)
                    | Pending::AccumulatorInitial(_, depth, ..)
                    | Pending::AccumulatorStep(_, depth, ..)
                    | Pending::Set(_, depth, ..)
                    | Pending::DictionaryKey(_, depth, ..)
                    | Pending::DictionaryValue(_, depth, ..)
                    | Pending::DeclaredAxes(_, depth, ..)
                    | Pending::KeyedKey(_, depth, ..)
                    | Pending::KeyedValue(_, depth, ..) => *depth,
                    Pending::Clauses(c, index, _)
                    | Pending::Source(c, index, _)
                    | Pending::Candidate(c, index, ..)
                    | Pending::Filter(c, index, _) => c.depth + index + 1,
                    Pending::Head(c) | Pending::KeyHead(c, _) | Pending::DictionaryHead(c, ..) => {
                        c.depth + c.clauses.len()
                    }
                    Pending::FinishComprehension(c) | Pending::EmptyAxes(c, ..) => c.depth,
                    Pending::Loop(state) => state.depth,
                    Pending::Let(..)
                    | Pending::Return(_)
                    | Pending::Restore(_)
                    | Pending::PredicateResult(_, _)
                    | Pending::Invoke { .. } => 0,
                };
                // Native argument callbacks can reenter this runner. Dispatch them
                // outside the large pure-value step so that frame is not retained.
                if let Pending::Eval(expression, _, _) = &next
                    && let PreparedKind::NativeQuery { query, arguments }
                    | PreparedKind::NativeValue { query, arguments } = &expression.value
                {
                    budget.within_evaluation(
                        depth,
                        location.clone(),
                        |budget| -> Result<(), H::Error> {
                            budget.charge(1, location.clone())?;
                            push_metadata(
                                &mut pending,
                                Pending::Check(expression, depth),
                                budget,
                                &location,
                            )?;
                            let frame = frames.last_mut().expect("active expression scope");
                            result = Some(host.native(
                                query,
                                arguments,
                                &mut frame.locals,
                                bodies,
                                offset + frame_depth - 1,
                                frame.function,
                                matches!(expression.value, PreparedKind::NativeQuery { .. }),
                                budget,
                                &expression.range,
                            )?);
                            Ok(())
                        },
                    )?;
                    continue;
                }
                budget.within_evaluation(
                    depth,
                    location.clone(),
                    |budget| -> Result<(), H::Error> {
                        let frame = frames.last_mut().expect("active expression scope");
                        // Each push is admitted before vector growth. Stored pending work is
                        // bounded by spending; completed/tail activations leave no history.
                        macro_rules! push {
                            ($v:expr) => {
                                push_metadata(&mut pending, $v, budget, &location)?
                            };
                        }
                        macro_rules! owned {
                            ($v:expr) => {
                                Some(Value::constructed(RuntimeValue::Owned($v)))
                            };
                        }
                        match next {
                            Pending::Eval(expression, depth, tail) => {
                                budget.charge(1, location.clone())?;
                                push!(Pending::Check(expression, depth));
                                match &expression.value {
                                    PreparedKind::Literal(value) => {
                                        result =
                                            owned!(copy_constructed_value(value, budget, location)?)
                                    }
                                    PreparedKind::PredicateSubject => {
                                        result = Some(
                                            host.predicate_subject(budget, &expression.range)?,
                                        );
                                    }
                                    PreparedKind::CurrentInput => {
                                        result =
                                            Some(host.current_input(budget, &expression.range)?);
                                    }
                                    PreparedKind::Name(ReadSlot::Local(index)) => {
                                        result =
                                            Some(frame.locals[*index].1.share(budget, &location)?)
                                    }
                                    PreparedKind::Name(ReadSlot::Outer(index)) => {
                                        result = Some(host.outer(
                                            *index,
                                            frame.names[*index],
                                            budget,
                                            &expression.range,
                                        )?)
                                    }
                                    PreparedKind::NativeQuery { .. }
                                    | PreparedKind::NativeValue { .. } => unreachable!(
                                        "native callbacks are dispatched before pure-value steps"
                                    ),
                                    PreparedKind::Filter { collection, .. } => {
                                        push!(Pending::SelectionSource(expression, depth));
                                        push!(Pending::Eval(collection, depth + 1, false));
                                    }
                                    PreparedKind::SubjectScope { subject, .. } => {
                                        push!(Pending::Predicate(
                                            expression,
                                            depth,
                                            frame.locals.len()
                                        ));
                                        push!(Pending::Eval(subject, depth + 1, false));
                                    }
                                    PreparedKind::IdentitySet { .. } => {
                                        result =
                                            Some(host.apply(expression, Vec::new(), budget)?);
                                    }
                                    PreparedKind::Call { arguments, .. } => {
                                        let values =
                                            metadata_vec(arguments.len(), budget, &location)?;
                                        push!(Pending::Call(expression, depth, 0, tail, values));
                                        if let Some(first) = arguments.first() {
                                            push!(Pending::Eval(first, depth + 1, false));
                                        }
                                    }
                                    PreparedKind::Accumulate { items, .. } => {
                                        push!(Pending::AccumulatorItems(expression, depth));
                                        push!(Pending::Eval(items, depth + 1, false));
                                    }
                                    PreparedKind::Loop { .. }
                                    | PreparedKind::Break(_)
                                    | PreparedKind::Continue(_) => {
                                        enter_loop(
                                            expression,
                                            depth,
                                            tail,
                                            frame.base,
                                            &mut pending,
                                            budget,
                                            &location,
                                        )?;
                                    }
                                    PreparedKind::Block { .. } => {
                                        push!(Pending::Restore(frame.locals.len()));
                                        push!(Pending::Block(expression, depth, 0, tail));
                                    }
                                    PreparedKind::ValueMatch { subject, .. } => {
                                        push!(Pending::Match(expression, depth, 0, tail, None));
                                        push!(Pending::Eval(subject, depth + 1, false));
                                    }
                                    PreparedKind::Conditional {
                                        branches,
                                        otherwise,
                                    } => {
                                        if let Some((condition, _)) = branches.first() {
                                            push!(Pending::Branch(expression, depth, 0, tail));
                                            push!(Pending::Eval(condition, depth + 1, false));
                                        } else {
                                            push!(Pending::Eval(otherwise, depth + 1, tail));
                                        }
                                    }
                                    PreparedKind::Require { condition, .. } => {
                                        push!(Pending::Require(expression, depth, tail));
                                        push!(Pending::Eval(condition, depth + 1, false));
                                    }
                                    PreparedKind::Default { value, .. } => {
                                        push!(Pending::Default(expression, depth, tail));
                                        push!(Pending::Eval(value, depth + 1, false));
                                    }
                                    PreparedKind::Set(children) => {
                                        let values =
                                            admit_container(children.len(), budget, &location)?;
                                        if let Some(child) = children.first() {
                                            push!(Pending::Set(
                                                expression,
                                                depth,
                                                0,
                                                values,
                                                HashIndex::default()
                                            ));
                                            push!(Pending::Eval(child, depth + 1, false));
                                        } else {
                                            result =
                                                Some(host.finish_set(values, budget, &location)?);
                                        }
                                    }
                                    PreparedKind::Dictionary(_)
                                    | PreparedKind::DictionaryComprehension { .. } => {
                                        start_dictionary(
                                            expression,
                                            depth,
                                            frame.locals.len(),
                                            &mut pending,
                                            &mut result,
                                            host,
                                            budget,
                                        )?;
                                    }
                                    PreparedKind::KeyedArray { .. }
                                    | PreparedKind::KeyedComprehension { .. } => {
                                        if let Some(sources) = declared_axis_sources(expression) {
                                            let axes =
                                                admit_container(sources.len(), budget, &location)?;
                                            let source = sources.first().ok_or_else(|| {
                                                error(&location, "axis sources need nonzero rank")
                                            })?;
                                            push!(Pending::DeclaredAxes(
                                                expression, depth, 0, axes
                                            ));
                                            push!(Pending::Eval(source, depth + 1, false));
                                        } else {
                                            let axes = match &expression.value {
                                                PreparedKind::KeyedArray { axes, .. } => axes
                                                    .map(|axes| copy_axes(axes, budget, &location))
                                                    .transpose()?,
                                                _ => None,
                                            };
                                            start_keyed(
                                                expression,
                                                depth,
                                                frame.locals.len(),
                                                axes,
                                                &mut pending,
                                                &mut result,
                                                budget,
                                            )?;
                                        }
                                    }
                                    PreparedKind::Comprehension {
                                        constructor,
                                        clauses,
                                        head,
                                        ..
                                    } => {
                                        let output = match constructor {
                                            ValueConstructor::Array => Output::Array(
                                                admit_container(0, budget, &location)?,
                                            ),
                                            ValueConstructor::Set => Output::Set(
                                                admit_container(0, budget, &location)?,
                                                HashIndex::default(),
                                            ),
                                        };
                                        let c = Rc::new(Comprehension {
                                            clauses,
                                            key: None,
                                            head,
                                            empty: None,
                                            general_keys: false,
                                            depth,
                                            output: std::cell::RefCell::new(output),
                                        });
                                        push!(Pending::FinishComprehension(Rc::clone(&c)));
                                        push!(Pending::Clauses(c, 0, frame.locals.len()));
                                    }
                                    _ => {
                                        let children = operands(expression, budget, &location)?;
                                        let values =
                                            metadata_vec(children.len(), budget, &location)?;
                                        let first = children.first().copied();
                                        push!(Pending::Apply(expression, depth, children, values));
                                        if let Some(first) = first {
                                            push!(Pending::Eval(first, depth + 1, false));
                                        }
                                    }
                                }
                            }
                            Pending::Check(expression, _) => {
                                if let Some(Value {
                                    data: Data::Constructed(value),
                                    ..
                                }) = &result
                                {
                                    host.check(
                                        value.as_ref(),
                                        budget,
                                        &Location::Query(expression.range.clone()),
                                    )?;
                                }
                            }
                            Pending::Apply(expression, depth, children, mut values) => {
                                if !children.is_empty() {
                                    let value = result.take().expect("operand result");
                                    let shared = matches!(
                                        expression.value,
                                        PreparedKind::Queue { .. }
                                            | PreparedKind::PopFront(_)
                                            | PreparedKind::PushBackAll { .. }
                                            | PreparedKind::Insert { .. }
                                            | PreparedKind::Array(_)
                                            | PreparedKind::Record(_)
                                            | PreparedKind::With { .. }
                                            | PreparedKind::Tuple(_)
                                            | PreparedKind::TupleAccess { .. }
                                            | PreparedKind::Tally { .. }
                                            | PreparedKind::Windows { .. }
                                            | PreparedKind::RunsBy { .. }
                                            | PreparedKind::Sort { .. }
                                            | PreparedKind::GroupBy { .. }
                                            | PreparedKind::Align { .. }
                                            | PreparedKind::Lookup { .. }
                                            | PreparedKind::HasKey { .. }
                                            | PreparedKind::Binary {
                                                operator: ValueBinaryOperator::Equal
                                                    | ValueBinaryOperator::NotEqual
                                                    | ValueBinaryOperator::Less
                                                    | ValueBinaryOperator::LessEqual
                                                    | ValueBinaryOperator::Greater
                                                    | ValueBinaryOperator::GreaterEqual,
                                                ..
                                            }
                                            | PreparedKind::Binary {
                                                operator: ValueBinaryOperator::In,
                                                ..
                                            }
                                            | PreparedKind::Field { .. }
                                            | PreparedKind::ArrayAccess { .. }
                                            | PreparedKind::Occurs(_)
                                            | PreparedKind::Absent(_)
                                            | PreparedKind::Reduce { .. }
                                            | PreparedKind::Binary {
                                                operator: ValueBinaryOperator::Contains
                                                    | ValueBinaryOperator::StartsWith
                                                    | ValueBinaryOperator::EndsWith,
                                                ..
                                            }
                                            | PreparedKind::Extremum { .. }
                                    );
                                    let value = if values.is_empty()
                                        && matches!(expression.value, PreparedKind::Range { .. })
                                    {
                                        Value::constructed(RuntimeValue::Owned(
                                            ConstructedValue::Integer(integer(
                                                value.materialize_with(host, budget, &location)?,
                                                &location,
                                            )?),
                                        ))
                                    } else {
                                        value
                                    };
                                    // Constructed operands were admitted when introduced. Keep
                                    // native lookup operands on their existing materialization path.
                                    let shared = shared
                                        || (matches!(
                                            expression.value,
                                            PreparedKind::Lookup { .. }
                                                | PreparedKind::HasKey { .. }
                                        ) && matches!(value.data, Data::Constructed(_)));
                                    values.push(if shared {
                                        value
                                    } else {
                                        Value::constructed(RuntimeValue::Owned(
                                            value.materialize_with(host, budget, &location)?,
                                        ))
                                    });
                                }
                                if values.len() == 1
                                    && let PreparedKind::Binary {
                                        operator:
                                            operator @ (ValueBinaryOperator::And
                                            | ValueBinaryOperator::Or),
                                        ..
                                    } = expression.value
                                {
                                    let a = match values[0].scalar(&location)?.data() {
                                        ConstructedValue::Boolean(a) => *a,
                                        _ => {
                                            return Err(error(
                                                &location,
                                                "condition needs a fixed Boolean",
                                            )
                                            .into());
                                        }
                                    };
                                    if (operator == ValueBinaryOperator::And && !a)
                                        || (operator == ValueBinaryOperator::Or && a)
                                    {
                                        result = owned!(scalar(
                                            ConstructedValue::Boolean(a),
                                            budget,
                                            &location
                                        )?);
                                        return Ok(());
                                    }
                                }
                                if values.len() < children.len() {
                                    let next = children[values.len()];
                                    push!(Pending::Apply(expression, depth, children, values));
                                    push!(Pending::Eval(next, depth + 1, false));
                                } else {
                                    result = Some(host.apply(expression, values, budget)?);
                                }
                            }
                            next @ (Pending::Initialize(..)
                            | Pending::Loop(_)
                            | Pending::Break(..)
                            | Pending::Next(..)) => {
                                loop_step(
                                    next,
                                    &mut pending,
                                    frame,
                                    &mut result,
                                    host,
                                    budget,
                                    &location,
                                )?;
                            }
                            Pending::Branch(expression, depth, index, tail) => {
                                let PreparedKind::Conditional {
                                    branches,
                                    otherwise,
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if boolean(
                                    result
                                        .take()
                                        .expect("condition")
                                        .materialize_with(host, budget, &location)?,
                                    &Location::Query(branches[index].0.range.clone()),
                                )? {
                                    push!(Pending::Eval(&branches[index].1, depth + 1, tail));
                                } else if let Some((condition, _)) = branches.get(index + 1) {
                                    push!(Pending::Branch(expression, depth, index + 1, tail));
                                    push!(Pending::Eval(condition, depth + 1, false));
                                } else {
                                    push!(Pending::Eval(otherwise, depth + 1, tail));
                                }
                            }
                            Pending::Require(expression, depth, tail) => {
                                let PreparedKind::Require {
                                    condition,
                                    value,
                                    message,
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if !boolean(
                                    result
                                        .take()
                                        .expect("require condition")
                                        .materialize_with(host, budget, &location)?,
                                    &Location::Query(condition.range.clone()),
                                )? {
                                    admit_diagnostic("", &message.value, budget, &location)?;
                                    return Err(error(&location, message.value.clone()).into());
                                }
                                push!(Pending::Eval(value, depth + 1, tail));
                            }
                            next @ (Pending::Default(..) | Pending::Fallback(..)) => {
                                default_step(
                                    next,
                                    &mut pending,
                                    &mut result,
                                    host,
                                    budget,
                                    &location,
                                )?;
                            }
                            Pending::Block(expression, depth, index, tail) => {
                                let PreparedKind::Block { bindings, value } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if index > 0 {
                                    let pattern = bindings[index - 1].0;
                                    let value = result.take().expect("block let result");
                                    host.inspect_pattern(
                                        &value,
                                        budget,
                                        &Location::Query(pattern.range.clone()),
                                    )?;
                                    let bound = super::pattern::bind(
                                        super::pattern::PatternRef::Pattern(pattern),
                                        value,
                                        host,
                                        budget,
                                    )?
                                    .ok_or_else(|| {
                                        error(
                                            &Location::Query(pattern.range.clone()),
                                            "binding pattern did not match",
                                        )
                                    })?;
                                    for binding in bound {
                                        push_metadata(
                                            &mut frame.locals,
                                            binding,
                                            budget,
                                            &location,
                                        )?;
                                    }
                                }
                                if let Some((_, value)) = bindings.get(index) {
                                    push!(Pending::Block(expression, depth, index + 1, tail));
                                    push!(Pending::Eval(value, depth + 1, false));
                                } else {
                                    push!(Pending::Eval(value, depth + 1, tail));
                                }
                            }
                            Pending::Match(expression, depth, mut index, tail, subject) => {
                                let PreparedKind::ValueMatch { arms, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let mut subject = match subject {
                                    Some(subject) => subject,
                                    None => {
                                        let subject = result.take().expect("match subject");
                                        host.inspect_pattern(&subject, budget, &location)?;
                                        subject
                                    }
                                };
                                loop {
                                    let arm = &arms[index];
                                    let candidate = subject.share(
                                        budget,
                                        &Location::Query(arm.pattern.range.clone()),
                                    )?;
                                    if let Some(bound) = super::pattern::bind(
                                        super::pattern::PatternRef::Pattern(arm.pattern),
                                        candidate,
                                        host,
                                        budget,
                                    )? {
                                        let saved = frame.locals.len();
                                        for binding in bound {
                                            push_metadata(
                                                &mut frame.locals,
                                                binding,
                                                budget,
                                                &location,
                                            )?;
                                        }
                                        if let Some(guard) = &arm.guard {
                                            push!(Pending::Guard(
                                                expression, depth, index, tail, saved, subject
                                            ));
                                            push!(Pending::Eval(guard, depth + 1, false));
                                        } else {
                                            drop(subject);
                                            push!(Pending::Restore(saved));
                                            push!(Pending::Eval(&arm.value, depth + 1, tail));
                                        }
                                        break;
                                    }
                                    index += 1;
                                }
                            }
                            Pending::Guard(expression, depth, index, tail, saved, subject) => {
                                let PreparedKind::ValueMatch { arms, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let arm = &arms[index];
                                let guard = arm.guard.as_ref().expect("guard");
                                budget.charge(1, Location::Query(guard.range.clone()))?;
                                if boolean(
                                    result
                                        .take()
                                        .expect("guard result")
                                        .materialize_with(host, budget, &location)?,
                                    &Location::Query(guard.range.clone()),
                                )? {
                                    drop(subject);
                                    push!(Pending::Restore(saved));
                                    push!(Pending::Eval(&arm.value, depth + 1, tail));
                                } else {
                                    frame.locals.truncate(saved);
                                    push!(Pending::Match(
                                        expression,
                                        depth,
                                        index + 1,
                                        tail,
                                        Some(subject)
                                    ));
                                }
                            }
                            Pending::Call(expression, depth, index, tail, mut arguments) => {
                                let PreparedKind::Call {
                                    function,
                                    arguments: sources,
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if !sources.is_empty() {
                                    arguments.push(result.take().expect("call argument"));
                                }
                                if let Some(next) = sources.get(index + 1) {
                                    push!(Pending::Call(
                                        expression,
                                        depth,
                                        index + 1,
                                        tail,
                                        arguments
                                    ));
                                    push!(Pending::Eval(next, depth + 1, false));
                                    return Ok(());
                                }
                                let mut ranges = metadata_vec(sources.len(), budget, &location)?;
                                ranges.extend(sources.iter().map(|source| &source.range));
                                push!(Pending::Invoke {
                                    expression,
                                    function: *function,
                                    arguments,
                                    ranges,
                                    tail,
                                    return_contract: None
                                });
                            }
                            Pending::Invoke {
                                expression,
                                function,
                                mut arguments,
                                ranges,
                                tail,
                                return_contract,
                            } => {
                                budget.charge(1, location.clone())?;
                                let call_range = match &expression.value {
                                    PreparedKind::Accumulate { step, .. } => &step.range,
                                    _ => &expression.range,
                                };
                                if function >= bodies.len() {
                                    result = Some(host.native_call(
                                        function - bodies.len(),
                                        arguments,
                                        &ranges,
                                        budget,
                                        call_range,
                                    )?);
                                    return Ok(());
                                }
                                let body = bodies.get(function).ok_or_else(|| {
                                    error(&location, "function was not prepared in this request")
                                })?;
                                // All arguments still refer to old slots until admission completes.
                                for ((argument, parameter), source) in arguments
                                    .iter_mut()
                                    .zip(body.declaration.parameters.iter())
                                    .zip(ranges)
                                {
                                    admit(
                                        argument,
                                        parameter.declaration,
                                        host,
                                        budget,
                                        &Location::Query(source.clone()),
                                    )?;
                                }
                                let mut locals = Vec::new();
                                for (parameter, value) in
                                    body.declaration.parameters.iter().zip(arguments)
                                {
                                    let bound = super::pattern::bind(
                                        parameter.pattern,
                                        value,
                                        host,
                                        budget,
                                    )?
                                    .ok_or_else(|| {
                                        error(
                                            &Location::Query(parameter.pattern.range().clone()),
                                            "parameter pattern did not match",
                                        )
                                    })?;
                                    for binding in bound {
                                        push_metadata(&mut locals, binding, budget, &location)?;
                                    }
                                }
                                let reuse = if tail {
                                    match frame.function {
                                        Some(caller) => crate::runtime_types::same_declaration(
                                            &bodies[caller].declaration.result.value,
                                            &body.declaration.result.value,
                                            budget,
                                            &location,
                                        )?,
                                        None => false,
                                    }
                                } else {
                                    false
                                };
                                if reuse {
                                    pending.truncate(frame.base);
                                    // Arguments retain any old-slot aliases until admission has
                                    // finished; installing them then releases dead locals.
                                    frame.locals = locals;
                                    frame.function = Some(function);
                                    frame.range = call_range.clone();
                                } else {
                                    if offset + frames.len() > limit {
                                        return Err(error(
                                            &location,
                                            "active function depth limit exceeded",
                                        )
                                        .into());
                                    }
                                    let base = pending.len();
                                    push_metadata(
                                        &mut frames,
                                        Activation {
                                            function: Some(function),
                                            range: call_range.clone(),
                                            base,
                                            locals,
                                            names: &[],
                                            return_contract,
                                        },
                                        budget,
                                        &location,
                                    )?;
                                }
                                push!(Pending::Return(function));
                                push!(Pending::Let(function, 0));
                            }
                            Pending::AccumulatorItems(expression, depth) => {
                                let PreparedKind::Accumulate { initial, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let items = result.take().expect("accumulator items");
                                push!(Pending::AccumulatorInitial(expression, depth, items));
                                push!(Pending::Eval(initial, depth + 1, false));
                            }
                            Pending::AccumulatorInitial(expression, depth, items_value) => {
                                let PreparedKind::Accumulate {
                                    items,
                                    initial,
                                    function,
                                    ..
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let declaration = &bodies[*function].declaration;
                                let mut parameters = declaration.parameters.iter();
                                let state = parameters.next().expect("step state").declaration;
                                let element = parameters.next().expect("step item").declaration;
                                let mut initial_value = result.take().expect("accumulator initial");
                                admit(
                                    &mut initial_value,
                                    state,
                                    host,
                                    budget,
                                    &Location::Query(initial.range.clone()),
                                )?;
                                let source_location = Location::Query(items.range.clone());
                                let source = accumulator_source(
                                    items_value,
                                    element,
                                    host,
                                    budget,
                                    &source_location,
                                )?;
                                result = Some(initial_value);
                                push!(Pending::AccumulatorStep(
                                    expression,
                                    depth,
                                    Accumulator {
                                        source,
                                        index: 0,
                                        snapshots: Vec::new()
                                    }
                                ));
                            }
                            Pending::AccumulatorStep(expression, depth, mut accumulator) => {
                                let PreparedKind::Accumulate {
                                    items,
                                    function,
                                    step,
                                    scan,
                                    ..
                                } = &expression.value
                                else {
                                    unreachable!()
                                };
                                budget.charge(1, location.clone())?;
                                let state_contract = bodies[*function]
                                    .declaration
                                    .parameters
                                    .iter()
                                    .next()
                                    .expect("step state")
                                    .declaration;
                                let mut state = result.take().expect("accumulator state");
                                if *scan && accumulator.index > 0 {
                                    budget.charge_results(1, location.clone())?;
                                    let snapshot = state.share(budget, &location)?;
                                    push_metadata(
                                        &mut accumulator.snapshots,
                                        snapshot,
                                        budget,
                                        &location,
                                    )?;
                                }
                                if accumulator.index == accumulator.source.len() {
                                    result = Some(if *scan {
                                        host.finish_sequence(
                                            accumulator.snapshots,
                                            state_contract,
                                            budget,
                                            &location,
                                        )?
                                    } else {
                                        state
                                    });
                                } else {
                                    let item = accumulator.source.child(
                                        accumulator.index,
                                        budget,
                                        &Location::Query(items.range.clone()),
                                    )?;
                                    accumulator.index += 1;
                                    let mut arguments = metadata_vec(2, budget, &location)?;
                                    arguments.push(state);
                                    arguments.push(item);
                                    let mut ranges = metadata_vec(2, budget, &location)?;
                                    ranges.push(&step.range);
                                    ranges.push(&items.range);
                                    push!(Pending::AccumulatorStep(expression, depth, accumulator));
                                    push!(Pending::Invoke {
                                        expression,
                                        function: *function,
                                        arguments,
                                        ranges,
                                        tail: false,
                                        return_contract: Some(state_contract)
                                    });
                                }
                            }
                            Pending::Set(expression, depth, index, mut values, mut hashes) => {
                                let value = result.take().expect("set child");
                                let mut duplicate = false;
                                let hash = host.collection_hash(&value, budget, &location)?;
                                for &index in hashes.candidates(hash) {
                                    if host.collection_equal(
                                        &values[index],
                                        &value,
                                        budget,
                                        &location,
                                    )? {
                                        duplicate = true;
                                        break;
                                    }
                                }
                                if !duplicate {
                                    hashes.insert(hash, values.len(), budget, &location)?;
                                    push_metadata(&mut values, value, budget, &location)?;
                                }
                                let PreparedKind::Set(children) = &expression.value else {
                                    unreachable!()
                                };
                                if let Some(child) = children.get(index + 1) {
                                    push!(Pending::Set(
                                        expression,
                                        depth,
                                        index + 1,
                                        values,
                                        hashes
                                    ));
                                    push!(Pending::Eval(child, depth + 1, false));
                                } else {
                                    result = Some(host.finish_set(values, budget, &location)?);
                                }
                            }
                            next @ (Pending::DictionaryKey(..) | Pending::DictionaryValue(..)) => {
                                dictionary_step(next, &mut pending, &mut result, host, budget)?;
                            }
                            Pending::DeclaredAxes(expression, depth, index, mut axes) => {
                                let sources =
                                    declared_axis_sources(expression).expect("declared sources");
                                let source_location = Location::Query(sources[index].range.clone());
                                axes.push(source_axis(
                                    result.take().expect("axis source").materialize_with(
                                        host,
                                        budget,
                                        &source_location,
                                    )?,
                                    budget,
                                    &source_location,
                                )?);
                                if let Some(source) = sources.get(index + 1) {
                                    push!(Pending::DeclaredAxes(
                                        expression,
                                        depth,
                                        index + 1,
                                        axes
                                    ));
                                    push!(Pending::Eval(source, depth + 1, false));
                                } else {
                                    start_keyed(
                                        expression,
                                        depth,
                                        frame.locals.len(),
                                        Some(axes),
                                        &mut pending,
                                        &mut result,
                                        budget,
                                    )?;
                                }
                            }
                            Pending::KeyedKey(expression, depth, index, axes, values) => {
                                let PreparedKind::KeyedArray { entries, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let key = result
                                    .take()
                                    .expect("literal key")
                                    .materialize_with(host, budget, &location)?;
                                let keys = value_keys(
                                    key,
                                    budget,
                                    &Location::Query(entries[index].0.range.clone()),
                                )?;
                                push!(Pending::KeyedValue(
                                    expression, depth, index, axes, values, keys
                                ));
                                push!(Pending::Eval(&entries[index].1, depth + 1, false));
                            }
                            Pending::KeyedValue(
                                expression,
                                depth,
                                index,
                                axes,
                                mut values,
                                keys,
                            ) => {
                                let PreparedKind::KeyedArray { entries, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                let value = result
                                    .take()
                                    .expect("literal cell")
                                    .materialize_with(host, budget, &location)?;
                                values.push(Located::new(
                                    KeyedEntry { key: keys, value },
                                    entries[index].0.range.clone(),
                                ));
                                if let Some((key, _)) = entries.get(index + 1) {
                                    push!(Pending::KeyedKey(
                                        expression,
                                        depth,
                                        index + 1,
                                        axes,
                                        values
                                    ));
                                    push!(Pending::Eval(key, depth + 1, false));
                                } else {
                                    result = owned!(finish_keyed(axes, values, budget, &location)?);
                                }
                            }
                            Pending::Let(function, index) => {
                                let body = &bodies[function];
                                if index > 0 {
                                    push_metadata(
                                        &mut frame.locals,
                                        (
                                            body.bindings[index - 1].0,
                                            result.take().expect("local let"),
                                        ),
                                        budget,
                                        &location,
                                    )?;
                                }
                                if let Some((_, expression)) = body.bindings.get(index) {
                                    push!(Pending::Let(function, index + 1));
                                    push!(Pending::Eval(expression, 1, false));
                                } else {
                                    push!(Pending::Eval(&body.value, 1, true));
                                }
                            }
                            Pending::Return(function) => {
                                let mut value = result.take().expect("function result");
                                admit(
                                    &mut value,
                                    &bodies[function].declaration.result.value,
                                    host,
                                    budget,
                                    &Location::Query(bodies[function].value.range.clone()),
                                )?;
                                if let Some(expected) = frame.return_contract {
                                    admit(
                                        &mut value,
                                        expected,
                                        host,
                                        budget,
                                        &Location::Query(bodies[function].value.range.clone()),
                                    )?;
                                }
                                frames.pop();
                                result = Some(value);
                            }
                            Pending::Clauses(c, index, saved) => {
                                if let Some(clause) = c.clauses.get(index) {
                                    budget.charge(1, Location::Query(clause.range.clone()))?;
                                    push!(Pending::Source(Rc::clone(&c), index, saved));
                                    let source = match &clause.value {
                                        PreparedGenerator::Bind { value, .. } => value,
                                        PreparedGenerator::In { source, .. } => source,
                                    };
                                    push!(Pending::Eval(source, c.depth + index + 2, false));
                                } else {
                                    push!(Pending::Head(Rc::clone(&c)));
                                    push!(Pending::Eval(
                                        c.key.unwrap_or(c.head),
                                        c.depth + index + 1,
                                        false
                                    ));
                                }
                            }
                            Pending::Source(c, index, saved) => {
                                let value = result.take().expect("generator source");
                                let clause = &c.clauses[index];
                                let cl = Location::Query(clause.range.clone());
                                match &clause.value {
                                    PreparedGenerator::Bind { binder, filter, .. } => {
                                        budget.charge(1, cl.clone())?;
                                        if !install_pattern(
                                            &mut frame.locals,
                                            binder,
                                            value,
                                            host,
                                            budget,
                                        )? {
                                            let location = Location::Query(binder.range.clone());
                                            return Err(error(
                                                &location,
                                                "generator pattern does not match its value",
                                            )
                                            .into());
                                        }
                                        push!(Pending::Restore(saved));
                                        if let Some(filter) = filter {
                                            push!(Pending::Filter(
                                                Rc::clone(&c),
                                                index,
                                                frame.locals.len()
                                            ));
                                            push!(Pending::Eval(
                                                filter,
                                                c.depth + index + 2,
                                                false
                                            ));
                                        } else {
                                            push!(Pending::Clauses(
                                                c,
                                                index + 1,
                                                frame.locals.len()
                                            ));
                                        }
                                    }
                                    PreparedGenerator::In {
                                        binders,
                                        positional,
                                        ..
                                    } => {
                                        let value = generator_source(
                                            value,
                                            *positional,
                                            host,
                                            budget,
                                            &cl,
                                        )?;
                                        let len = value.len();
                                        let mut count = 1usize;
                                        for _ in binders {
                                            budget.charge(1, cl.clone())?;
                                            count = count.checked_mul(len).ok_or_else(|| {
                                                error(&cl, "grouped generator cardinality overflow")
                                            })?;
                                        }
                                        budget.charge(count, cl.clone())?;
                                        let mut positions =
                                            metadata_vec(binders.len(), budget, &cl)?;
                                        positions.resize(binders.len(), 0);
                                        push!(Pending::Candidate(
                                            c, index, saved, value, positions, count
                                        ));
                                    }
                                }
                            }
                            Pending::Candidate(
                                c,
                                index,
                                saved,
                                mut source,
                                mut positions,
                                count,
                            ) => {
                                frame.locals.truncate(saved);
                                if count == 0 {
                                    return Ok(());
                                }
                                let clause = &c.clauses[index];
                                let PreparedGenerator::In { filter, .. } = &clause.value else {
                                    unreachable!()
                                };
                                let cl = Location::Query(clause.range.clone());
                                let len = source.len();
                                let matched = install_candidate(
                                    &mut frame.locals,
                                    &clause.value,
                                    &mut source,
                                    &positions,
                                    host,
                                    budget,
                                    &cl,
                                )?;
                                for position in positions.iter_mut().rev() {
                                    budget.charge(1, cl.clone())?;
                                    *position += 1;
                                    if *position < len {
                                        break;
                                    }
                                    *position = 0;
                                }
                                push!(Pending::Candidate(
                                    Rc::clone(&c),
                                    index,
                                    saved,
                                    source,
                                    positions,
                                    count - 1
                                ));
                                if !matched {
                                    frame.locals.truncate(saved);
                                    return Ok(());
                                }
                                if let Some(filter) = filter {
                                    push!(Pending::Filter(
                                        Rc::clone(&c),
                                        index,
                                        frame.locals.len()
                                    ));
                                    push!(Pending::Eval(filter, c.depth + index + 2, false));
                                } else {
                                    push!(Pending::Clauses(c, index + 1, frame.locals.len()));
                                }
                            }
                            Pending::Filter(c, index, saved) => {
                                let filter = match &c.clauses[index].value {
                                    PreparedGenerator::Bind { filter, .. }
                                    | PreparedGenerator::In { filter, .. } => {
                                        filter.as_ref().expect("generator filter")
                                    }
                                };
                                if boolean(
                                    result
                                        .take()
                                        .expect("filter")
                                        .materialize_with(host, budget, &location)?,
                                    &Location::Query(filter.range.clone()),
                                )? {
                                    push!(Pending::Clauses(c, index + 1, saved));
                                }
                            }
                            Pending::SelectionSource(expression, depth) => {
                                let collection = result
                                    .take()
                                    .expect("filter source")
                                    .materialize_with(host, budget, &location)?;
                                let len = super::collection_len(&collection, false, &location)?;
                                budget.check_items(len, location.clone())?;
                                let retained = metadata_vec(len, budget, &location)?;
                                push!(Pending::SelectionNext(
                                    expression,
                                    depth,
                                    0,
                                    collection,
                                    retained,
                                    frame.locals.len()
                                ));
                            }
                            Pending::SelectionNext(
                                expression,
                                depth,
                                index,
                                collection,
                                mut retained,
                                saved,
                            ) => {
                                let PreparedKind::Filter { argument, body, .. } = &expression.value
                                else {
                                    unreachable!()
                                };
                                if index > 0 {
                                    frame.locals.truncate(saved);
                                    retained.push(boolean(
                                        result
                                            .take()
                                            .expect("filter predicate")
                                            .materialize_with(host, budget, &location)?,
                                        &Location::Query(body.range.clone()),
                                    )?);
                                }
                                if index == super::collection_len(&collection, false, &location)? {
                                    result = Some(Value::constructed(RuntimeValue::Owned(
                                        super::retain_collection(
                                            collection, &retained, budget, &location,
                                        )?,
                                    )));
                                } else {
                                    let subject = Value::constructed(RuntimeValue::Owned(
                                        super::collection_subject(
                                            &collection,
                                            index,
                                            budget,
                                            &location,
                                        )?,
                                    ));
                                    install_predicate(
                                        &mut frame.locals,
                                        subject,
                                        *argument,
                                        host,
                                        budget,
                                        &location,
                                    )?;
                                    push!(Pending::SelectionNext(
                                        expression,
                                        depth,
                                        index + 1,
                                        collection,
                                        retained,
                                        saved
                                    ));
                                    push!(Pending::Eval(body, depth + 1, false));
                                }
                            }
                            Pending::Predicate(expression, depth, saved) => {
                                let PreparedKind::SubjectScope { argument, body, .. } =
                                    &expression.value
                                else {
                                    unreachable!()
                                };
                                install_predicate(
                                    &mut frame.locals,
                                    result.take().expect("predicate subject"),
                                    *argument,
                                    host,
                                    budget,
                                    &location,
                                )?;
                                push!(Pending::PredicateResult(expression, saved));
                                push!(Pending::Eval(body, depth + 1, false));
                            }
                            Pending::PredicateResult(expression, saved) => {
                                frame.locals.truncate(saved);
                                if matches!(
                                    expression.value,
                                    PreparedKind::SubjectScope {
                                        predicate: true,
                                        ..
                                    }
                                ) {
                                    result = Some(predicate_result(
                                        result.take().expect("predicate result"),
                                        host,
                                        budget,
                                        &Location::Query(expression.range.clone()),
                                    )?);
                                }
                            }
                            Pending::Restore(saved) => frame.locals.truncate(saved),
                            Pending::Head(c) => {
                                let value = result.take().expect("head or key");
                                if let Some(key) = c.key {
                                    if let Output::Dictionary(entries, hashes) = &*c.output.borrow()
                                    {
                                        let location = Location::Query(key.range.clone());
                                        let hash =
                                            host.collection_hash(&value, budget, &location)?;
                                        for &index in hashes.candidates(hash) {
                                            if host.collection_equal(
                                                &entries[index].0,
                                                &value,
                                                budget,
                                                &location,
                                            )? {
                                                return Err(error(
                                                    &location,
                                                    "duplicate dictionary key",
                                                )
                                                .into());
                                            }
                                        }
                                        push!(Pending::DictionaryHead(Rc::clone(&c), value, hash));
                                        push!(Pending::Eval(
                                            c.head,
                                            c.depth + c.clauses.len() + 1,
                                            false
                                        ));
                                        return Ok(());
                                    }
                                    let keys = value_keys(
                                        host.materialize(value, budget, &location)?,
                                        budget,
                                        &Location::Query(key.range.clone()),
                                    )?;
                                    push!(Pending::KeyHead(Rc::clone(&c), keys));
                                    push!(Pending::Eval(
                                        c.head,
                                        c.depth + c.clauses.len() + 1,
                                        false
                                    ));
                                } else if let Output::Set(elements, hashes) =
                                    &mut *c.output.borrow_mut()
                                {
                                    let mut duplicate = false;
                                    let hash = host.collection_hash(&value, budget, &location)?;
                                    for &index in hashes.candidates(hash) {
                                        if host.collection_equal(
                                            &elements[index],
                                            &value,
                                            budget,
                                            &location,
                                        )? {
                                            duplicate = true;
                                            break;
                                        }
                                    }
                                    if !duplicate {
                                        hashes.insert(hash, elements.len(), budget, &location)?;
                                        budget.charge_results(1, location.clone())?;
                                        push_metadata(elements, value, budget, &location)?;
                                    }
                                } else if let Output::Array(elements) = &mut *c.output.borrow_mut()
                                {
                                    budget.check_items(
                                        sum(elements.len(), 1, &location)?,
                                        location.clone(),
                                    )?;
                                    budget.charge_results(1, location.clone())?;
                                    push_metadata(elements, value, budget, &location)?;
                                } else {
                                    unreachable!("positional or set comprehension");
                                }
                            }
                            Pending::KeyHead(c, key) => {
                                let value = result
                                    .take()
                                    .expect("keyed head")
                                    .materialize_with(host, budget, &location)?;
                                append(&c, value, Some(key), budget)?;
                            }
                            Pending::DictionaryHead(c, key, hash) => {
                                let value = result.take().expect("dictionary head");
                                let Output::Dictionary(entries, hashes) =
                                    &mut *c.output.borrow_mut()
                                else {
                                    unreachable!("dictionary output")
                                };
                                budget.check_items(
                                    sum(entries.len(), 1, &location)?,
                                    location.clone(),
                                )?;
                                budget.charge_results(1, location.clone())?;
                                hashes.insert(hash, entries.len(), budget, &location)?;
                                push_metadata(entries, (key, value), budget, &location)?;
                            }
                            Pending::FinishComprehension(c) => {
                                let output = std::mem::replace(
                                    &mut *c.output.borrow_mut(),
                                    Output::Array(Vec::new()),
                                );
                                let value = match output {
                                    Output::Array(values) => {
                                        result =
                                            Some(host.finish_array(values, budget, &location)?);
                                        return Ok(());
                                    }
                                    Output::Dictionary(entries, _) => {
                                        if entries.is_empty() && c.general_keys {
                                            result = owned!(ConstructedValue::Dictionary(
                                                Vec::new().into()
                                            ));
                                            return Ok(());
                                        }
                                        result = Some(
                                            host.finish_dictionary(entries, budget, &location)?,
                                        );
                                        return Ok(());
                                    }
                                    Output::Set(elements, _) => {
                                        result =
                                            Some(host.finish_set(elements, budget, &location)?);
                                        return Ok(());
                                    }
                                    Output::Keyed(None, entries) if entries.is_empty() => {
                                        *c.output.borrow_mut() = Output::Keyed(None, entries);
                                        let sources = c.empty.ok_or_else(|| {
                                            error(&location, "unestablished empty keyed axes")
                                        })?;
                                        let axes =
                                            admit_container(sources.len(), budget, &location)?;
                                        if let Some(source) = sources.first() {
                                            push!(Pending::EmptyAxes(Rc::clone(&c), 0, axes));
                                            push!(Pending::Eval(source, c.depth + 1, false));
                                            return Ok(());
                                        }
                                        finish_keyed(Some(axes), Vec::new(), budget, &location)?
                                    }
                                    Output::Keyed(axes, entries) => {
                                        finish_keyed(axes, entries, budget, &location)?
                                    }
                                };
                                result = owned!(value);
                            }
                            Pending::EmptyAxes(c, index, mut axes) => {
                                let sources = c.empty.expect("empty sources");
                                axes.push(source_axis(
                                    result
                                        .take()
                                        .expect("axis source")
                                        .materialize_with(host, budget, &location)?,
                                    budget,
                                    &Location::Query(sources[index].range.clone()),
                                )?);
                                if let Some(source) = sources.get(index + 1) {
                                    push!(Pending::EmptyAxes(Rc::clone(&c), index + 1, axes));
                                    push!(Pending::Eval(source, c.depth + 1, false));
                                } else {
                                    result = owned!(finish_keyed(
                                        Some(axes),
                                        Vec::new(),
                                        budget,
                                        &location
                                    )?);
                                }
                            }
                        }
                        Ok(())
                    },
                )?;
            }
            Ok(result.expect("entry result"))
        })();
        if outcome.is_err() {
            let calls = frames
                .iter()
                .filter_map(|frame| frame.function.map(|id| (id, frame.range.clone())))
                .rev()
                .take(16)
                .collect::<Vec<_>>();
            host.failure(&calls, frames.len().saturating_sub(1 + calls.len()));
        }
        outcome
    })
}

// Presence and fallback transitions stage their next continuation without
// retaining owned temporaries in the reentrant execution dispatch frame.
fn default_step<'a, Q, H: Host<'a, Q>>(
    next: Pending<'a, Q, H::Native, H::Kind>,
    pending: &mut Vec<Pending<'a, Q, H::Native, H::Kind>>,
    result: &mut Option<ResultValue<'a, H, Q>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    macro_rules! push {
        ($value:expr) => {
            push_metadata(pending, $value, budget, location)?
        };
    }
    match next {
        Pending::Default(expression, depth, tail) => {
            let left = result.take().expect("default left");
            if host.occurs(&left, budget, location)? {
                *result = Some(left);
            } else {
                let left = host.normalize(left, budget, location)?;
                let PreparedKind::Default { fallback, .. } = &expression.value else {
                    unreachable!()
                };
                let typed = left.contract.is_some()
                    || match &left.data {
                        Data::Constructed(data) => data.declaration().is_some(),
                        Data::Native(_) => false,
                    };
                if typed {
                    push!(Pending::Fallback(expression, depth, left));
                }
                push!(Pending::Eval(fallback, depth + 1, tail && !typed));
            }
        }
        Pending::Fallback(_, _, left) => {
            let right = result.take().expect("default fallback");
            if let Some(mut expected) = left.contract {
                if host.occurs(&right, budget, location)? {
                    while let DeclaredType::Optional(child) = expected {
                        expected = child;
                    }
                }
                let mut right = right;
                admit(&mut right, expected, host, budget, location)?;
                *result = Some(right);
                return Ok(());
            }
            let right = host.normalize(right, budget, location)?;
            let expected = match left.contract {
                Some(c) => crate::runtime_types::constructed_declaration(c, budget, location)?,
                None => copy_declaration(
                    match &left.data {
                        Data::Constructed(data) => data.declaration().expect("typed absent"),
                        Data::Native(_) => unreachable!(),
                    },
                    budget,
                    location,
                )?,
            };
            let expected = if right.scalar(location)?.occurs() {
                underlying(&expected)
            } else {
                &expected
            };
            crate::admit_constructed(right.scalar(location)?, expected, budget, location.clone())?;
            *result = Some(Value::constructed(RuntimeValue::Owned(annotate(
                right.materialize_with(host, budget, location)?,
                copy_declaration(expected, budget, location)?,
                budget,
                location,
            )?)));
        }
        _ => unreachable!("default continuation dispatch"),
    }
    Ok(())
}

fn accumulator_source<'a, Q, H: Host<'a, Q>>(
    items_value: ResultValue<'a, H, Q>,
    element: &DeclaredType<H::Kind>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultSource<'a, H, Q>, H::Error> {
    let items_value = host.normalize(items_value, budget, location)?;
    if let Some(previous) = items_value.contract
        && let Some(previous) = sequence_element(previous, budget, location)?
        && !crate::runtime_types::declarations_compatible(previous, element, budget, location)?
    {
        return Err(error(location, "incompatible declared underlying types").into());
    }
    Ok(match items_value.data {
        Data::Constructed(mut value) => {
            if let Some(previous) = value.declaration()
                && let Some(previous) = sequence_element(previous, budget, location)?
                && !crate::runtime_types::compatible_declarations(
                    previous, element, budget, location,
                )?
            {
                return Err(error(location, "incompatible declared underlying types").into());
            }
            let value = value
                .share(budget, location)?
                .into_present(budget, location)?;
            let ConstructedValue::Array(values) = value.as_ref() else {
                return Err(
                    error(location, "named accumulator needs a positional sequence").into(),
                );
            };
            budget.check_items(values.len(), location.clone())?;
            GeneratorSource::Constructed(value)
        }
        Data::Native(value) => {
            let values = host.sequence_items(value, element, budget, location)?;
            budget.check_items(values.len(), location.clone())?;
            GeneratorSource::Native(values)
        }
    })
}

fn sequence_element<'t, K>(
    mut declaration: &'t DeclaredType<K>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Option<&'t DeclaredType<K>>, QueryError> {
    loop {
        budget.charge(1, location.clone())?;
        match declaration {
            DeclaredType::Optional(child) => declaration = child,
            DeclaredType::Array(child) | DeclaredType::Sequence(child) => return Ok(Some(child)),
            _ => return Ok(None),
        }
    }
}

pub(crate) fn constructed_sequence<'a, Q, H: Host<'a, Q> + ?Sized>(
    values: Vec<ResultValue<'a, H, Q>>,
    element: &DeclaredType<H::Kind>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, QueryError> {
    let element = crate::runtime_types::constructed_declaration(element, budget, location)?;
    let mut output = admit_container::<ConstructedValue>(values.len(), budget, location)?;
    for value in values {
        output.push(value.materialize_with(host, budget, location)?);
    }
    let value = annotate(
        ConstructedValue::Array(output),
        DeclaredType::Sequence(Box::new(element)),
        budget,
        location,
    )?;
    Ok(Value::constructed(RuntimeValue::Owned(value)))
}

fn enter_loop<'a, Q, N, K>(
    expression: &'a PreparedValue<'a, Q>,
    depth: usize,
    tail: bool,
    base: usize,
    pending: &mut Vec<Pending<'a, Q, N, K>>,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    macro_rules! push {
        ($value:expr) => {
            push_metadata(pending, $value, budget, location)?
        };
    }
    match &expression.value {
        PreparedKind::Loop { states, .. } => {
            let values = metadata_vec(states.len(), budget, location)?;
            push!(Pending::Initialize(expression, depth, 0, tail, values));
            if let Some((_, first)) = states.first() {
                push!(Pending::Eval(first, depth + 1, false));
            }
        }
        PreparedKind::Break(value) => {
            let index = loop_target(pending, base, budget, location)?;
            let Pending::Loop(state) = &pending[index] else {
                unreachable!()
            };
            let tail = state.tail;
            push!(Pending::Break(expression, depth));
            push!(Pending::Eval(value, depth + 1, tail));
        }
        PreparedKind::Continue(values) => {
            let staged = metadata_vec(values.len(), budget, location)?;
            push!(Pending::Next(expression, depth, staged));
            if let Some(first) = values.first() {
                push!(Pending::Eval(first, depth + 1, false));
            }
        }
        _ => unreachable!("loop expression dispatch"),
    }
    Ok(())
}

fn loop_step<'a, Q, H: Host<'a, Q>>(
    next: Pending<'a, Q, H::Native, H::Kind>,
    pending: &mut Vec<Pending<'a, Q, H::Native, H::Kind>>,
    frame: &mut Activation<'a, H::Native, H::Kind>,
    result: &mut Option<ResultValue<'a, H, Q>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    macro_rules! push {
        ($value:expr) => {
            push_metadata(pending, $value, budget, location)?
        };
    }
    match next {
        Pending::Initialize(expression, depth, index, tail, mut values) => {
            let PreparedKind::Loop { states, body } = &expression.value else {
                unreachable!()
            };
            if !states.is_empty() {
                values.push(result.take().expect("loop initializer"));
            }
            if let Some((_, next)) = states.get(index + 1) {
                push!(Pending::Initialize(
                    expression,
                    depth,
                    index + 1,
                    tail,
                    values
                ));
                push!(Pending::Eval(next, depth + 1, false));
                return Ok(());
            }
            let mut contracts = metadata_vec(values.len(), budget, location)?;
            for ((_, source), value) in states.iter().zip(&mut values) {
                let at = Location::Query(source.range.clone());
                let contract = match value.contract {
                    Some(contract) => StateContract::Borrowed(contract),
                    None => match host.constructed(value, budget, &at)? {
                        Some(data) => StateContract::Constructed(std::sync::Arc::new(
                            match data.declaration {
                                Some(contract) => copy_declaration(contract, budget, &at)?,
                                None => infer_state(data.value, budget, &at)?,
                            },
                        )),
                        None => {
                            let Data::Native(view) = &value.data else {
                                unreachable!()
                            };
                            StateContract::Native(host.native_contract(
                                host.native_data(view, budget, &at)?,
                                budget,
                                &at,
                            )?)
                        }
                    },
                };
                admit_state(value, &contract, host, budget, &at)?;
                contracts.push(contract);
            }
            let saved = frame.locals.len();
            for ((name, _), value) in states.iter().zip(values) {
                push_metadata(
                    &mut frame.locals,
                    (name.value.as_str(), value),
                    budget,
                    location,
                )?;
            }
            push!(Pending::Loop(Loop {
                expression,
                depth,
                tail,
                saved,
                contracts
            }));
            push!(Pending::Eval(body, depth + 1, false));
        }
        Pending::Loop(_) => {
            return Err(error(location, "loop body fell through").into());
        }
        Pending::Break(_, _) => {
            let index = loop_target(pending, frame.base, budget, location)?;
            pending.truncate(index + 1);
            let Pending::Loop(state) = pending.pop().expect("loop boundary") else {
                unreachable!()
            };
            frame.locals.truncate(state.saved);
        }
        Pending::Next(expression, depth, mut values) => {
            let PreparedKind::Continue(sources) = &expression.value else {
                unreachable!()
            };
            if !sources.is_empty() {
                values.push(result.take().expect("next state operand"));
            }
            if let Some(next) = sources.get(values.len()) {
                push!(Pending::Next(expression, depth, values));
                push!(Pending::Eval(next, depth + 1, false));
                return Ok(());
            }
            let index = loop_target(pending, frame.base, budget, location)?;
            let Pending::Loop(state) = &pending[index] else {
                unreachable!()
            };
            for ((value, contract), source) in values.iter_mut().zip(&state.contracts).zip(sources)
            {
                admit_state(
                    value,
                    contract,
                    host,
                    budget,
                    &Location::Query(source.range.clone()),
                )?;
            }
            // Keep all old slots and iteration locals until every operand
            // and contract succeeds. The selected tuple then replaces them.
            pending.truncate(index + 1);
            let Pending::Loop(state) = pending.pop().expect("loop boundary") else {
                unreachable!()
            };
            frame.locals.truncate(state.saved + values.len());
            for (slot, value) in frame.locals[state.saved..].iter_mut().zip(values) {
                slot.1 = value;
            }
            let PreparedKind::Loop { body, .. } = &state.expression.value else {
                unreachable!()
            };
            let body_depth = state.depth + 1;
            push!(Pending::Loop(state));
            push!(Pending::Eval(body, body_depth, false));
        }
        _ => unreachable!("loop continuation dispatch"),
    }
    Ok(())
}

fn loop_target<Q, N, K>(
    pending: &[Pending<'_, Q, N, K>],
    base: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<usize, QueryError> {
    for index in (base..pending.len()).rev() {
        budget.charge(1, location.clone())?;
        if matches!(pending[index], Pending::Loop(_)) {
            return Ok(index);
        }
    }
    unreachable!("prepared terminal transfer has a loop in this activation")
}

fn admit_state<'a, Q, H: Host<'a, Q>>(
    value: &mut ResultValue<'a, H, Q>,
    contract: &StateContract<'a, H::Kind>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    let expected = match contract {
        StateContract::Borrowed(expected) => return admit(value, expected, host, budget, location),
        StateContract::Constructed(expected) => {
            if let Some(previous) = value.contract
                && !crate::runtime_types::compatible_declarations(
                    expected, previous, budget, location,
                )?
            {
                return Err(error(location, "incompatible declared loop state types").into());
            }
            let data = host
                .constructed(value, budget, location)?
                .ok_or_else(|| error(location, "loop state needs a constructed value"))?;
            if let Some(previous) = data.declaration
                && !crate::runtime_types::compatible_declarations(
                    previous,
                    expected.as_ref(),
                    budget,
                    location,
                )?
            {
                return Err(error(location, "incompatible declared loop state types").into());
            }
            crate::admit_constructed(data.value, expected.as_ref(), budget, location.clone())?;
            match &mut value.data {
                Data::Constructed(data) => data.retain_declaration(expected, budget, location)?,
                Data::Native(view) => {
                    view.retain_declaration(std::sync::Arc::clone(expected), budget, location)?;
                }
            }
            return Ok(());
        }
        StateContract::Native(expected) => expected,
    };
    if let Data::Native(view) = &value.data {
        host.check_native_context(
            host.native_data(view, budget, location)?,
            expected,
            budget,
            location,
        )?;
    }
    check_state_contract(value, expected, host, budget, location)?;
    match host.constructed(value, budget, location)? {
        Some(data) => Ok(crate::admit_constructed(
            data.value,
            expected,
            budget,
            location.clone(),
        )?),
        None => {
            let Data::Native(view) = &value.data else {
                unreachable!()
            };
            host.admit_native(
                host.native_data(view, budget, location)?,
                expected,
                budget,
                location.clone(),
            )
        }
    }
}

fn check_state_contract<'a, Q, H: Host<'a, Q>>(
    value: &ResultValue<'a, H, Q>,
    expected: &DeclaredType<H::Kind>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    let borrowed = match value.contract {
        Some(previous) => {
            crate::runtime_types::declarations_compatible(previous, expected, budget, location)?
        }
        None => true,
    };
    let constructed = match host.constructed(value, budget, location)? {
        Some(data) => match data.declaration {
            Some(previous) => {
                crate::runtime_types::compatible_declarations(previous, expected, budget, location)?
            }
            None => true,
        },
        None => true,
    };
    if !borrowed || !constructed {
        return Err(error(location, "incompatible declared loop state types").into());
    }
    Ok(())
}

fn infer_state(
    value: &ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<DeclaredType<()>, QueryError> {
    let facts = constructed_value_type(value, budget, location.clone())?;
    infer_state_type(facts, budget, location)
}
fn infer_state_type(
    facts: super::ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<DeclaredType<()>, QueryError> {
    use super::{ArrayType, ValueType as T};
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match facts {
            T::Declared { declaration, .. } => declaration,
            T::Boolean => DeclaredType::Boolean,
            T::Integer => DeclaredType::Integer,
            T::Float => DeclaredType::Float,
            T::String => DeclaredType::String,
            T::Member(id) => DeclaredType::Member(id),
            T::Set(Some(kind)) => DeclaredType::Set(kind),
            T::Tuple(fields) => {
                let mut types = metadata_vec(fields.len(), budget, location)?;
                for field in fields {
                    types.push(infer_state_type(field, budget, location)?);
                }
                DeclaredType::Tuple(types)
            }
            T::Dictionary { .. } => DeclaredType::Dictionary(Box::new(DeclaredType::Value), Box::new(DeclaredType::Value)),
            T::Record(fields) => {
                let mut types = metadata_vec(fields.len(), budget, location)?;
                for (name, field) in fields {
                    types.push((name, infer_state_type(field, budget, location)?));
                }
                DeclaredType::Record(types)
            }
            T::Array(ArrayType::Positions(elements) | ArrayType::Elements(elements)) => {
                DeclaredType::Sequence(Box::new(infer_state_element(elements, budget, location)?))
            }
            T::KeyedArray(array) => {
                DeclaredType::Keyed(Box::new(infer_state_element(array.elements, budget, location)?))
            }
            T::Absent | T::Unestablished | T::ScalarKind | T::ArrayKind(_) | T::TupleKind | T::RecordKind | T::Set(None) => {
                return Err(error(location,
                    "loop state needs an established type; use an explicit contract for absent or empty values"));
            }
        })
    })
}
fn infer_state_element(
    elements: Vec<super::ValueType>,
    budget: &mut Budget,
    location: &Location,
) -> Result<DeclaredType<()>, QueryError> {
    let mut types = elements.into_iter();
    let first = types.next().ok_or_else(|| {
        error(
            location,
            "empty loop state needs an explicit element contract",
        )
    })?;
    let expected = infer_state_type(first, budget, location)?;
    for element in types {
        let actual = infer_state_type(element, budget, location)?;
        if !crate::runtime_types::same_declaration(&actual, &expected, budget, location)? {
            return Err(error(
                location,
                "loop state needs homogeneous collection elements",
            ));
        }
    }
    Ok(expected)
}

fn admit<'a, Q, H: Host<'a, Q> + ?Sized>(
    value: &mut ResultValue<'a, H, Q>,
    expected: &'a DeclaredType<H::Kind>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    if let Data::Native(view) = &value.data {
        host.check_native_context(
            host.native_data(view, budget, location)?,
            expected,
            budget,
            location,
        )?;
    }
    if let Some(previous) = value.contract
        && !std::ptr::eq(previous, expected)
        && !crate::runtime_types::declarations_compatible(previous, expected, budget, location)?
    {
        return Err(error(location, "incompatible declared underlying types").into());
    }
    let constructed = host.constructed(value, budget, location)?;
    if let Some(data) = &constructed
        && let Some(previous) = data.declaration
        && !crate::runtime_types::compatible_declarations(previous, expected, budget, location)?
    {
        return Err(error(location, "incompatible declared underlying types").into());
    }
    let reusable = constructed.is_some() || host.retains_native_admission();
    if reusable
        && let Some(previous) = value.contract
        && (std::ptr::eq(previous, expected)
            || crate::runtime_types::same_declaration(previous, expected, budget, location)?)
    {
        value.contract = Some(expected);
        return Ok(());
    }
    match constructed {
        None => {
            let Data::Native(view) = &value.data else {
                unreachable!()
            };
            host.admit_native(
                host.native_data(view, budget, location)?,
                expected,
                budget,
                location.clone(),
            )?
        }
        Some(data) => {
            crate::admit_constructed(data.value, expected, budget, location.clone())?;
        }
    }
    value.contract = Some(expected);
    Ok(())
}
fn start_dictionary<'a, Q, H: Host<'a, Q>>(
    expression: &'a PreparedValue<'a, Q>,
    depth: usize,
    saved: usize,
    pending: &mut Vec<Pending<'a, Q, H::Native, H::Kind>>,
    result: &mut Option<Value<'a, H::Native, H::Kind>>,
    host: &mut H,
    budget: &mut Budget,
) -> Result<(), H::Error> {
    let location = Location::Query(expression.range.clone());
    macro_rules! push {
        ($v:expr) => {
            push_metadata(pending, $v, budget, &location)?
        };
    }
    match &expression.value {
        PreparedKind::Dictionary(entries) => {
            let values = admit_container(0, budget, &location)?;
            if let Some((key, _)) = entries.first() {
                budget.charge(1, Location::Query(key.range.clone()))?;
                push!(Pending::DictionaryKey(
                    expression,
                    depth,
                    0,
                    values,
                    HashIndex::default()
                ));
                push!(Pending::Eval(key, depth + 1, false));
            } else {
                *result = Some(host.finish_dictionary(values, budget, &location)?);
            }
        }
        PreparedKind::DictionaryComprehension {
            general_keys,
            clauses,
            key,
            head,
        } => {
            let c = Rc::new(Comprehension {
                clauses,
                key: Some(key),
                head,
                empty: None,
                general_keys: *general_keys,
                depth,
                output: std::cell::RefCell::new(Output::Dictionary(
                    admit_container(0, budget, &location)?,
                    HashIndex::default(),
                )),
            });
            push!(Pending::FinishComprehension(Rc::clone(&c)));
            push!(Pending::Clauses(c, 0, saved));
        }
        _ => unreachable!("dictionary construction"),
    }
    Ok(())
}

fn dictionary_step<'a, Q, H: Host<'a, Q>>(
    next: Pending<'a, Q, H::Native, H::Kind>,
    pending: &mut Vec<Pending<'a, Q, H::Native, H::Kind>>,
    result: &mut Option<ResultValue<'a, H, Q>>,
    host: &mut H,
    budget: &mut Budget,
) -> Result<(), H::Error> {
    let location = match &next {
        Pending::DictionaryKey(expression, ..) | Pending::DictionaryValue(expression, ..) => {
            Location::Query(expression.range.clone())
        }
        _ => unreachable!("dictionary continuation"),
    };
    macro_rules! push {
        ($v:expr) => {
            push_metadata(pending, $v, budget, &location)?
        };
    }
    match next {
        Pending::DictionaryKey(expression, depth, index, values, hashes) => {
            let PreparedKind::Dictionary(entries) = &expression.value else {
                unreachable!("dictionary entries")
            };
            let key_location = Location::Query(entries[index].0.range.clone());
            let key = result.take().expect("dictionary key");
            let hash = host.collection_hash(&key, budget, &key_location)?;
            for &index in hashes.candidates(hash) {
                if host.collection_equal(&values[index].0, &key, budget, &key_location)? {
                    return Err(error(&key_location, "duplicate dictionary key").into());
                }
            }
            push!(Pending::DictionaryValue(
                expression, depth, index, values, hashes, key, hash
            ));
            push!(Pending::Eval(&entries[index].1, depth + 1, false));
        }
        Pending::DictionaryValue(expression, depth, index, mut values, mut hashes, key, hash) => {
            let PreparedKind::Dictionary(entries) = &expression.value else {
                unreachable!("dictionary entries")
            };
            let value = result.take().expect("dictionary value");
            budget.check_items(sum(values.len(), 1, &location)?, location.clone())?;
            budget.charge_results(1, location.clone())?;
            hashes.insert(hash, values.len(), budget, &location)?;
            push_metadata(&mut values, (key, value), budget, &location)?;
            if let Some((key, _)) = entries.get(index + 1) {
                budget.charge(1, Location::Query(key.range.clone()))?;
                push!(Pending::DictionaryKey(
                    expression,
                    depth,
                    index + 1,
                    values,
                    hashes
                ));
                push!(Pending::Eval(key, depth + 1, false));
            } else {
                *result = Some(host.finish_dictionary(values, budget, &location)?);
            }
        }
        _ => unreachable!("dictionary continuation"),
    }
    Ok(())
}

fn declared_axis_sources<'a, Q>(
    expression: &'a PreparedValue<'a, Q>,
) -> Option<&'a [PreparedValue<'a, Q>]> {
    match &expression.value {
        PreparedKind::KeyedArray {
            declared_axis_sources,
            ..
        }
        | PreparedKind::KeyedComprehension {
            declared_axis_sources,
            ..
        } => declared_axis_sources.as_deref(),
        _ => unreachable!("keyed construction"),
    }
}

fn start_keyed<'a, Q, N, K>(
    expression: &'a PreparedValue<'a, Q>,
    depth: usize,
    saved: usize,
    axes: Option<Vec<KeyAxis>>,
    pending: &mut Vec<Pending<'a, Q, N, K>>,
    result: &mut Option<Value<'a, N, K>>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    let location = Location::Query(expression.range.clone());
    macro_rules! push {
        ($v:expr) => {
            push_metadata(pending, $v, budget, &location)?
        };
    }
    match &expression.value {
        PreparedKind::KeyedArray { entries, .. } => {
            let values = admit_container(entries.len(), budget, &location)?;
            if let Some((key, _)) = entries.first() {
                push!(Pending::KeyedKey(expression, depth, 0, axes, values));
                push!(Pending::Eval(key, depth + 1, false));
            } else {
                *result = Some(Value::constructed(RuntimeValue::Owned(finish_keyed(
                    axes, values, budget, &location,
                )?)));
            }
        }
        PreparedKind::KeyedComprehension {
            clauses,
            key,
            head,
            empty_axis_sources,
            ..
        } => {
            let c = Rc::new(Comprehension {
                clauses,
                key: Some(key),
                head,
                empty: empty_axis_sources.as_deref(),
                general_keys: false,
                depth,
                output: std::cell::RefCell::new(Output::Keyed(
                    axes,
                    admit_container(0, budget, &location)?,
                )),
            });
            push!(Pending::FinishComprehension(Rc::clone(&c)));
            push!(Pending::Clauses(c, 0, saved));
        }
        _ => unreachable!("keyed construction"),
    }
    Ok(())
}

fn append<Q, N, K>(
    c: &Comprehension<'_, Q, N, K>,
    value: ConstructedValue,
    key: Option<Vec<ValueKey>>,
    budget: &mut Budget,
) -> Result<(), QueryError> {
    match &mut *c.output.borrow_mut() {
        Output::Array(_) => unreachable!("array heads retain native values"),
        Output::Set(..) => unreachable!("set heads retain native values"),
        Output::Dictionary(..) => unreachable!("dictionary head has a string key"),
        Output::Keyed(_, entries) => {
            let expression = c.key.expect("key expression");
            let location = Location::Query(expression.range.clone());
            let keys = key.expect("keyed value");
            allocation::<Located<KeyedEntry>>(sum(entries.len(), 1, &location)?, &location)?;
            budget.charge(1, location.clone())?;
            budget.charge_results(1, location)?;
            entries.push(Located::new(
                KeyedEntry { key: keys, value },
                expression.range.clone(),
            ));
        }
    }
    Ok(())
}

fn generator_source<'a, Q, H: Host<'a, Q> + ?Sized>(
    mut value: ResultValue<'a, H, Q>,
    positional: bool,
    host: &mut H,
    budget: &mut Budget,
    cl: &Location,
) -> Result<GeneratorSource<'a, H::Native, H::Kind>, H::Error> {
    host.normalize_in_place(&mut value, budget, cl)?;
    let contract = value.contract;
    Ok(match value.data {
        Data::Native(owner) => {
            if positional {
                let native = host.native_data(&owner, budget, cl)?;
                let values = host
                    .native_list(native)
                    .ok_or_else(|| error(cl, "operation needs positional sequences"))?;
                budget.check_items(values.len(), cl.clone())?;
                let mut items = metadata_vec(values.len(), budget, cl)?;
                let element = contract.and_then(|contract| match contract {
                    DeclaredType::Sequence(element) | DeclaredType::Array(element) => {
                        Some(element.as_ref())
                    }
                    _ => None,
                });
                for index in 0..values.len() {
                    budget.charge_results(1, cl.clone())?;
                    items.push(Value {
                        data: Data::Native(owner.child(NativeStep::List(index), budget, cl)?),
                        contract: element,
                    });
                }
                return Ok(GeneratorSource::Native(items));
            }
            let values = host.subjects(owner, budget, cl)?;
            budget.check_items(values.len(), cl.clone())?;
            let mut items = metadata_vec(values.len(), budget, cl)?;
            let element = contract.and_then(|contract| match contract {
                DeclaredType::Sequence(element) | DeclaredType::Array(element) => {
                    Some(element.as_ref())
                }
                _ => None,
            });
            for value in values {
                budget.charge(1, cl.clone())?;
                budget.charge_results(1, cl.clone())?;
                items.push(Value {
                    data: Data::Native(NativeView::new(std::sync::Arc::new(value))),
                    contract: element,
                });
            }
            GeneratorSource::Native(items)
        }
        Data::Constructed(data) => {
            let value = Value {
                data: Data::<H::Native>::Constructed(data),
                contract,
            };
            let value = if contract.is_some() {
                RuntimeValue::Owned(value.materialize(budget, cl)?)
            } else {
                let Data::Constructed(value) = value.data else {
                    unreachable!()
                };
                value
            };
            let value = if matches!(value.as_ref(), ConstructedValue::Declared { .. }) {
                RuntimeValue::Owned(expose(value.materialize(budget, cl)?, budget, cl)?)
            } else {
                value
            };
            if positional {
                let ConstructedValue::Array(elements) = value.as_ref() else {
                    return Err(error(cl, "operation needs positional sequences").into());
                };
                budget.check_items(elements.len(), cl.clone())?;
            }
            if !matches!(
                value.as_ref(),
                ConstructedValue::Array(_)
                    | ConstructedValue::Set { .. }
                    | ConstructedValue::KeyedArray { .. }
            ) {
                return Err(error(cl, "in source needs an array or supported set").into());
            }
            GeneratorSource::Constructed(value)
        }
    })
}

fn install_pattern<'a, Q, H: Host<'a, Q> + ?Sized>(
    locals: &mut Vec<(&'a str, ResultValue<'a, H, Q>)>,
    pattern: &'a super::Pattern,
    value: Value<'a, H::Native, H::Kind>,
    host: &mut H,
    budget: &mut Budget,
) -> Result<bool, H::Error> {
    let location = Location::Query(pattern.range.clone());
    if !matches!(
        pattern.value,
        super::PatternKind::Name(_) | super::PatternKind::Wildcard
    ) {
        host.inspect_pattern(&value, budget, &location)?;
    }
    let Some(bindings) = super::pattern::bind(
        super::pattern::PatternRef::Pattern(pattern),
        value,
        host,
        budget,
    )?
    else {
        return Ok(false);
    };
    for binding in bindings {
        push_metadata(locals, binding, budget, &location)?;
    }
    Ok(true)
}
fn install_candidate<'a, Q, H: Host<'a, Q> + ?Sized>(
    locals: &mut Vec<(&'a str, ResultValue<'a, H, Q>)>,
    clause: &PreparedGenerator<'a, Q>,
    source: &mut GeneratorSource<'a, H::Native, H::Kind>,
    positions: &[usize],
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, H::Error> {
    let PreparedGenerator::In { binders, case, .. } = clause else {
        unreachable!("candidate belongs to an in generator")
    };
    for (binder, position) in binders.iter().zip(positions) {
        let value = source.child(*position, budget, location)?;
        if let Some(pattern) = binder {
            if !install_pattern(locals, pattern, value, host, budget)? {
                if *case {
                    return Ok(false);
                }
                return Err(error(
                    &Location::Query(pattern.range.clone()),
                    "generator pattern does not match its value",
                )
                .into());
            }
        } else {
            push_metadata(locals, (super::PREDICATE_SUBJECT, value), budget, location)?;
        }
    }
    Ok(true)
}

fn install_predicate<'a, Q, H: Host<'a, Q> + ?Sized>(
    locals: &mut Vec<(&'a str, ResultValue<'a, H, Q>)>,
    mut subject: Value<'a, H::Native, H::Kind>,
    argument: Option<&'a super::Pattern>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), H::Error> {
    if argument.is_some_and(|p| {
        !matches!(
            p.value,
            super::PatternKind::Name(_) | super::PatternKind::Wildcard
        )
    }) {
        host.inspect_pattern(&subject, budget, location)?;
    }
    let bindings = match argument {
        Some(pattern) => {
            let shared = subject.share(budget, location)?;
            super::pattern::bind(
                super::pattern::PatternRef::Pattern(pattern),
                shared,
                host,
                budget,
            )?
            .ok_or_else(|| {
                error(
                    &Location::Query(pattern.range.clone()),
                    "predicate pattern does not match its subject",
                )
            })?
        }
        None => Vec::new(),
    };
    push_metadata(
        locals,
        (super::PREDICATE_SUBJECT, subject),
        budget,
        location,
    )?;
    for binding in bindings {
        push_metadata(locals, binding, budget, location)?;
    }
    Ok(())
}

fn predicate_result<'a, Q, H: Host<'a, Q> + ?Sized>(
    value: ResultValue<'a, H, Q>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let value = boolean(value.materialize_with(host, budget, location)?, location)?;
    Ok(Value::constructed(RuntimeValue::Owned(scalar(
        ConstructedValue::Boolean(value),
        budget,
        location,
    )?)))
}

fn operands<'a, Q>(
    expression: &'a PreparedValue<'a, Q>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<&'a PreparedValue<'a, Q>>, QueryError> {
    let count = match &expression.value {
        PreparedKind::Array(v) | PreparedKind::Tuple(v) | PreparedKind::Set(v) => v.len(),
        PreparedKind::Record(v) => v.len(),
        PreparedKind::With { fields, .. } => sum(fields.len(), 1, location)?,
        PreparedKind::Check {
            operand, message, ..
        } => 1 + usize::from(operand.is_some()) + usize::from(message.is_some()),
        PreparedKind::PushBackAll { .. }
        | PreparedKind::Insert { .. }
        | PreparedKind::Binary { .. }
        | PreparedKind::Lookup { .. }
        | PreparedKind::HasKey { .. }
        | PreparedKind::ArrayAccess { .. }
        | PreparedKind::Range { .. }
        | PreparedKind::Windows { .. }
        | PreparedKind::Head { .. }
        | PreparedKind::Tail { .. }
        | PreparedKind::Align { .. }
        | PreparedKind::ZipExact { .. } => 2,
        _ => 1,
    };
    let mut result = metadata_vec(count, budget, location)?;
    match &expression.value {
        PreparedKind::Array(v) | PreparedKind::Tuple(v) | PreparedKind::Set(v) => result.extend(v),
        PreparedKind::Record(v) => result.extend(v.iter().map(|(_, value)| value)),
        PreparedKind::With { record, fields } => {
            result.push(record.as_ref());
            result.extend(fields.iter().map(|(_, value)| value));
        }
        PreparedKind::PushBackAll {
            queue: left,
            items: right,
        }
        | PreparedKind::Insert {
            set: left,
            item: right,
        }
        | PreparedKind::Binary { left, right, .. }
        | PreparedKind::Windows {
            items: left,
            size: right,
        }
        | PreparedKind::Align { left, right, .. }
        | PreparedKind::ZipExact { left, right } => result.extend([left.as_ref(), right.as_ref()]),
        PreparedKind::Lookup { value, key } | PreparedKind::HasKey { value, key } => {
            result.extend([value.as_ref(), key.as_ref()])
        }
        PreparedKind::ArrayAccess { array, index } => {
            result.extend([array.as_ref(), index.as_ref()])
        }
        PreparedKind::Range { lower, upper } => result.extend([lower.as_ref(), upper.as_ref()]),
        PreparedKind::Head { collection, count } | PreparedKind::Tail { collection, count } => {
            result.extend([collection.as_ref(), count.as_ref()])
        }
        PreparedKind::Check {
            collection,
            operand,
            message,
        } => {
            result.push(collection.as_ref());
            if let Some(operand) = operand {
                result.push(operand.as_ref());
            }
            if let Some(message) = message {
                result.push(message.as_ref());
            }
        }
        PreparedKind::Queue { items: value, .. }
        | PreparedKind::Reverse(value)
        | PreparedKind::PopFront(value)
        | PreparedKind::One(value)
        | PreparedKind::Occurs(value)
        | PreparedKind::Absent(value)
        | PreparedKind::IndexSet(value)
        | PreparedKind::Entries(value)
        | PreparedKind::Unique(value)
        | PreparedKind::Tally { array: value, .. }
        | PreparedKind::Unary { operand: value, .. }
        | PreparedKind::Field { value, .. }
        | PreparedKind::TupleAccess { value, .. }
        | PreparedKind::Reduce { array: value, .. }
        | PreparedKind::Sort { array: value, .. }
        | PreparedKind::Extremum { array: value, .. }
        | PreparedKind::RunsBy { rows: value, .. }
        | PreparedKind::GroupBy { rows: value, .. } => result.push(value),
        _ => unreachable!("special control has no ordinary operands"),
    }
    Ok(result)
}

fn select_record_field<'a, Q, H: Host<'a, Q> + ?Sized>(
    value: &mut ResultValue<'a, H, Q>,
    label: &str,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let data = host.constructed(value, budget, location)?;
    if let Some(data) = &data
        && let Some(DeclaredType::Record(fields)) = data.declaration.map(underlying)
    {
        record_declaration(fields, label, budget, location)?;
    }
    if let Some(mut contract) = value.contract {
        while let DeclaredType::Optional(child) = contract {
            contract = child;
        }
        if let DeclaredType::Record(fields) = contract {
            record_declaration(fields, label, budget, location)?;
        }
    }
    let index = match data {
        Some(data) => {
            let ConstructedValue::Record(fields) = data.value.data() else {
                return Err(error(location, "field access needs a record").into());
            };
            record_field_position(
                fields.iter().map(|(label, _)| label.as_str()),
                label,
                budget,
                location,
            )?
        }
        None => {
            let Data::Native(view) = &value.data else {
                unreachable!("native record")
            };
            let fields = &host
                .native_record(host.native_data(view, budget, location)?)
                .ok_or_else(|| error(location, "field access needs a record"))?
                .fields;
            record_field_position(
                fields.iter().map(|(label, _)| label.as_str()),
                label,
                budget,
                location,
            )?
        }
    };
    super::pattern::select(value, NativeStep::Record(index), host, budget, location)
}

pub(crate) fn record_field_position<'v>(
    labels: impl Iterator<Item = &'v str>,
    label: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<usize, QueryError> {
    for (index, candidate) in labels.enumerate() {
        budget.charge(
            sum(1, candidate.len().min(label.len()), location)?,
            location.clone(),
        )?;
        if candidate == label {
            return Ok(index);
        }
    }
    admit_diagnostic("missing record field: ", label, budget, location)?;
    Err(error(location, format!("missing record field: {label}")))
}

fn windows<'a, Q, H: Host<'a, Q> + ?Sized>(
    items: ResultValue<'a, H, Q>,
    size: ConstructedValue,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
    size_location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let size = integer(size, size_location)?;
    if size <= 0 {
        return Err(error(size_location, "window size must be positive").into());
    }
    let mut source = generator_source(items, true, host, budget, location)?;
    let count = match usize::try_from(size) {
        Ok(size) if size <= source.len() => source.len() - size + 1,
        _ => 0,
    };
    budget.check_items(count, location.clone())?;
    let mut windows = admit_container(count, budget, location)?;
    if count > 0 {
        let size = usize::try_from(size).expect("window fits input length");
        for start in 0..count {
            let window = budget.within_evaluation(1, location.clone(), |budget| {
                budget.check_items(size, location.clone())?;
                let mut items = admit_container(size, budget, location)?;
                for index in start..start + size {
                    items.push(source.child(index, budget, location)?);
                }
                host.finish_array(items, budget, location)
            })?;
            windows.push(window);
        }
    }
    host.finish_array(windows, budget, location)
}

fn runs_by<'a, Q, H: Host<'a, Q> + ?Sized>(
    rows: ResultValue<'a, H, Q>,
    field: &Located<String>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let mut source = generator_source(rows, true, host, budget, location)?;
    let selector = Location::Query(field.range.clone());
    let mut runs = admit_container(0, budget, location)?;
    let mut key = None;
    let mut items = admit_container(0, budget, location)?;
    for index in 0..source.len() {
        let mut row = source.child(index, budget, location)?;
        let next = select_record_field(&mut row, &field.value, host, budget, &selector)?;
        if let Some(previous) = &key
            && !host.collection_equal(previous, &next, budget, &selector)?
        {
            let run = finish_run(
                key.take().expect("current run key"),
                items,
                None,
                host,
                budget,
                location,
            )?;
            budget.check_items(sum(runs.len(), 1, location)?, location.clone())?;
            push_metadata(&mut runs, run, budget, location)?;
            budget.charge_results(1, location.clone())?;
            items = admit_container(0, budget, location)?;
        }
        if key.is_none() {
            key = Some(next);
        }
        budget.check_items(sum(items.len(), 1, location)?, location.clone())?;
        push_metadata(&mut items, row, budget, location)?;
        budget.charge_results(1, location.clone())?;
    }
    if let Some(key) = key {
        let run = finish_run(key, items, None, host, budget, location)?;
        budget.check_items(sum(runs.len(), 1, location)?, location.clone())?;
        push_metadata(&mut runs, run, budget, location)?;
        budget.charge_results(1, location.clone())?;
    }
    host.finish_array(runs, budget, location)
}

pub(crate) fn sort_rows<'a, Q, H: Host<'a, Q> + ?Sized>(
    mut rows: ResultValue<'a, H, Q>,
    field: &Located<super::SortSelector>,
    descending: bool,
    element: Option<&DeclaredType<H::Kind>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let super::SortSelector::Field(label) = &field.value else {
        unreachable!("row ordering selects a field")
    };
    let selector = Location::Query(field.range.clone());
    let mut source = generator_source(rows.share(budget, location)?, true, host, budget, location)?;
    if source.len() == 0 {
        return Ok(rows);
    }
    let mut items = admit_container(source.len(), budget, location)?;
    let mut keys = metadata_vec(source.len(), budget, location)?;
    for index in 0..source.len() {
        let mut row = source.child(index, budget, location)?;
        let key = select_record_field(&mut row, label, host, budget, &selector)?;
        keys.push(host.materialize(key, budget, &selector)?);
        items.push(row);
    }
    super::admit_ordering_values(&keys, budget, &selector)?;
    let order =
        crate::ordering::sort_indices(items.len(), budget, location, |left, right, budget| {
            let comparison =
                super::identity::compare_admitted(&keys[left], &keys[right], budget, &selector)?;
            Ok(if descending {
                comparison.reverse()
            } else {
                comparison
            })
        })?;
    crate::ordering::apply_order(&mut items, order, budget, location)?;
    match element.or_else(|| {
        rows.contract.and_then(|d| match d {
            DeclaredType::Sequence(element) | DeclaredType::Array(element) => {
                Some(element.as_ref())
            }
            _ => None,
        })
    }) {
        Some(element) => host.finish_sequence(items, element, budget, location),
        None => host.finish_array(items, budget, location),
    }
}

struct RowGroup<V> {
    key: ConstructedValue,
    items: Vec<V>,
}

pub(crate) fn group_rows<'a, Q, H: Host<'a, Q> + ?Sized>(
    rows: ResultValue<'a, H, Q>,
    field: &Located<String>,
    element: Option<&DeclaredType<H::Kind>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let declaration = match rows.contract {
        Some(contract) if !crate::runtime_types::contains_native(contract, budget, location)? => {
            Some(crate::runtime_types::constructed_declaration(
                contract, budget, location,
            )?)
        }
        _ => host
            .constructed(&rows, budget, location)?
            .and_then(|data| data.declaration)
            .map(|declaration| copy_declaration(declaration, budget, location))
            .transpose()?,
    };
    let group_declaration = match declaration {
        Some(DeclaredType::Sequence(row) | DeclaredType::Array(row)) => match row.as_ref() {
            DeclaredType::Record(fields) => {
                let key = record_declaration(
                    fields,
                    &field.value,
                    budget,
                    &Location::Query(field.range.clone()),
                )?;
                allocation::<(String, DeclaredType<()>)>(2, location)?;
                budget.charge("key".len() + "items".len() + 2, location.clone())?;
                budget.charge_results(2, location.clone())?;
                Some(DeclaredType::Sequence(Box::new(DeclaredType::Record(
                    vec![
                        ("key".into(), copy_declaration(key, budget, location)?),
                        ("items".into(), DeclaredType::Sequence(row)),
                    ],
                ))))
            }
            _ => None,
        },
        _ => None,
    };
    let mut source = generator_source(rows, true, host, budget, location)?;
    let selector = Location::Query(field.range.clone());
    let mut groups: Vec<RowGroup<ResultValue<'a, H, Q>>> = admit_container(0, budget, location)?;
    let mut key_type = None;
    for index in 0..source.len() {
        budget.charge(1, location.clone())?;
        let mut row = source.child(index, budget, location)?;
        let key = select_record_field(&mut row, &field.value, host, budget, &selector)?;
        let key = host.materialize(key, budget, &selector)?;
        let ty = match key.data() {
            ConstructedValue::Integer(_) => super::ValueType::Integer,
            ConstructedValue::String(_) => super::ValueType::String,
            ConstructedValue::Member(member) => super::ValueType::Member(member.type_id),
            _ => {
                return Err(error(
                    &selector,
                    "group_by keys need integers, strings or nominal members",
                )
                .into());
            }
        };
        budget.charge(1, selector.clone())?;
        if key_type.as_ref().is_some_and(|previous| previous != &ty) {
            return Err(error(&selector, "group_by needs homogeneous key types").into());
        }
        key_type = Some(ty);
        let mut matched = None;
        for (index, group) in groups.iter().enumerate() {
            if super::equal_values(&key, &group.key, budget, &selector)? {
                matched = Some(index);
                break;
            }
        }
        let index = match matched {
            Some(index) => index,
            None => {
                budget.check_items(sum(groups.len(), 1, location)?, location.clone())?;
                let group = RowGroup {
                    key,
                    items: admit_container(0, budget, location)?,
                };
                push_metadata(&mut groups, group, budget, location)?;
                budget.charge_results(1, location.clone())?;
                groups.len() - 1
            }
        };
        let items = &mut groups[index].items;
        budget.check_items(sum(items.len(), 1, location)?, location.clone())?;
        push_metadata(items, row, budget, location)?;
        budget.charge_results(1, location.clone())?;
    }
    let item_element = element.and_then(|element| match element {
        DeclaredType::Record(fields) => {
            fields
                .iter()
                .find_map(|(label, declaration)| match (label.as_str(), declaration) {
                    ("items", DeclaredType::Sequence(element)) => Some(element.as_ref()),
                    _ => None,
                })
        }
        _ => None,
    });
    let mut output = admit_container(groups.len(), budget, location)?;
    for group in groups {
        output.push(finish_run(
            Value::constructed(RuntimeValue::Owned(group.key)),
            group.items,
            item_element,
            host,
            budget,
            location,
        )?);
    }
    match element {
        Some(element) => host.finish_sequence(output, element, budget, location),
        None => {
            let mut result = host.finish_array(output, budget, location)?;
            if let Some(declaration) = group_declaration {
                match &mut result.data {
                    Data::Constructed(value) => value.retain_declaration(
                        &std::sync::Arc::new(declaration),
                        budget,
                        location,
                    )?,
                    Data::Native(_) => {
                        unreachable!("constructed groups retain constructed declarations")
                    }
                }
            }
            Ok(result)
        }
    }
}

struct AlignmentRow<V> {
    key: V,
    before: Option<V>,
    after: Option<V>,
}

fn align<'a, Q, H: Host<'a, Q> + ?Sized>(
    left: ResultValue<'a, H, Q>,
    right: ResultValue<'a, H, Q>,
    field: &Located<String>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let left = generator_source(left, true, host, budget, location)?;
    let right = generator_source(right, true, host, budget, location)?;
    let selector = Location::Query(field.range.clone());
    let mut rows: Vec<AlignmentRow<ResultValue<'a, H, Q>>> = admit_container(0, budget, location)?;
    let mut hashes = HashIndex::default();
    for (mut source, side) in [(left, "left"), (right, "right")] {
        for index in 0..source.len() {
            let mut row = source.child(index, budget, location)?;
            let key = select_record_field(&mut row, &field.value, host, budget, &selector)?;
            if !host.occurs(&key, budget, &selector)? {
                return Err(error(&selector, "alignment keys must be present").into());
            }
            let hash = host.collection_hash(&key, budget, &selector)?;
            let mut matched = None;
            for &candidate in hashes.candidates(hash) {
                budget.charge(1, selector.clone())?;
                if host.collection_equal(&rows[candidate].key, &key, budget, &selector)? {
                    matched = Some(candidate);
                    break;
                }
            }
            if let Some(index) = matched {
                let previous = &mut rows[index];
                if side == "left" || previous.after.is_some() {
                    return Err(error(&selector, format!("duplicate {side} alignment key")).into());
                }
                previous.after = Some(row);
            } else {
                budget.check_items(sum(rows.len(), 1, location)?, location.clone())?;
                hashes.insert(hash, rows.len(), budget, &selector)?;
                let (before, after) = if side == "left" {
                    (Some(row), None)
                } else {
                    (None, Some(row))
                };
                push_metadata(
                    &mut rows,
                    AlignmentRow { key, before, after },
                    budget,
                    location,
                )?;
                budget.charge_results(1, location.clone())?;
            }
        }
    }
    let mut output = admit_container(rows.len(), budget, location)?;
    for row in rows {
        output.push(finish_alignment_row(row, host, budget, location)?);
    }
    host.finish_array(output, budget, location)
}

fn finish_alignment_row<'a, Q, H: Host<'a, Q> + ?Sized>(
    row: AlignmentRow<ResultValue<'a, H, Q>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    budget.within_evaluation(1, location.clone(), |budget| {
        let mut fields = admit_container(3, budget, location)?;
        fields.push(("key".into(), row.key));
        for (label, value) in [("before", row.before), ("after", row.after)] {
            let value = match value {
                Some(value) => value,
                None => Value::constructed(RuntimeValue::Owned(scalar(
                    ConstructedValue::Absent,
                    budget,
                    location,
                )?)),
            };
            fields.push((label.into(), value));
        }
        budget.charge(
            "key".len() + "before".len() + "after".len(),
            location.clone(),
        )?;
        host.finish_record(fields, budget, location)
    })
}

fn finish_run<'a, Q, H: Host<'a, Q> + ?Sized>(
    key: ResultValue<'a, H, Q>,
    items: Vec<ResultValue<'a, H, Q>>,
    element: Option<&DeclaredType<H::Kind>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    budget.within_evaluation(1, location.clone(), |budget| {
        let items = budget.within_evaluation(1, location.clone(), |budget| match element {
            Some(element) => host.finish_sequence(items, element, budget, location),
            None => host.finish_array(items, budget, location),
        })?;
        let mut fields = admit_container(2, budget, location)?;
        for (label, value) in [("key", key), ("items", items)] {
            budget.charge(label.len(), location.clone())?;
            fields.push((label.into(), value));
        }
        host.finish_record(fields, budget, location)
    })
}

fn replace_record_fields<'a, Q, H: Host<'a, Q> + ?Sized>(
    mut record: ResultValue<'a, H, Q>,
    updates: &[(&Located<String>, PreparedValue<'a, Q>)],
    replacements: impl Iterator<Item = ResultValue<'a, H, Q>>,
    host: &mut H,
    budget: &mut Budget,
    location: &Location,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let data = host.constructed(&record, budget, location)?;
    let declaration = data
        .as_ref()
        .and_then(|data| data.declaration)
        .map(|declaration| copy_declaration(declaration, budget, location))
        .transpose()?;
    let labels = match data {
        Some(data) => {
            let ConstructedValue::Record(fields) = data.value.data() else {
                return Err(error(location, "with needs a record").into());
            };
            copy_record_labels(
                fields.iter().map(|(label, _)| label.as_str()),
                budget,
                location,
            )?
        }
        None => {
            let Data::Native(view) = &record.data else {
                unreachable!("native record")
            };
            let labels = host
                .native_record(host.native_data(view, budget, location)?)
                .ok_or_else(|| error(location, "with needs a record"))?
                .fields
                .iter()
                .map(|(label, _)| label.as_str());
            copy_record_labels(labels, budget, location)?
        }
    };
    let mut fields = admit_container(labels.len(), budget, location)?;
    for label in labels {
        fields.push((label, None));
    }
    for ((label, _), value) in updates.iter().zip(replacements) {
        let field_location = Location::Query(label.range.clone());
        let index = record_field_position(
            fields.iter().map(|(label, _)| label.as_str()),
            &label.value,
            budget,
            &field_location,
        )?;
        if fields[index].1.is_some() {
            return Err(error(&field_location, "duplicate update field").into());
        }
        fields[index].1 = Some(value);
    }
    let mut output = admit_container(fields.len(), budget, location)?;
    for (index, (label, replacement)) in fields.into_iter().enumerate() {
        let value = match replacement {
            Some(value) => value,
            None => super::pattern::select(
                &mut record,
                NativeStep::Record(index),
                host,
                budget,
                location,
            )?,
        };
        output.push((label, value));
    }
    let mut result = host.finish_record(output, budget, location)?;
    if let Some(expected) = record.contract {
        admit(&mut result, expected, host, budget, location)?;
    }
    if let Some(declaration) = declaration {
        let value = host.materialize(result, budget, location)?;
        result = Value::constructed(RuntimeValue::Owned(annotate(
            value,
            declaration,
            budget,
            location,
        )?));
        result.contract = record.contract;
    }
    Ok(result)
}

fn copy_record_labels<'v>(
    labels: impl ExactSizeIterator<Item = &'v str>,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<String>, QueryError> {
    let mut result = metadata_vec(labels.len(), budget, location)?;
    for label in labels {
        budget.charge(label.len(), location.clone())?;
        result.push(label.to_owned());
    }
    Ok(result)
}

fn constructed_access_declaration<K>(
    contract: Option<&DeclaredType<K>>,
    data: &RuntimeValue,
    key: &ConstructedValue,
    budget: &mut Budget,
    location: &Location,
) -> Result<Option<DeclaredType<()>>, QueryError> {
    if let Some(contract) = contract
        && !crate::runtime_types::contains_native(contract, budget, location)?
        && let Some(child) = access_declaration(contract, key, budget, location)?
    {
        return crate::runtime_types::constructed_declaration(child, budget, location).map(Some);
    }
    data.declaration()
        .map(|declaration| access_declaration(declaration, key, budget, location))
        .transpose()?
        .flatten()
        .map(|child| copy_declaration(child, budget, location))
        .transpose()
}

pub(crate) fn apply<'a, Q, H: Host<'a, Q> + ?Sized>(
    expression: &'a PreparedValue<'a, Q>,
    mut values: Vec<ResultValue<'a, H, Q>>,
    host: &mut H,
    budget: &mut Budget,
) -> Result<ResultValue<'a, H, Q>, H::Error> {
    let location = Location::Query(expression.range.clone());
    match &expression.value {
        PreparedKind::With { fields, .. } => {
            let mut values = values.into_iter();
            return replace_record_fields(
                values.next().expect("record base"),
                fields,
                values,
                host,
                budget,
                &location,
            );
        }
        PreparedKind::Windows { size, .. } => {
            let size_location = Location::Query(size.range.clone());
            let size =
                host.materialize(values.pop().expect("window size"), budget, &size_location)?;
            return windows(
                values.pop().expect("window input"),
                size,
                host,
                budget,
                &location,
                &size_location,
            );
        }
        PreparedKind::Align { field, .. } => {
            let right = values.pop().expect("right alignment input");
            let left = values.pop().expect("left alignment input");
            return align(left, right, field, host, budget, &location);
        }
        PreparedKind::GroupBy { field, .. } => {
            return group_rows(
                values.pop().expect("group input"),
                field,
                None,
                host,
                budget,
                &location,
            );
        }
        PreparedKind::Sort {
            selector: Some(field),
            descending,
            ..
        } if matches!(field.value, super::SortSelector::Field(_))
            && host.constructed(&values[0], budget, &location)?.is_none()
            && matches!(&values[0].data, Data::Native(view) if host.native_list(host.native_data(view, budget, &location)?).is_some()) =>
        {
            return sort_rows(
                values.pop().expect("sort input"),
                field,
                *descending,
                None,
                host,
                budget,
                &location,
            );
        }
        PreparedKind::RunsBy { field, .. } => {
            return runs_by(
                values.pop().expect("run input"),
                field,
                host,
                budget,
                &location,
            );
        }
        _ => {}
    }
    if let PreparedKind::Field { field, .. } = &expression.value {
        return select_record_field(
            &mut values.pop().expect("field operand"),
            &field.value,
            host,
            budget,
            &location,
        );
    }
    if matches!(expression.value, PreparedKind::ArrayAccess { .. })
        && host
            .constructed(&values[0], budget, &location)?
            .is_some_and(|data| matches!(data.value.data(), ConstructedValue::Dictionary(_)))
    {
        let key = host.materialize(values.pop().expect("dictionary key"), budget, &location)?;
        let value = host.normalize(values.pop().expect("dictionary operand"), budget, &location)?;
        let Data::Constructed(mut data) = value.data else {
            return Err(error(&location, "dictionary access needs constructed data").into());
        };
        let child = constructed_access_declaration(value.contract, &data, &key, budget, &location)?;
        let index = optional_position(data.as_ref(), key, budget, &location)?
            .ok_or_else(|| error(&location, "missing dictionary key"))?;
        let mut selected = data
            .share(budget, &location)?
            .into_data(budget, &location)?
            .child(index, budget, &location)?;
        if let Some(child) = child {
            selected = selected.into_data(budget, &location)?;
            selected.retain_declaration(&std::sync::Arc::new(child), budget, &location)?;
        }
        return Ok(Value::constructed(selected));
    }
    if matches!(expression.value, PreparedKind::ArrayAccess { .. })
        && host
            .constructed(&values[0], budget, &location)?
            .is_some_and(|data| matches!(data.value.data(), ConstructedValue::Record(_)))
    {
        let key = host.materialize(values.pop().expect("record key"), budget, &location)?;
        let ConstructedValue::String(key) = expose(key, budget, &location)? else {
            return Err(error(&location, "record access needs a string label").into());
        };
        return select_record_field(
            &mut values.pop().expect("record operand"),
            &key,
            host,
            budget,
            &location,
        );
    }
    if matches!(
        expression.value,
        PreparedKind::Lookup { .. } | PreparedKind::HasKey { .. }
    ) {
        let key = host.materialize(values.pop().expect("lookup key"), budget, &location)?;
        let value = host.normalize(values.pop().expect("lookup operand"), budget, &location)?;
        let Data::Constructed(mut data) = value.data else {
            return Err(error(&location, "lookup needs a dictionary, record or array").into());
        };
        let membership = matches!(expression.value, PreparedKind::HasKey { .. });
        let child = constructed_access_declaration(value.contract, &data, &key, budget, &location)?;
        let position = optional_position(data.as_ref(), key, budget, &location)?;
        if membership {
            return Ok(Value::constructed(RuntimeValue::Owned(scalar(
                ConstructedValue::Boolean(position.is_some()),
                budget,
                &location,
            )?)));
        }
        let mut selected = match position {
            Some(index) => data
                .share(budget, &location)?
                .into_data(budget, &location)?
                .child(index, budget, &location)?,
            None => RuntimeValue::Owned(scalar(ConstructedValue::Absent, budget, &location)?),
        };
        if let Some(child) = child {
            // Selection preserves admission; only the optional contract is new.
            budget.charge(1, location.clone())?;
            let declaration = std::sync::Arc::new(DeclaredType::Optional(Box::new(child)));
            selected = selected.into_data(budget, &location)?;
            selected.retain_declaration(&declaration, budget, &location)?;
        }
        return Ok(Value::constructed(selected));
    }
    if matches!(expression.value, PreparedKind::ArrayAccess { .. })
        && matches!(&values[0].data, Data::Constructed(data)
            if matches!(data.as_ref().data(), ConstructedValue::Array(_)))
    {
        let key = host.materialize(values.pop().expect("array index"), budget, &location)?;
        let position = usize::try_from(integer(key, &location)?)
            .map_err(|_| error(&location, "missing array position"))?;
        let mut value = values.pop().expect("array operand");
        let Data::Constructed(data) = &value.data else {
            unreachable!()
        };
        let ConstructedValue::Array(elements) = data.as_ref().data() else {
            unreachable!()
        };
        budget.charge(1, location.clone())?;
        let index = position
            .checked_sub(1)
            .filter(|&index| index < elements.len())
            .ok_or_else(|| error(&location, "missing collection position"))?;
        return super::pattern::select(
            &mut value,
            NativeStep::List(index),
            host,
            budget,
            &location,
        );
    }
    if let PreparedKind::Occurs(_) | PreparedKind::Absent(_) = &expression.value {
        let present = host.occurs(&values[0], budget, &location)?;
        return Ok(Value::constructed(RuntimeValue::Owned(scalar(
            ConstructedValue::Boolean(if matches!(expression.value, PreparedKind::Occurs(_)) {
                present
            } else {
                !present
            }),
            budget,
            &location,
        )?)));
    }
    if let PreparedKind::Reduce { operator, .. } = &expression.value {
        return Ok(Value::constructed(RuntimeValue::Owned(reduce_ref(
            *operator,
            host.scalar(&values[0], budget, &location)?,
            budget,
            &location,
        )?)));
    }
    if let PreparedKind::Binary {
        operator:
            operator @ (ValueBinaryOperator::Contains
            | ValueBinaryOperator::StartsWith
            | ValueBinaryOperator::EndsWith),
        ..
    } = &expression.value
    {
        return Ok(Value::constructed(RuntimeValue::Owned(string_match_ref(
            *operator,
            host.scalar(&values[0], budget, &location)?,
            host.scalar(&values[1], budget, &location)?,
            budget,
            &location,
        )?)));
    }
    if let PreparedKind::Binary {
        operator: ValueBinaryOperator::In,
        ..
    } = &expression.value
    {
        return Ok(Value::constructed(RuntimeValue::Owned(
            super::membership_ref(
                host.scalar(&values[0], budget, &location)?,
                host.scalar(&values[1], budget, &location)?,
                budget,
                &location,
            )?,
        )));
    }
    if let PreparedKind::Extremum {
        operator, field, ..
    } = &expression.value
    {
        let mut value = values.pop().expect("extremum operand");
        let position = extremum_position(
            *operator,
            host.scalar(&value, budget, &location)?,
            *field,
            budget,
            &location,
        )?;
        let contract = value.contract.and_then(|mut contract| {
            while let DeclaredType::Optional(child) = contract {
                contract = child;
            }
            match contract {
                DeclaredType::Array(child) | DeclaredType::Sequence(child) => Some(child.as_ref()),
                _ => None,
            }
        });
        let child_declaration = match host
            .constructed(&value, budget, &location)?
            .and_then(|v| v.declaration)
            .map(underlying)
        {
            Some(DeclaredType::Array(child) | DeclaredType::Sequence(child)) => {
                Some(copy_declaration(child, budget, &location)?)
            }
            _ => None,
        };
        let Some(position) = position else {
            let mut absent = scalar(ConstructedValue::Absent, budget, &location)?;
            let declaration = match child_declaration {
                Some(child) => Some(child),
                None => contract
                    .map(|child| {
                        crate::runtime_types::constructed_declaration(child, budget, &location)
                    })
                    .transpose()?,
            };
            if let Some(declaration) = declaration {
                absent = annotate(
                    absent,
                    DeclaredType::Optional(Box::new(declaration)),
                    budget,
                    &location,
                )?;
            }
            return Ok(Value::constructed(RuntimeValue::Owned(absent)));
        };
        if matches!(value.data, Data::Native(_)) {
            let mut selected = super::pattern::select(
                &mut value,
                NativeStep::Tuple(position),
                host,
                budget,
                &location,
            )?;
            selected.contract = contract;
            return Ok(selected);
        }
        let Data::Constructed(data) = value.data else {
            unreachable!("constructed extremum")
        };
        let selected = data
            .into_data(budget, &location)?
            .child(position, budget, &location)?;
        let selected = if let Some(declaration) = child_declaration {
            RuntimeValue::Owned(annotate(
                selected.materialize(budget, &location)?,
                declaration,
                budget,
                &location,
            )?)
        } else {
            selected
        };
        return Ok(Value {
            data: Data::Constructed(selected),
            contract,
        });
    }
    let mut arguments = metadata_vec(values.len(), budget, &location)?;
    for value in values {
        arguments.push(host.materialize(value, budget, &location)?);
    }
    let mut arguments = arguments.into_iter();
    let mut next = || arguments.next().expect("prepared operand");
    let value = match &expression.value {
        PreparedKind::One(_) => {
            let ConstructedValue::Array(values) = expose(next(), budget, &location)? else {
                return Err(error(&location, "one needs a positional array").into());
            };
            take_one(values, expression.range.clone())?
        }
        PreparedKind::Array(_) | PreparedKind::Tuple(_) | PreparedKind::Set(_) => {
            let mut values = admit_container(arguments.len(), budget, &location)?;
            let mut element_type = None;
            let mut hashes = HashIndex::default();
            for value in arguments {
                if matches!(expression.value, PreparedKind::Set(_)) {
                    insert_set(
                        &mut values,
                        &mut hashes,
                        &mut element_type,
                        value,
                        false,
                        budget,
                        &location,
                    )?;
                } else {
                    values.push(value);
                }
            }
            match expression.value {
                PreparedKind::Array(_) => ConstructedValue::Array(values),
                PreparedKind::Tuple(_) => ConstructedValue::Tuple(values),
                _ => ConstructedValue::Set {
                    elements: values.into(),
                    element_type,
                },
            }
        }
        PreparedKind::Record(fields) => {
            let mut result = admit_container(fields.len(), budget, &location)?;
            for ((label, _), value) in fields.iter().zip(arguments) {
                budget.charge(label.value.len(), Location::Query(label.range.clone()))?;
                result.push((label.value.clone(), value));
            }
            ConstructedValue::Record(result)
        }
        PreparedKind::Unary { operator, .. } => unary(*operator, next(), budget, &location)?,
        PreparedKind::Binary { operator, .. } => {
            binary(*operator, next(), next(), budget, &location)?
        }
        PreparedKind::TupleAccess { position, .. } => {
            let ConstructedValue::Tuple(values) = expose(next(), budget, &location)? else {
                return Err(error(&location, "tuple access needs a tuple").into());
            };
            take_position(values, position.value, budget, &location)?
        }
        PreparedKind::ArrayAccess { .. } => {
            let array = expose(next(), budget, &location)?;
            let index = next();
            match array {
                ConstructedValue::Array(values) => {
                    let position = usize::try_from(integer(index, &location)?)
                        .map_err(|_| error(&location, "missing array position"))?;
                    take_position(values, position, budget, &location)?
                }
                ConstructedValue::KeyedArray { axes, entries } => {
                    let key = value_keys(index, budget, &location)?;
                    key_positions(&key, &axes, budget, &location)?;
                    let mut selected = None;
                    for entry in entries {
                        budget.charge(1, location.clone())?;
                        if equal_keys(&key, &entry.key, budget, &location)? {
                            selected = Some(entry.value);
                            break;
                        }
                    }
                    selected.ok_or_else(|| error(&location, "missing keyed array cell"))?
                }
                _ => return Err(error(&location, "array access needs an array").into()),
            }
        }
        PreparedKind::IndexSet(_) => match expose(next(), budget, &location)? {
            ConstructedValue::Array(values) => {
                let len =
                    i64::try_from(values.len()).map_err(|_| error(&location, "index overflow"))?;
                integer_range(1, len, budget, &location)?
            }
            ConstructedValue::KeyedArray { mut axes, .. } if axes.len() == 1 => {
                let axis = axes.pop().expect("rank one");
                let mut values = admit_container(axis.keys.len(), budget, &location)?;
                let mut element_type = Some(key_set_type(axis.key_type));
                let mut hashes = HashIndex::default();
                for key in axis.keys {
                    budget.charge(1, location.clone())?;
                    insert_set(
                        &mut values,
                        &mut hashes,
                        &mut element_type,
                        key_value(key),
                        false,
                        budget,
                        &location,
                    )?;
                }
                ConstructedValue::Set {
                    element_type,
                    elements: values.into(),
                }
            }
            ConstructedValue::KeyedArray { .. } => {
                return Err(error(&location, "index_set needs rank one").into());
            }
            _ => return Err(error(&location, "index_set needs an array").into()),
        },
        PreparedKind::Entries(_) => entries(next(), budget, &location)?,
        PreparedKind::Head { count: source, .. } | PreparedKind::Tail { count: source, .. } => {
            let collection = next();
            let argument_location = Location::Query(source.range.clone());
            let count = crate::nonnegative_integer_to_usize(
                integer(next(), &argument_location)?,
                argument_location,
                "collection bound must be nonnegative",
            )?;
            let operation = if matches!(expression.value, PreparedKind::Head { .. }) {
                crate::Stage::Head(count)
            } else {
                crate::Stage::Tail(count)
            };
            super::select_collection(collection, &operation, budget, &location)?
        }
        PreparedKind::Reverse(_) => {
            super::select_collection(next(), &crate::Stage::Reverse, budget, &location)?
        }
        PreparedKind::Check {
            operand, message, ..
        } => {
            let collection = next();
            let mut expected = None;
            let mut text = None;
            if let Some(operand) = operand {
                let argument_location = Location::Query(operand.range.clone());
                match next().into_data() {
                    ConstructedValue::Integer(value) => {
                        expected = Some(crate::nonnegative_integer_to_usize(
                            value,
                            argument_location.clone(),
                            "collection bound must be nonnegative",
                        )?)
                    }
                    ConstructedValue::String(value) if message.is_none() => text = Some(value),
                    _ => {
                        return Err(error(
                            &argument_location,
                            "check operand must be an integer or string",
                        )
                        .into());
                    }
                }
            }
            if let Some(message) = message {
                let ConstructedValue::String(value) = next().into_data() else {
                    return Err(error(
                        &Location::Query(message.range.clone()),
                        "check message must be a string",
                    )
                    .into());
                };
                text = Some(value);
            }
            super::select_collection(
                collection,
                &crate::Stage::Check {
                    expected,
                    message: text,
                },
                budget,
                &location,
            )?
        }
        PreparedKind::Unique(_) => unique_value(next(), budget, &location)?,
        PreparedKind::Tally {
            general_keys,
            set_type,
            ..
        } => tally_value(next(), *general_keys, *set_type, budget, &location)?,
        PreparedKind::ZipExact { .. } => zip_exact(next(), next(), budget, &location)?,
        PreparedKind::Sort {
            selector,
            descending,
            ..
        } => sort_value(next(), *selector, *descending, budget, &location)?,
        PreparedKind::Range { .. } => integer_range(
            integer(next(), &location)?,
            integer(next(), &location)?,
            budget,
            &location,
        )?,
        _ => unreachable!("control handled by continuations"),
    };
    Ok(Value::constructed(RuntimeValue::Owned(value)))
}

pub(super) struct ScalarHost<'f, 'a, Q, H: ?Sized, F> {
    pub(super) queries: &'f mut H,
    pub(super) frame: &'f mut Frame<'a>,
    pub(super) check: &'f mut F,
    pub(super) query: std::marker::PhantomData<Q>,
    pub(super) calls: &'f mut crate::functions::CallTrace,
}
impl<'a, 's, Q, H, F> Host<'a, Q> for ScalarHost<'_, 's, Q, H, F>
where
    H: ValueQueryHost<Q> + ?Sized,
    F: FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
{
    type Native = std::convert::Infallible;
    type Kind = ();
    type Error = H::Error;
    fn current_input(
        &mut self,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, ()>, Self::Error> {
        Ok(Value::constructed(self.frame.current_input(
            self.queries,
            budget,
            range,
        )?))
    }
    fn predicate_subject(
        &mut self,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, ()>, Self::Error> {
        Ok(Value::constructed(RuntimeValue::Owned(
            self.queries.read_predicate_subject(range, budget)?,
        )))
    }
    fn outer(
        &mut self,
        slot: usize,
        _: &str,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, ()>, Self::Error> {
        Ok(Value::constructed(self.frame.read(
            &ReadSlot::Outer(slot),
            self.queries,
            budget,
            range,
        )?))
    }
    fn native(
        &mut self,
        query: &'a Q,
        arguments: &'a [PreparedArgument<'a, Q>],
        locals: &mut [(&'a str, Value<'a, Self::Native, ()>)],
        _: &'a [Body<'a, Q, ()>],
        _: usize,
        _: Option<usize>,
        projected: bool,
        budget: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Value<'a, Self::Native, ()>, Self::Error> {
        if !projected {
            return Err(QueryError::query(
                range.clone(),
                "native values require a function request",
            )
            .into());
        }
        let location = Location::Query(range.clone());
        let mut values = metadata_vec(locals.len(), budget, &location)?;
        for (name, value) in locals {
            let Data::Constructed(value) = &mut value.data else {
                unreachable!()
            };
            values.push((
                *name,
                value
                    .share(budget, &location)?
                    .into_shared(budget, &location)?,
            ));
        }
        let mut frames = metadata_vec(arguments.len(), budget, &location)?;
        for argument in arguments {
            let mut names = metadata_vec(argument.names.len(), budget, &location)?;
            names.extend_from_slice(&argument.names);
            frames.push(Frame::new(names, budget, &location)?);
        }
        let mut arguments = PreparedArguments {
            arguments,
            frames,
            check_value: self.check,
        };
        let value = self.queries.evaluate_query_with_arguments(
            query,
            &values,
            &mut arguments,
            budget,
            range,
        )?;
        constructed_value_type(&value, budget, location)?;
        Ok(Value::constructed(RuntimeValue::Owned(value)))
    }
    fn admit_native(
        &mut self,
        value: &Self::Native,
        _: &DeclaredType<()>,
        _: &mut Budget,
        _: Location,
    ) -> Result<(), Self::Error> {
        match *value {}
    }
    fn check(
        &mut self,
        value: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        (self.check)(value, budget, location)
    }
    fn failure(&mut self, calls: &[(usize, std::ops::Range<usize>)], omitted: usize) {
        self.calls.append(calls, omitted);
    }
}

pub(super) fn run_scalar<'a, Q, H: ValueQueryHost<Q> + ?Sized>(
    expression: &'a PreparedValue<'a, Q>,
    queries: &mut H,
    frame: &mut Frame<'a>,
    budget: &mut Budget,
    check: &mut impl FnMut(&ConstructedValue, &mut Budget, &Location) -> Result<(), QueryError>,
) -> Result<RuntimeValue, H::Error> {
    let mut names = metadata_vec(
        frame.names.len(),
        budget,
        &Location::Query(expression.range.clone()),
    )?;
    names.extend_from_slice(&frame.names);
    let mut calls = crate::functions::CallTrace::default();
    let mut host = ScalarHost {
        queries,
        frame,
        check,
        query: std::marker::PhantomData,
        calls: &mut calls,
    };
    let value = execute(expression, &names, &[], &mut host, budget, 0, 0)?;
    let Data::Constructed(value) = value.data else {
        unreachable!()
    };
    Ok(value)
}
