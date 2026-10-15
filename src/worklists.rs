//! Native computational values. Their storage is private and never a data format.
use std::{collections::hash_map::RandomState, hash::BuildHasher, ops::Range, sync::Arc};

use crate::{
    Budget, DeclaredType, EvaluationError, FunctionHost, Location, ProjectionHost, QueryError,
};

/// The immutable native owner and its current validity stamp. Hosts must change
/// this exact token whenever previously admitted handles can become invalid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativeContextId(pub [u64; 2]);

/// An exact native identity, including owner, side and object distinctions.
/// This is a canonical semantic token, never a printed label or hash digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativeIdentity(pub [u64; 4]);

/// A FIFO retaining native values and its established whole element contract.
/// Sharing roots is constant work; an advancing queue rotates its rear once.
pub struct NativeQueue<V, K> {
    element: Arc<DeclaredType<K>>,
    context: NativeContextId,
    front: Chain<V>,
    rear: Chain<V>,
    len: usize,
}
struct Chain<V>(Option<Arc<Link<V>>>);
struct Link<V> {
    value: Arc<V>,
    next: Chain<V>,
}
impl<V> Clone for Chain<V> {
    fn clone(&self) -> Self {
        Self(self.0.as_ref().map(Arc::clone))
    }
}
impl<V> Drop for Chain<V> {
    fn drop(&mut self) {
        let mut next = self.0.take();
        while let Some(link) = next {
            match Arc::try_unwrap(link) {
                Ok(mut link) => next = link.next.0.take(),
                Err(_) => break,
            }
        }
    }
}
impl<V> Chain<V> {
    fn take_front(&mut self) -> Option<Arc<V>> {
        let link = self.0.take()?;
        match Arc::try_unwrap(link) {
            Ok(mut link) => {
                self.0 = link.next.0.take();
                Some(link.value)
            }
            Err(link) => {
                self.0 = link.next.0.as_ref().map(Arc::clone);
                Some(Arc::clone(&link.value))
            }
        }
    }
    fn prepend(
        &mut self,
        value: Arc<V>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(), QueryError> {
        grow(1, budget, location)?;
        self.0 = Some(Arc::new(Link {
            value,
            next: Self(self.0.take()),
        }));
        Ok(())
    }
}

/// A native identity set with bounded hash-path copies and exact collision checks.
/// It deliberately exposes no iteration or set algebra.
pub struct NativeIdentitySet<V, K> {
    element: Arc<K>,
    context: NativeContextId,
    root: Option<Arc<Trie<V>>>,
    hash: RandomState,
    len: usize,
}
enum Trie<V> {
    Branch([Option<Arc<Trie<V>>>; 16]),
    Leaf(Vec<(NativeIdentity, Arc<V>)>),
}

/// Host values opt in by wrapping one of these computational values.
#[derive(Debug)]
pub enum ComputationalValue<V, K> {
    Queue(NativeQueue<V, K>),
    IdentitySet(NativeIdentitySet<V, K>),
}

fn grow(count: usize, budget: &mut Budget, location: &Location) -> Result<(), QueryError> {
    budget.charge(count, location.clone())?;
    budget.charge_results(count, location.clone())
}
fn refuse(
    message: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<QueryError, QueryError> {
    budget.charge(message.len(), location.clone())?;
    Ok(QueryError {
        location: location.clone(),
        message: message.into(),
        category: crate::QueryErrorCategory::Other,
    })
}
fn range(location: &Location) -> &Range<usize> {
    match location {
        Location::Query(r) | Location::Source(r) => r,
    }
}
fn current<C, H: FunctionHost<C>>(
    stamp: NativeContextId,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), EvaluationError<H::Error>> {
    budget.charge(1, location.clone())?;
    if host.worklist_context(context, budget, range(location))? != stamp {
        return Err(refuse(
            "stale or foreign computational value context",
            budget,
            location,
        )?
        .into());
    }
    Ok(())
}
impl<V, K> NativeQueue<V, K> {
    /// Number of retained elements; reading metadata does not inspect handles.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the collection has no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The established whole element declaration, including for empty queues.
    pub fn element(&self) -> &DeclaredType<K> {
        &self.element
    }
    /// Retain immutable roots, charging metadata rather than copying elements.
    pub fn share<C, H>(
        &self,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
    {
        current(self.context, host, context, budget, &location)?;
        budget.charge(4, location)?;
        Ok(Self {
            element: Arc::clone(&self.element),
            context: self.context,
            front: self.front.clone(),
            rear: self.rear.clone(),
            len: self.len,
        })
    }
    /// Admit every fresh item before establishing the queue invariant, including
    /// an owner stamp for an explicitly typed empty queue.
    pub fn new<C, H>(
        items: Vec<V>,
        element: DeclaredType<K>,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
        K: Clone + PartialEq,
    {
        crate::runtime_types::validate_declaration(&element, budget, &location)?;
        let declaration = DeclaredType::Queue(Box::new(crate::runtime_types::copy_declaration(
            &element, budget, &location,
        )?));
        host.prepare_computational(Some(&declaration), budget, range(&location))?;
        let stamp = host.worklist_context(context, budget, range(&location))?;
        grow(1, budget, &location)?;
        Self {
            element: Arc::new(element),
            context: stamp,
            front: Chain(None),
            rear: Chain(None),
            len: 0,
        }
        .push_back_all(items, host, context, budget, location)
    }
    /// Append freshly admitted items in order. Retained aliases are unchanged.
    pub fn push_back_all<C, H>(
        mut self,
        items: Vec<V>,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
        K: Clone + PartialEq,
    {
        current(self.context, host, context, budget, &location)?;
        let len = self
            .len
            .checked_add(items.len())
            .ok_or_else(|| QueryError::query(range(&location).clone(), "queue length overflow"))?;
        budget.check_items(len, location.clone())?;
        for item in &items {
            crate::admit_value(item, &self.element, host, context, budget, location.clone())?;
        }
        for item in items {
            grow(1, budget, &location)?;
            self.rear.prepend(Arc::new(item), budget, &location)?;
            self.len += 1;
        }
        Ok(self)
    }
    /// Return the first value and remaining queue. Rear rotation charges each
    /// shared/moved link; an advancing computation has amortized constant work.
    #[expect(
        clippy::type_complexity,
        reason = "Pop preserves a shared native item and the remaining queue without a native Clone bound"
    )]
    pub fn pop_front<C, H>(
        mut self,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Option<(Arc<V>, Self)>, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
    {
        current(self.context, host, context, budget, &location)?;
        budget.charge(1, location.clone())?;
        if self.front.0.is_none() {
            while self.rear.0.is_some() {
                budget.charge(1, location.clone())?;
                let item = self.rear.take_front().expect("nonempty rear");
                self.front.prepend(item, budget, &location)?;
            }
        }
        let Some(item) = self.front.take_front() else {
            return Ok(None);
        };
        self.len -= 1;
        Ok(Some((item, self)))
    }
}
impl<V, K> NativeIdentitySet<V, K> {
    /// Number of retained elements; reading metadata does not inspect handles.
    pub fn len(&self) -> usize {
        self.len
    }
    /// Whether the collection has no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    /// The native element kind, including for empty sets.
    pub fn element(&self) -> &K {
        &self.element
    }
    /// Retain roots after checking the current native owner.
    ///
    /// # Errors
    /// Stale context and exhausted work retain `location`.
    pub fn share<C, H>(
        &self,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
    {
        current(self.context, host, context, budget, &location)?;
        budget.charge(4, location)?;
        Ok(Self {
            element: Arc::clone(&self.element),
            context: self.context,
            root: self.root.as_ref().map(Arc::clone),
            hash: self.hash.clone(),
            len: self.len,
        })
    }
    /// Establish an explicitly typed empty set in the current immutable owner.
    ///
    /// # Errors
    /// Unsupported host context or exhausted growth allowances retain location.
    pub fn new<C, H>(
        element: K,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
        K: Clone,
    {
        host.prepare_computational(
            Some(&DeclaredType::IdentitySet(element.clone())),
            budget,
            range(&location),
        )?;
        let context = host.worklist_context(context, budget, range(&location))?;
        grow(1, budget, &location)?;
        Ok(Self {
            element: Arc::new(element),
            context,
            root: None,
            hash: RandomState::new(),
            len: 0,
        })
    }
    fn identity<C, H>(
        &self,
        item: &V,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<(NativeIdentity, u64), EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
        K: Clone,
    {
        current(self.context, host, context, budget, location)?;
        crate::admit_value(
            item,
            &DeclaredType::Native((*self.element).clone()),
            host,
            context,
            budget,
            location.clone(),
        )?;
        let id = host.native_identity(item, &self.element, context, budget, range(location))?;
        budget.charge(4, location.clone())?;
        Ok((id, self.hash.hash_one(id)))
    }
    /// Admit the actual operand and check its exact host identity.
    ///
    /// # Errors
    /// Host admission, stale context and resource failures propagate.
    pub fn contains<C, H>(
        &self,
        item: &V,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<bool, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
        K: Clone,
    {
        let (id, hash) = self.identity(item, host, context, budget, &location)?;
        lookup(self.root.as_deref(), id, hash, 0, budget, &location).map_err(Into::into)
    }
    /// Admit and retain an item, sharing the existing value for a duplicate.
    ///
    /// # Errors
    /// Host admission, stale context, item ceilings and growth failures propagate.
    pub fn insert<C, H>(
        mut self,
        item: V,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
        K: Clone,
    {
        let (id, hash) = self.identity(&item, host, context, budget, &location)?;
        if lookup(self.root.as_deref(), id, hash, 0, budget, &location)? {
            return Ok(self);
        }
        let len = self.len.checked_add(1).ok_or_else(|| {
            QueryError::query(range(&location).clone(), "identity set length overflow")
        })?;
        budget.check_items(len, location.clone())?;
        grow(1, budget, &location)?;
        self.root = Some(insert(
            self.root.take(),
            id,
            Arc::new(item),
            hash,
            0,
            budget,
            &location,
        )?);
        self.len = len;
        Ok(self)
    }
}
fn lookup<V>(
    node: Option<&Trie<V>>,
    id: NativeIdentity,
    hash: u64,
    depth: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, QueryError> {
    budget.charge(1, location.clone())?;
    match node {
        None => Ok(false),
        Some(Trie::Branch(children)) => lookup(
            children[((hash >> (depth * 4)) & 15) as usize].as_deref(),
            id,
            hash,
            depth + 1,
            budget,
            location,
        ),
        Some(Trie::Leaf(entries)) => {
            for (other, _) in entries {
                budget.charge(4, location.clone())?;
                if *other == id {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}
fn insert<V>(
    node: Option<Arc<Trie<V>>>,
    id: NativeIdentity,
    value: Arc<V>,
    hash: u64,
    depth: usize,
    budget: &mut Budget,
    location: &Location,
) -> Result<Arc<Trie<V>>, QueryError> {
    budget.charge(1, location.clone())?;
    // A fixed 16-nibble path gives bounded stack and alias-copy costs. Only the
    // final leaf is a collision bucket; equality uses the full exact token.
    let mut node = match node {
        Some(node) => match Arc::try_unwrap(node) {
            Ok(node) => node,
            Err(node) => match node.as_ref() {
                Trie::Branch(children) => {
                    budget.charge(16, location.clone())?;
                    Trie::Branch(std::array::from_fn(|i| {
                        children[i].as_ref().map(Arc::clone)
                    }))
                }
                Trie::Leaf(entries) => {
                    grow(entries.len(), budget, location)?;
                    Trie::Leaf(entries.iter().map(|(id, v)| (*id, Arc::clone(v))).collect())
                }
            },
        },
        None => {
            if depth == 16 {
                Trie::Leaf(Vec::new())
            } else {
                Trie::Branch(std::array::from_fn(|_| None))
            }
        }
    };
    match &mut node {
        Trie::Branch(children) => {
            let slot = ((hash >> (depth * 4)) & 15) as usize;
            children[slot] = Some(insert(
                children[slot].take(),
                id,
                value,
                hash,
                depth + 1,
                budget,
                location,
            )?);
        }
        Trie::Leaf(entries) => {
            grow(1, budget, location)?;
            entries.push((id, value));
        }
    }
    grow(1, budget, location)?;
    if matches!(node, Trie::Branch(_)) {
        grow(16, budget, location)?;
    }
    Ok(Arc::new(node))
}
impl<V, K> ComputationalValue<V, K> {
    pub fn share<C, H>(
        &self,
        host: &H,
        context: &mut C,
        budget: &mut Budget,
        location: Location,
    ) -> Result<Self, EvaluationError<H::Error>>
    where
        H: FunctionHost<C, Value = V> + ProjectionHost<C, Kind = K>,
    {
        Ok(match self {
            Self::Queue(q) => Self::Queue(q.share(host, context, budget, location)?),
            Self::IdentitySet(s) => Self::IdentitySet(s.share(host, context, budget, location)?),
        })
    }
    pub fn declaration(
        &self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<DeclaredType<K>, QueryError>
    where
        K: Clone,
    {
        Ok(match self {
            Self::Queue(q) => DeclaredType::Queue(Box::new(
                crate::runtime_types::copy_declaration(q.element(), budget, location)?,
            )),
            Self::IdentitySet(s) => {
                budget.charge(1, location.clone())?;
                DeclaredType::IdentitySet(s.element().clone())
            }
        })
    }
}

/// Check computational wrappers and explicit native tuple/record containers.
/// Call this first in a participating adapter's `admit_native`; `false` leaves
/// ordinary native admission with that adapter. Stored elements are already
/// admitted: reuse checks the current owner stamp and whole metadata contract.
/// Fresh items still receive the adapter's actual native admission.
pub fn admit_computational<C, H: FunctionHost<C>>(
    value: &H::Value,
    expected: &DeclaredType<<H as ProjectionHost<C>>::Kind>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<bool, EvaluationError<H::Error>> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let Some(c) = host.computational_value(value) {
            let stamp = match c {
                ComputationalValue::Queue(q) => q.context,
                ComputationalValue::IdentitySet(s) => s.context,
            };
            current(stamp, host, context, budget, &location)?;
            let mut expected = expected;
            while let DeclaredType::Optional(child) = expected {
                expected = child;
            }
            let actual = c.declaration(budget, &location)?;
            if !crate::runtime_types::same_declaration(&actual, expected, budget, &location)? {
                return Err(refuse(
                    "incompatible computational value declaration",
                    budget,
                    &location,
                )?
                .into());
            }
            return Ok(true);
        }
        if !crate::runtime_types::contains_computational(expected, budget, &location)? {
            return Ok(false);
        }
        if let Some(v) = host.constructed_value(value) {
            crate::admit_constructed(v, expected, budget, location.clone())?;
            return Ok(true);
        }
        match expected {
            DeclaredType::Optional(child) => {
                if !admit_computational(value, child, host, context, budget, location.clone())? {
                    crate::admit_value(value, child, host, context, budget, location.clone())?;
                }
            }
            DeclaredType::Tuple(children) => {
                let values = host.tuple_view(value).ok_or_else(|| {
                    QueryError::query(range(&location).clone(), "expected native tuple")
                })?;
                if values.len() != children.len() {
                    return Err(refuse("tuple length mismatch", budget, &location)?.into());
                }
                for (v, d) in values.iter().zip(children) {
                    if !admit_computational(v, d, host, context, budget, location.clone())? {
                        crate::admit_value(v, d, host, context, budget, location.clone())?;
                    }
                }
            }
            DeclaredType::Array(element) | DeclaredType::Sequence(element) => {
                let values =
                    host.sequence_items(value, element, context, budget, range(&location))?;
                budget.check_items(values.len(), location.clone())?;
                for v in &values {
                    if !admit_computational(v, element, host, context, budget, location.clone())? {
                        crate::admit_value(v, element, host, context, budget, location.clone())?;
                    }
                }
            }
            DeclaredType::Record(children) => {
                let values = host.capture_record(value).ok_or_else(|| {
                    QueryError::query(range(&location).clone(), "expected native record")
                })?;
                for (name, _) in &values.fields {
                    let mut allowed = false;
                    for (s, _) in children {
                        budget.charge(1 + s.len().min(name.len()), location.clone())?;
                        allowed |= s == name;
                    }
                    if !allowed {
                        return Err(refuse(
                            "record label outside declared schema",
                            budget,
                            &location,
                        )?
                        .into());
                    }
                }
                for (name, d) in children {
                    let mut found = None;
                    for (s, v) in &values.fields {
                        budget.charge(1 + s.len().min(name.len()), location.clone())?;
                        if s == name {
                            found = Some(v);
                            break;
                        }
                    }
                    if let Some(v) = found {
                        if !admit_computational(v, d, host, context, budget, location.clone())? {
                            crate::admit_value(v, d, host, context, budget, location.clone())?;
                        }
                    } else if !matches!(d, DeclaredType::Optional(_)) {
                        return Err(
                            refuse("missing declared record field", budget, &location)?.into()
                        );
                    }
                }
            }
            _ => return Err(refuse("expected computational value", budget, &location)?.into()),
        }
        Ok(true)
    })
}

/// Validate current context even when the evaluator can reuse earlier admission.
pub(crate) fn check_context<C, H: FunctionHost<C>>(
    value: &H::Value,
    expected: &DeclaredType<<H as ProjectionHost<C>>::Kind>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), EvaluationError<H::Error>> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let Some(c) = host.computational_value(value) {
            let stamp = match c {
                ComputationalValue::Queue(q) => q.context,
                ComputationalValue::IdentitySet(s) => s.context,
            };
            current(stamp, host, context, budget, location)?;
        } else if crate::runtime_types::contains_computational(expected, budget, location)? {
            admit_computational(value, expected, host, context, budget, location.clone())?;
        } else if let Some(values) = host.tuple_view(value) {
            for v in values {
                check_context(v, expected, host, context, budget, location)?;
            }
        } else if let Some(values) = host.capture_record(value) {
            for (_, v) in &values.fields {
                check_context(v, expected, host, context, budget, location)?;
            }
        }
        Ok(())
    })
}

pub(crate) fn scalar_contract<K>(value: &crate::ConstructedValue) -> Option<DeclaredType<K>> {
    Some(match value.data() {
        crate::ConstructedValue::Boolean(_) => DeclaredType::Boolean,
        crate::ConstructedValue::Integer(_) => DeclaredType::Integer,
        crate::ConstructedValue::Float(_) => DeclaredType::Float,
        crate::ConstructedValue::String(_) => DeclaredType::String,
        crate::ConstructedValue::Member(m) => DeclaredType::Member(m.type_id),
        _ => return None,
    })
}

// This only translates constructed type facts into a native-capable declaration;
// actual whole-value admission remains with runtime_types and the host.
pub(crate) fn fact_contract<K>(
    fact: &crate::ValueType,
    budget: &mut Budget,
    location: &Location,
) -> Result<Option<DeclaredType<K>>, QueryError> {
    use crate::{ArrayType, ValueType as T};
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(Some(match fact {
            T::Boolean => DeclaredType::Boolean,
            T::Integer => DeclaredType::Integer,
            T::Float => DeclaredType::Float,
            T::String => DeclaredType::String,
            T::Member(id) => DeclaredType::Member(*id),
            T::Set(Some(t)) => DeclaredType::Set(*t),
            T::Declared { declaration, .. } => promote_declaration(declaration, budget, location)?,
            T::Array(ArrayType::Positions(types) | ArrayType::Elements(types)) => match types
                .first()
                .map(|t| fact_contract(t, budget, location))
                .transpose()?
                .flatten()
            {
                Some(t) => DeclaredType::Sequence(Box::new(t)),
                None => return Ok(None),
            },
            T::Tuple(types) => {
                let mut output = Vec::new();
                for t in types {
                    let Some(t) = fact_contract(t, budget, location)? else {
                        return Ok(None);
                    };
                    grow(1, budget, location)?;
                    output.push(t);
                }
                DeclaredType::Tuple(output)
            }
            T::Record(fields) => {
                let mut output = Vec::new();
                for (name, t) in fields {
                    let Some(t) = fact_contract(t, budget, location)? else {
                        return Ok(None);
                    };
                    grow(1 + name.len(), budget, location)?;
                    output.push((name.clone(), t));
                }
                DeclaredType::Record(output)
            }
            _ => return Ok(None),
        }))
    })
}
fn promote_declaration<K>(
    d: &DeclaredType<()>,
    budget: &mut Budget,
    location: &Location,
) -> Result<DeclaredType<K>, QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        Ok(match d {
            DeclaredType::Boolean => DeclaredType::Boolean,
            DeclaredType::Integer => DeclaredType::Integer,
            DeclaredType::Float => DeclaredType::Float,
            DeclaredType::String => DeclaredType::String,
            DeclaredType::Member(t) => DeclaredType::Member(*t),
            DeclaredType::Set(t) => DeclaredType::Set(*t),
            DeclaredType::Optional(c) => {
                DeclaredType::Optional(Box::new(promote_declaration(c, budget, location)?))
            }
            DeclaredType::Sequence(c) => {
                DeclaredType::Sequence(Box::new(promote_declaration(c, budget, location)?))
            }
            DeclaredType::Array(c) => {
                DeclaredType::Array(Box::new(promote_declaration(c, budget, location)?))
            }
            DeclaredType::Keyed(c) => {
                DeclaredType::Keyed(Box::new(promote_declaration(c, budget, location)?))
            }
            DeclaredType::Tuple(c) => {
                let mut output = Vec::new();
                for c in c {
                    grow(1, budget, location)?;
                    output.push(promote_declaration(c, budget, location)?);
                }
                DeclaredType::Tuple(output)
            }
            DeclaredType::Record(c) => {
                let mut output = Vec::new();
                for (name, c) in c {
                    grow(1 + name.len(), budget, location)?;
                    output.push((name.clone(), promote_declaration(c, budget, location)?));
                }
                DeclaredType::Record(output)
            }
            DeclaredType::Queue(_) | DeclaredType::IdentitySet(_) | DeclaredType::Native(_) => {
                return Err(refuse(
                    "computational/native declaration requires a host value",
                    budget,
                    location,
                )?);
            }
        })
    })
}

impl<V, K> std::fmt::Debug for NativeQueue<V, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeQueue")
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}
impl<V, K> std::fmt::Debug for NativeIdentitySet<V, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeIdentitySet")
            .field("len", &self.len)
            .finish_non_exhaustive()
    }
}
