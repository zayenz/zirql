//! Shared collection operations over host-owned values. Native leaves keep their
//! host identity; collection structure is compared without rendering it.
use crate::{
    Budget, ConstructedValue as V, DeclaredType, EvaluationError, FunctionHost, Location,
    QueryError,
};

enum Node<'a, T> {
    Host(&'a T),
    Data(&'a V),
    Text(&'a str),
}
impl<T> Copy for Node<'_, T> {}
impl<T> Clone for Node<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, T> Node<'a, T> {
    fn data<C, H: FunctionHost<C, Value = T>>(self, host: &H) -> Option<&'a V> {
        match self {
            Self::Data(value) => Some(value.data()),
            Self::Host(value) => host.constructed_value(value).map(V::data),
            Self::Text(_) => None,
        }
    }
}

enum Items<'a, T> {
    Host(&'a [T]),
    Data(&'a [V]),
}
impl<'a, T> Items<'a, T> {
    fn len(&self) -> usize {
        match self {
            Self::Host(v) => v.len(),
            Self::Data(v) => v.len(),
        }
    }
    fn get(&self, index: usize) -> Node<'a, T> {
        match self {
            Self::Host(v) => Node::Host(&v[index]),
            Self::Data(v) => Node::Data(&v[index]),
        }
    }
}
enum Entries<'a, T> {
    Host(&'a [(T, T)]),
    Data(&'a [(V, V)]),
    Fields(&'a [(String, V)]),
    Capture(&'a [(String, T)]),
}
impl<'a, T> Entries<'a, T> {
    fn len(&self) -> usize {
        match self {
            Self::Host(v) => v.len(),
            Self::Data(v) => v.len(),
            Self::Fields(v) => v.len(),
            Self::Capture(v) => v.len(),
        }
    }
    fn get(&self, index: usize) -> (Node<'a, T>, Node<'a, T>) {
        match self {
            Self::Host(v) => (Node::Host(&v[index].0), Node::Host(&v[index].1)),
            Self::Data(v) => (Node::Data(&v[index].0), Node::Data(&v[index].1)),
            Self::Fields(v) => (Node::Text(&v[index].0), Node::Data(&v[index].1)),
            Self::Capture(v) => (Node::Text(&v[index].0), Node::Host(&v[index].1)),
        }
    }
}
fn items<'a, C, H: FunctionHost<C>>(
    node: Node<'a, H::Value>,
    host: &H,
    kind: u8,
) -> Option<Items<'a, H::Value>> {
    match node {
        Node::Host(value) => {
            if let Some(data) = host.constructed_value(value) {
                return items(Node::Data(data), host, kind);
            }
            match kind {
                0 => host.tuple_view(value),
                1 => host.positional_view(value),
                _ => host.set_view(value),
            }
            .map(Items::Host)
        }
        Node::Data(value) => match (kind, value.data()) {
            (0, V::Tuple(values)) | (1, V::Array(values)) => Some(Items::Data(values)),
            (
                2,
                V::Set {
                    elements: values, ..
                },
            ) => Some(Items::Data(values)),
            _ => None,
        },
        Node::Text(_) => None,
    }
}
fn entries<'a, C, H: FunctionHost<C>>(
    node: Node<'a, H::Value>,
    host: &H,
) -> Option<Entries<'a, H::Value>> {
    match node {
        Node::Host(value) => {
            if let Some(data) = host.constructed_value(value) {
                return entries(Node::Data(data), host);
            }
            host.dictionary_view(value).map(Entries::Host).or_else(|| {
                host.capture_record(value)
                    .map(|v| Entries::Capture(&v.fields))
            })
        }
        Node::Data(value) => match value.data() {
            V::Dictionary(v) => Some(Entries::Data(v)),
            V::Record(v) => Some(Entries::Fields(v)),
            _ => None,
        },
        Node::Text(_) => None,
    }
}
fn text<'a, C, H: FunctionHost<C>>(node: Node<'a, H::Value>, host: &H) -> Option<&'a str> {
    match node {
        Node::Text(text) => Some(text),
        _ => match node.data(host) {
            Some(V::String(text)) => Some(text),
            _ => None,
        },
    }
}
fn equal<C, H: FunctionHost<C>>(
    left: Node<'_, H::Value>,
    right: Node<'_, H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
) -> Result<bool, EvaluationError<H::Error>> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let (Some(a), Some(b)) = (left.data(host), right.data(host)) {
            return Ok(crate::value::key_equal(a, b, budget, location)?);
        }
        if let (Some(a), Some(b)) = (text(left, host), text(right, host)) {
            budget.charge(a.len().min(b.len()), location.clone())?;
            return Ok(a == b);
        }
        for kind in 0..3 {
            if let Some(a) = items(left, host, kind) {
                let Some(b) = items(right, host, kind) else {
                    return Ok(false);
                };
                if a.len() != b.len() {
                    return Ok(false);
                }
                let mut membership = crate::value::identity::HashIndex::default();
                if kind == 2 {
                    for other in 0..b.len() {
                        membership.insert(
                            hash(b.get(other), host, context, budget, location)?,
                            other,
                            budget,
                            location,
                        )?;
                    }
                }
                for index in 0..a.len() {
                    let mut found = false;
                    if kind == 2 {
                        let key = hash(a.get(index), host, context, budget, location)?;
                        for &other in membership.candidates(key) {
                            if equal(a.get(index), b.get(other), host, context, budget, location)? {
                                found = true;
                                break;
                            }
                        }
                    } else {
                        found = equal(a.get(index), b.get(index), host, context, budget, location)?;
                    }
                    if !found {
                        return Ok(false);
                    }
                }
                return Ok(true);
            }
            if items(right, host, kind).is_some() {
                return Ok(false);
            }
        }
        if let Some(a) = entries(left, host) {
            let Some(b) = entries(right, host) else {
                return Ok(false);
            };
            if a.len() != b.len() {
                return Ok(false);
            }
            let mut membership = crate::value::identity::HashIndex::default();
            for other in 0..b.len() {
                membership.insert(
                    hash(b.get(other).0, host, context, budget, location)?,
                    other,
                    budget,
                    location,
                )?;
            }
            for index in 0..a.len() {
                let (key, value) = a.get(index);
                let mut found = false;
                let hashed = hash(key, host, context, budget, location)?;
                for &other in membership.candidates(hashed) {
                    let (other_key, other_value) = b.get(other);
                    if equal(key, other_key, host, context, budget, location)? {
                        if !equal(value, other_value, host, context, budget, location)? {
                            return Ok(false);
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let (Node::Host(a), Node::Host(b)) = (left, right) else {
            return Ok(false);
        };
        if host.constructed_value(a).is_some() || host.constructed_value(b).is_some() {
            return Ok(false);
        }
        let range = range(location);
        let a_kind = host.kind(a, budget, range)?;
        let b_kind = host.kind(b, budget, range)?;
        if a_kind != b_kind {
            return Ok(false);
        }
        for value in [a, b] {
            crate::admit_value(
                value,
                &DeclaredType::Native(a_kind.clone()),
                host,
                context,
                budget,
                location.clone(),
            )?;
        }
        Ok(host.native_identity(a, &a_kind, context, budget, range)?
            == host.native_identity(b, &b_kind, context, budget, range)?)
    })
}
fn range(location: &Location) -> &std::ops::Range<usize> {
    match location {
        Location::Query(r) | Location::Source(r) => r,
    }
}

fn hash<C, H: FunctionHost<C>>(
    value: Node<'_, H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
) -> Result<u64, EvaluationError<H::Error>> {
    use crate::value::identity::{self, hash_parts, hash_unordered};
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let Some(data) = value.data(host) {
            return Ok(identity::hash_admitted(data, budget, location)?);
        }
        if let Node::Text(text) = value {
            budget.charge(text.len(), location.clone())?;
            return Ok(identity::hash_string(text));
        }
        for (kind, tag) in [
            (0, identity::TUPLE),
            (1, identity::ARRAY),
            (2, identity::SET),
        ] {
            if let Some(values) = items(value, host, kind) {
                crate::projection::admit_slots::<u64>(values.len(), budget, range(location))?;
                let mut hashes = Vec::with_capacity(values.len());
                for index in 0..values.len() {
                    hashes.push(hash(values.get(index), host, context, budget, location)?);
                }
                return Ok(if kind == 2 {
                    hash_unordered(tag, hashes)
                } else {
                    hash_parts(tag, hashes)
                });
            }
        }
        if let Some(entries) = entries(value, host) {
            crate::projection::admit_slots::<u64>(entries.len(), budget, range(location))?;
            let mut hashes = Vec::with_capacity(entries.len());
            for index in 0..entries.len() {
                let (key, value) = entries.get(index);
                hashes.push(hash_parts(
                    identity::ENTRY,
                    [
                        hash(key, host, context, budget, location)?,
                        hash(value, host, context, budget, location)?,
                    ],
                ));
            }
            return Ok(hash_unordered(identity::DICTIONARY, hashes));
        }
        let Node::Host(value) = value else {
            unreachable!("data and text handled")
        };
        let kind = host.kind(value, budget, range(location))?;
        let identity = host.native_identity(value, &kind, context, budget, range(location))?;
        Ok(hash_parts(255, identity.0))
    })
}

fn rank<C, H: FunctionHost<C>>(value: Node<'_, H::Value>, host: &H) -> u8 {
    use crate::value::identity;
    if let Some(data) = value.data(host) {
        return identity::rank(data);
    }
    if matches!(value, Node::Text(_)) {
        return 4;
    }
    for (kind, rank) in [
        (0, identity::TUPLE),
        (1, identity::ARRAY),
        (2, identity::SET),
    ] {
        if items(value, host, kind).is_some() {
            return rank;
        }
    }
    if entries(value, host).is_some() {
        return identity::DICTIONARY;
    }
    255
}

fn compare<C, H: FunctionHost<C>>(
    left: Node<'_, H::Value>,
    right: Node<'_, H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
) -> Result<std::cmp::Ordering, EvaluationError<H::Error>> {
    use crate::value::identity;
    use std::cmp::Ordering;
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let (Some(a), Some(b)) = (left.data(host), right.data(host)) {
            return Ok(identity::compare_admitted(a, b, budget, location)?);
        }
        let ordering = rank(left, host).cmp(&rank(right, host));
        if ordering != Ordering::Equal {
            return Ok(ordering);
        }
        if let (Some(a), Some(b)) = (text(left, host), text(right, host)) {
            budget.charge(a.len().min(b.len()), location.clone())?;
            return Ok(a.cmp(b));
        }
        for kind in 0..3 {
            if let Some(a) = items(left, host, kind) {
                let b = items(right, host, kind).expect("matching structural rank");
                let mut a_order = crate::value::metadata_vec(a.len(), budget, location)?;
                let mut b_order = crate::value::metadata_vec(b.len(), budget, location)?;
                for index in 0..a.len() {
                    a_order.push(a.get(index));
                }
                for index in 0..b.len() {
                    b_order.push(b.get(index));
                }
                if kind == 2 {
                    for order in [&mut a_order, &mut b_order] {
                        identity::sort(order, budget, location, |a, b, budget| {
                            compare(*a, *b, host, context, budget, location)
                        })?;
                    }
                }
                for (a, b) in a_order.iter().zip(&b_order) {
                    let ordering = compare(*a, *b, host, context, budget, location)?;
                    if ordering != Ordering::Equal {
                        return Ok(ordering);
                    }
                }
                return Ok(a.len().cmp(&b.len()));
            }
        }
        if let Some(a) = entries(left, host) {
            let b = entries(right, host).expect("matching dictionary rank");
            let mut a_order = crate::value::metadata_vec(a.len(), budget, location)?;
            let mut b_order = crate::value::metadata_vec(b.len(), budget, location)?;
            for index in 0..a.len() {
                a_order.push(a.get(index));
            }
            for index in 0..b.len() {
                b_order.push(b.get(index));
            }
            for order in [&mut a_order, &mut b_order] {
                identity::sort(order, budget, location, |a, b, budget| {
                    compare(a.0, b.0, host, context, budget, location)
                })?;
            }
            for ((ak, av), (bk, bv)) in a_order.iter().zip(&b_order) {
                let ordering = compare(*ak, *bk, host, context, budget, location)?;
                if ordering != Ordering::Equal {
                    return Ok(ordering);
                }
                let ordering = compare(*av, *bv, host, context, budget, location)?;
                if ordering != Ordering::Equal {
                    return Ok(ordering);
                }
            }
            return Ok(a.len().cmp(&b.len()));
        }
        let (Node::Host(left), Node::Host(right)) = (left, right) else {
            unreachable!("matching native leaf rank")
        };
        host.compare_native_values(left, right, context, budget, range(location))
    })
}

fn validate<C, H: FunctionHost<C>>(
    value: &H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), EvaluationError<H::Error>> {
    if let Some(data) = host.constructed_value(value) {
        return Ok(crate::admit_constructed(
            data,
            &DeclaredType::<()>::Value,
            budget,
            location.clone(),
        )?);
    }
    let kind = host.kind(value, budget, range(location))?;
    crate::admit_value(
        value,
        &DeclaredType::Native(kind),
        host,
        context,
        budget,
        location.clone(),
    )
}

/// Compare data structurally and admitted native leaves by their exact host identity.
/// Sets and dictionaries compare independently of insertion order.
///
/// # Errors
/// Unsupported native identities, stale handles and budget exhaustion propagate.
pub fn native_values_equal<C, H: FunctionHost<C>>(
    left: &H::Value,
    right: &H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<bool, EvaluationError<H::Error>> {
    validate(left, host, context, budget, &location)?;
    validate(right, host, context, budget, &location)?;
    equal(
        Node::Host(left),
        Node::Host(right),
        host,
        context,
        budget,
        &location,
    )
}

/// Hash a structural value using exact host identities for native leaves.
/// Equal values have equal hashes, independently of set/dictionary order.
///
/// # Errors
/// Invalid data, unsupported native identities and budget exhaustion propagate.
pub fn native_value_hash<C, H: FunctionHost<C>>(
    value: &H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<u64, EvaluationError<H::Error>> {
    validate(value, host, context, budget, &location)?;
    hash(Node::Host(value), host, context, budget, &location)
}

/// Compare ordinary values structurally, using the host's native leaf order.
/// Collection order is canonical for sets/dictionaries and positional for lists/tuples.
///
/// # Errors
/// Invalid values, unsupported cross-kind native orders and exhausted allowances fail.
pub fn native_values_compare<C, H: FunctionHost<C>>(
    left: &H::Value,
    right: &H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<std::cmp::Ordering, EvaluationError<H::Error>> {
    validate(left, host, context, budget, &location)?;
    validate(right, host, context, budget, &location)?;
    compare(
        Node::Host(left),
        Node::Host(right),
        host,
        context,
        budget,
        &location,
    )
}

/// Construct an insertion-ordered host set, retaining the first equal value.
///
/// # Errors
/// Unsupported host storage/identity, invalid values and budget exhaustion propagate.
pub fn construct_native_set<C, H: FunctionHost<C>>(
    values: Vec<H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<H::Value, EvaluationError<H::Error>> {
    crate::projection::admit_slots::<H::Value>(values.len(), budget, range(&location))?;
    let mut output = Vec::with_capacity(values.len());
    let mut membership = crate::value::identity::HashIndex::default();
    for value in values {
        let mut duplicate = false;
        let hashed = native_value_hash(&value, host, context, budget, location.clone())?;
        for &index in membership.candidates(hashed) {
            if native_values_equal(
                &value,
                &output[index],
                host,
                context,
                budget,
                location.clone(),
            )? {
                duplicate = true;
                break;
            }
        }
        if !duplicate {
            membership.insert(hashed, output.len(), budget, &location)?;
            output.push(value);
        }
    }
    host.construct_set(output, context, budget, range(&location))
}

/// Construct an insertion-ordered host dictionary with arbitrary value keys.
///
/// # Errors
/// Duplicate keys, unsupported host storage/identity and budget exhaustion propagate.
pub fn construct_native_dictionary<C, H: FunctionHost<C>>(
    entries: Vec<(H::Value, H::Value)>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<H::Value, EvaluationError<H::Error>> {
    let mut membership = crate::value::identity::HashIndex::default();
    for (index, (key, value)) in entries.iter().enumerate() {
        let hashed = native_value_hash(key, host, context, budget, location.clone())?;
        validate(value, host, context, budget, &location)?;
        for &previous in membership.candidates(hashed) {
            if native_values_equal(
                key,
                &entries[previous].0,
                host,
                context,
                budget,
                location.clone(),
            )? {
                return Err(QueryError::query(
                    range(&location).clone(),
                    "duplicate dictionary key",
                )
                .into());
            }
        }
        membership.insert(hashed, index, budget, &location)?;
    }
    host.construct_dictionary(entries, context, budget, range(&location))
}

/// Tally an eager sequence into an insertion-ordered host dictionary.
///
/// # Errors
/// Invalid identities, count overflow, unsupported host storage and budget exhaustion propagate.
pub fn tally_native_values<C, H: FunctionHost<C>>(
    values: Vec<H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<H::Value, EvaluationError<H::Error>> {
    crate::projection::admit_slots::<(H::Value, i64)>(values.len(), budget, range(&location))?;
    let mut counts: Vec<(H::Value, i64)> = Vec::with_capacity(values.len());
    let mut membership = crate::value::identity::HashIndex::default();
    for value in values {
        let hashed = native_value_hash(&value, host, context, budget, location.clone())?;
        let mut found = false;
        for &index in membership.candidates(hashed) {
            let (key, count) = &mut counts[index];
            if native_values_equal(&value, key, host, context, budget, location.clone())? {
                *count = count.checked_add(1).ok_or_else(|| {
                    QueryError::query(range(&location).clone(), "tally count overflow")
                })?;
                found = true;
                break;
            }
        }
        if !found {
            membership.insert(hashed, counts.len(), budget, &location)?;
            counts.push((value, 1));
        }
    }
    crate::projection::admit_slots::<(H::Value, H::Value)>(counts.len(), budget, range(&location))?;
    let mut output = Vec::with_capacity(counts.len());
    for (key, count) in counts {
        budget.charge_results(1, location.clone())?;
        let value = V::Integer(count);
        let facts = vec![crate::constructed_value_type(
            &value,
            budget,
            location.clone(),
        )?];
        output.push((
            key,
            host.lift_constructed(value, facts, context, budget, range(&location))?,
        ));
    }
    host.construct_dictionary(output, context, budget, range(&location))
}

/// Look up an arbitrary key in a host dictionary, copying the retained host value.
///
/// # Errors
/// Unsupported dictionary storage, native identities and budget exhaustion propagate.
pub fn lookup_native_dictionary<C, H: FunctionHost<C>>(
    dictionary: &H::Value,
    key: &H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<Option<H::Value>, EvaluationError<H::Error>> {
    let entries = host
        .dictionary_view(dictionary)
        .ok_or_else(|| QueryError::query(range(&location).clone(), "lookup needs a dictionary"))?;
    validate(dictionary, host, context, budget, &location)?;
    let hashed = native_value_hash(key, host, context, budget, location.clone())?;
    if let Some(position) =
        host.dictionary_position(dictionary, key, context, budget, range(&location))?
    {
        let Some(position) = position else {
            return Ok(None);
        };
        let (candidate, value) = entries.get(position).ok_or_else(|| {
            QueryError::query(
                range(&location).clone(),
                "native dictionary index position is out of bounds",
            )
        })?;
        if !native_values_equal(candidate, key, host, context, budget, location.clone())? {
            return Err(QueryError::query(
                range(&location).clone(),
                "native dictionary index returned a different key",
            )
            .into());
        }
        return Ok(Some(host.copy_value(
            value,
            context,
            budget,
            range(&location),
        )?));
    }
    let mut membership = crate::value::identity::HashIndex::default();
    for (index, (candidate, _)) in entries.iter().enumerate() {
        membership.insert(
            native_value_hash(candidate, host, context, budget, location.clone())?,
            index,
            budget,
            &location,
        )?;
    }
    for &index in membership.candidates(hashed) {
        let (candidate, value) = &entries[index];
        if native_values_equal(candidate, key, host, context, budget, location.clone())? {
            return Ok(Some(host.copy_value(
                value,
                context,
                budget,
                range(&location),
            )?));
        }
    }
    Ok(None)
}

/// Test ordinary native set membership, using a retained host index when supplied.
///
/// # Errors
/// Invalid values, malformed host index positions and exhausted allowances fail.
pub fn native_set_contains<C, H: FunctionHost<C>>(
    set: &H::Value,
    item: &H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<bool, EvaluationError<H::Error>> {
    let elements = host
        .set_view(set)
        .ok_or_else(|| QueryError::query(range(&location).clone(), "membership needs a set"))?;
    validate(set, host, context, budget, &location)?;
    let hashed = native_value_hash(item, host, context, budget, location.clone())?;
    if let Some(position) = host.set_position(set, item, context, budget, range(&location))? {
        let Some(position) = position else {
            return Ok(false);
        };
        let candidate = elements.get(position).ok_or_else(|| {
            QueryError::query(
                range(&location).clone(),
                "native set index position is out of bounds",
            )
        })?;
        if !native_values_equal(candidate, item, host, context, budget, location.clone())? {
            return Err(QueryError::query(
                range(&location).clone(),
                "native set index returned a different element",
            )
            .into());
        }
        return Ok(true);
    }
    let mut membership = crate::value::identity::HashIndex::default();
    for (index, element) in elements.iter().enumerate() {
        membership.insert(
            native_value_hash(element, host, context, budget, location.clone())?,
            index,
            budget,
            &location,
        )?;
    }
    for &index in membership.candidates(hashed) {
        if native_values_equal(
            item,
            &elements[index],
            host,
            context,
            budget,
            location.clone(),
        )? {
            return Ok(true);
        }
    }
    Ok(false)
}
