//! Shared collection operations over host-owned values. Native leaves keep their
//! host identity; collection structure is compared without rendering it.
use crate::{
    Budget, ConstructedValue as V, DeclaredType, EvaluationError, FunctionHost, Location,
    QueryError,
};

pub(crate) enum Node<'a, T> {
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
// Infix compatibility is separate from key identity. Keep only supplied
// declarations and borrowed values; do not infer a schema for every native row.
pub(crate) struct InfixOperand<'a, T, K> {
    pub node: Node<'a, T>,
    pub declaration: Option<&'a DeclaredType<K>>,
}

enum InfixDeclaration<'a, K> {
    Host(&'a DeclaredType<K>),
    Data(&'a DeclaredType<()>),
}
impl<K> Copy for InfixDeclaration<'_, K> {}
impl<K> Clone for InfixDeclaration<'_, K> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'a, K> InfixDeclaration<'a, K> {
    fn child(
        self,
        index: usize,
        label: Option<&str>,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Option<Self>, QueryError> {
        fn child<'a, K>(
            d: &'a DeclaredType<K>,
            index: usize,
            label: Option<&str>,
            budget: &mut Budget,
            location: &Location,
        ) -> Result<Option<&'a DeclaredType<K>>, QueryError> {
            budget.charge(1, location.clone())?;
            Ok(match crate::value::underlying(d) {
                DeclaredType::Array(d) | DeclaredType::Sequence(d) => Some(d),
                DeclaredType::Tuple(ds) => ds.get(index),
                DeclaredType::Record(ds) => Some(crate::value::record_declaration(
                    ds,
                    label.expect("record child label"),
                    budget,
                    location,
                )?),
                _ => None,
            })
        }
        match self {
            Self::Host(d) => child(d, index, label, budget, location).map(|d| d.map(Self::Host)),
            Self::Data(d) => child(d, index, label, budget, location).map(|d| d.map(Self::Data)),
        }
    }
    fn compatible(
        self,
        other: Self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<bool, QueryError>
    where
        K: PartialEq,
    {
        match (self, other) {
            (Self::Host(a), Self::Host(b)) => {
                crate::runtime_types::compatible_state(a, b, budget, location)
            }
            (Self::Data(a), Self::Data(b)) => {
                crate::runtime_types::compatible_declarations(a, b, budget, location)
            }
            (Self::Data(a), Self::Host(b)) | (Self::Host(b), Self::Data(a)) => {
                crate::runtime_types::compatible_declarations(a, b, budget, location)
            }
        }
    }
}

fn infix_fact<C, H: FunctionHost<C>>(
    node: Node<'_, H::Value>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<crate::ValueType, QueryError> {
    use crate::{ArrayType, ValueType as T};
    if let Some(data) = node.data(host) {
        return Ok(match data {
            V::Array(_) => T::Array(ArrayType::Positions(vec![])),
            V::Tuple(_) => T::TupleKind,
            V::Record(_) => T::RecordKind,
            V::Dictionary(_) => T::Dictionary {
                keys: vec![],
                values: vec![],
            },
            _ => crate::constructed_value_type(data, budget, location.clone())?,
        });
    }
    if text(node, host).is_some() {
        return Ok(T::String);
    }
    if items(node, host, 0).is_some() {
        return Ok(T::TupleKind);
    }
    if items(node, host, 1).is_some() {
        if let Node::Host(value) = node {
            let kind = host.kind(value, budget, range(location))?;
            if let Some(element) = host.positional_view_kind(&kind, budget, range(location))? {
                return Ok(T::Array(ArrayType::Elements(infix_kind_facts(
                    &element, host, budget, location,
                )?)));
            }
        }
        return Ok(T::Array(ArrayType::Positions(vec![])));
    }
    if let Some(values) = items(node, host, 2) {
        let mut family = None;
        for i in 0..values.len() {
            let value = values.get(i);
            let fact = match value.data(host) {
                Some(
                    data @ (V::Boolean(_)
                    | V::Integer(_)
                    | V::Float(_)
                    | V::String(_)
                    | V::Member(_)),
                ) => crate::constructed_value_type(data, budget, location.clone())?,
                _ if text(value, host).is_some() => T::String,
                _ => T::TupleKind, // A collection or opaque native leaf is a general member.
            };
            crate::value::merge_set_types(&mut family, &[fact], budget, location)?;
        }
        return Ok(T::Set(family));
    }
    if let Some(values) = entries(node, host) {
        return Ok(if matches!(values, Entries::Capture(_)) {
            T::RecordKind
        } else {
            T::Dictionary {
                keys: vec![],
                values: vec![],
            }
        });
    }
    Ok(T::Unestablished)
}

// These are supplied kind schemas, including an empty sequence's element;
// discovery must not depend on there being a row to inspect.
fn infix_kind_facts<C, H: FunctionHost<C>>(
    kind: &<H as crate::ProjectionHost<C>>::Kind,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<Vec<crate::ValueType>, QueryError> {
    use crate::{ArrayType, ValueType as T};
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let Some(facts) = host.scalar_types(kind, budget, range(location))? {
            return Ok(facts);
        }
        if let Some(children) = host.tuple_view_kind(kind) {
            let mut positions = crate::value::metadata_vec(1, budget, location)?;
            positions.push(vec![]);
            for child in children {
                let facts = infix_kind_facts(child, host, budget, location)?;
                positions =
                    crate::value::extend_type_positions(&positions, &facts, budget, location)?;
            }
            let mut facts = crate::value::metadata_vec(positions.len(), budget, location)?;
            for position in positions {
                facts.push(T::Tuple(position));
            }
            return Ok(facts);
        }
        if let Some(record) = host.capture_record_kind(kind) {
            let mut positions = crate::value::metadata_vec(1, budget, location)?;
            positions.push(vec![]);
            for (_, child) in &record.fields {
                let facts = infix_kind_facts(child, host, budget, location)?;
                positions =
                    crate::value::extend_type_positions(&positions, &facts, budget, location)?;
            }
            let mut facts = crate::value::metadata_vec(positions.len(), budget, location)?;
            for position in positions {
                let mut fields = crate::value::metadata_vec(record.fields.len(), budget, location)?;
                for ((label, _), fact) in record.fields.iter().zip(position) {
                    budget.charge(label.len(), location.clone())?;
                    fields.push((label.clone(), fact));
                }
                facts.push(T::Record(fields));
            }
            return Ok(facts);
        }
        let fact =
            if let Some(element) = host.positional_view_kind(kind, budget, range(location))? {
                T::Array(ArrayType::Elements(infix_kind_facts(
                    &element, host, budget, location,
                )?))
            } else {
                T::Unestablished
            };
        let mut facts = crate::value::metadata_vec(1, budget, location)?;
        facts.push(fact);
        Ok(facts)
    })
}

fn equality_refusal(location: &Location) -> QueryError {
    crate::value::error(location, "equality needs compatible underlying types")
}

fn admit_infix_contract<C, H: FunctionHost<C>, K>(
    node: Node<'_, H::Value>,
    declaration: &DeclaredType<K>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        if let Some(data) = node.data(host) {
            if matches!(data, V::Absent) {
                return Ok(());
            }
            return crate::admit_constructed(data, declaration, budget, location.clone());
        }
        let d = crate::value::underlying(declaration);
        match d {
            DeclaredType::Native(_) => Ok(()), // The host admits and identifies opaque leaves.
            DeclaredType::Array(child) | DeclaredType::Sequence(child) => {
                let values = items(node, host, 1).ok_or_else(|| equality_refusal(location))?;
                let actual = infix_fact(node, host, budget, location)?;
                crate::runtime_types::check_type_facts(&actual, d, true, budget, location)?;
                for i in 0..values.len() {
                    admit_infix_contract(values.get(i), child, host, budget, location)?;
                }
                Ok(())
            }
            DeclaredType::Tuple(children) => {
                let values = items(node, host, 0).ok_or_else(|| equality_refusal(location))?;
                if values.len() != children.len() {
                    return Err(equality_refusal(location));
                }
                for (i, child) in children.iter().enumerate() {
                    admit_infix_contract(values.get(i), child, host, budget, location)?;
                }
                Ok(())
            }
            DeclaredType::Record(fields) => {
                let values = entries(node, host).ok_or_else(|| equality_refusal(location))?;
                for (label, child) in fields {
                    let mut found = None;
                    for i in 0..values.len() {
                        budget.charge(1 + label.len(), location.clone())?;
                        let (key, value) = values.get(i);
                        if text(key, host) == Some(label.as_str()) {
                            found = Some(value);
                            break;
                        }
                    }
                    if let Some(value) = found {
                        admit_infix_contract(value, child, host, budget, location)?;
                    } else if !matches!(child, DeclaredType::Optional(_)) {
                        return Err(equality_refusal(location));
                    }
                }
                for i in 0..values.len() {
                    let (key, _) = values.get(i);
                    budget.charge(fields.len(), location.clone())?;
                    if !fields
                        .iter()
                        .any(|(label, _)| text(key, host) == Some(label.as_str()))
                    {
                        return Err(equality_refusal(location));
                    }
                }
                Ok(())
            }
            DeclaredType::Dictionary(key, value) => {
                let values = entries(node, host).ok_or_else(|| equality_refusal(location))?;
                for i in 0..values.len() {
                    let (k, v) = values.get(i);
                    admit_infix_contract(k, key, host, budget, location)?;
                    admit_infix_contract(v, value, host, budget, location)?;
                }
                Ok(())
            }
            DeclaredType::Set(family) => {
                let actual = infix_fact(node, host, budget, location)?;
                crate::value::binary_type(
                    crate::ValueBinaryOperator::Equal,
                    &actual,
                    &crate::ValueType::Set(Some(*family)),
                    budget,
                    location,
                )?;
                Ok(())
            }
            DeclaredType::String if text(node, host).is_some() => Ok(()),
            DeclaredType::Value => {
                for kind in 0..3 {
                    if let Some(values) = items(node, host, kind) {
                        for i in 0..values.len() {
                            admit_infix_contract(values.get(i), d, host, budget, location)?;
                        }
                        return Ok(());
                    }
                }
                if let Some(values) = entries(node, host) {
                    for i in 0..values.len() {
                        let (k, v) = values.get(i);
                        admit_infix_contract(k, d, host, budget, location)?;
                        admit_infix_contract(v, d, host, budget, location)?;
                    }
                    return Ok(());
                }
                if text(node, host).is_some() {
                    Ok(())
                } else {
                    Err(equality_refusal(location))
                }
            }
            _ => Err(equality_refusal(location)),
        }
    })
}

pub(crate) fn admit_infix_equality<C, H: FunctionHost<C>>(
    left: InfixOperand<'_, H::Value, <H as crate::ProjectionHost<C>>::Kind>,
    right: InfixOperand<'_, H::Value, <H as crate::ProjectionHost<C>>::Kind>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    admit_equality_pair(
        left.node,
        right.node,
        left.declaration.map(InfixDeclaration::Host),
        right.declaration.map(InfixDeclaration::Host),
        host,
        budget,
        location,
    )
}

fn admit_equality_pair<C, H: FunctionHost<C>>(
    left: Node<'_, H::Value>,
    right: Node<'_, H::Value>,
    left_d: Option<InfixDeclaration<'_, <H as crate::ProjectionHost<C>>::Kind>>,
    right_d: Option<InfixDeclaration<'_, <H as crate::ProjectionHost<C>>::Kind>>,
    host: &H,
    budget: &mut Budget,
    location: &Location,
) -> Result<(), QueryError> {
    budget.within_evaluation(1, location.clone(), |budget| {
        budget.charge(1, location.clone())?;
        fn declaration<'a, C, H: FunctionHost<C>>(
            node: Node<'a, H::Value>,
            host: &H,
        ) -> Option<&'a DeclaredType<()>> {
            match node {
                Node::Data(v) => crate::value::declaration(v),
                Node::Host(v) => host
                    .constructed_value(v)
                    .and_then(crate::value::declaration),
                Node::Text(_) => None,
            }
        }
        let left_fact = infix_fact(left, host, budget, location)?;
        let right_fact = infix_fact(right, host, budget, location)?;
        let ordinary_contract = |d, fact: &crate::ValueType| match d {
            InfixDeclaration::Host(d)
                if matches!(crate::value::underlying(d), DeclaredType::Native(_)) =>
            {
                matches!(
                    fact,
                    crate::ValueType::Absent | crate::ValueType::Unestablished
                )
            }
            _ => true,
        };
        let left_d = left_d
            .filter(|d| ordinary_contract(*d, &left_fact))
            .or_else(|| declaration(left, host).map(InfixDeclaration::Data));
        let right_d = right_d
            .filter(|d| ordinary_contract(*d, &right_fact))
            .or_else(|| declaration(right, host).map(InfixDeclaration::Data));
        crate::value::binary_type(
            crate::ValueBinaryOperator::Equal,
            &left_fact,
            &right_fact,
            budget,
            location,
        )?;
        match (left_d, right_d) {
            (Some(a), Some(b)) => {
                if !a.compatible(b, budget, location)? {
                    return Err(equality_refusal(location));
                }
            }
            (Some(d), None) | (None, Some(d)) => {
                let node = if left_d.is_some() { right } else { left };
                match d {
                    InfixDeclaration::Host(d) => {
                        admit_infix_contract(node, d, host, budget, location)?
                    }
                    InfixDeclaration::Data(d) => {
                        admit_infix_contract(node, d, host, budget, location)?
                    }
                }
            }
            _ => {}
        }
        if let (Some(a), Some(b)) = (left.data(host), right.data(host)) {
            let a = crate::constructed_value_type(a, budget, location.clone())?;
            let b = crate::constructed_value_type(b, budget, location.clone())?;
            crate::value::binary_type(crate::ValueBinaryOperator::Equal, &a, &b, budget, location)?;
            return Ok(());
        }
        for kind in 0..2 {
            if let (Some(a), Some(b)) = (items(left, host, kind), items(right, host, kind)) {
                if kind == 0 && a.len() != b.len() {
                    return Err(equality_refusal(location));
                }
                for i in 0..a.len().min(b.len()) {
                    admit_equality_pair(
                        a.get(i),
                        b.get(i),
                        left_d
                            .map(|d| d.child(i, None, budget, location))
                            .transpose()?
                            .flatten(),
                        right_d
                            .map(|d| d.child(i, None, budget, location))
                            .transpose()?
                            .flatten(),
                        host,
                        budget,
                        location,
                    )?;
                }
                // A retained element declaration also governs unmatched rows.
                for (values, fact) in [(&a, &right_fact), (&b, &left_fact)] {
                    if let crate::ValueType::Array(crate::ArrayType::Elements(facts)) = fact {
                        for fact in facts {
                            if let Some(d) =
                                crate::worklists::fact_contract::<()>(fact, budget, location)?
                            {
                                for i in 0..values.len() {
                                    admit_infix_contract(
                                        values.get(i),
                                        &d,
                                        host,
                                        budget,
                                        location,
                                    )?;
                                }
                            }
                        }
                    }
                }
                return Ok(());
            }
        }
        let records = |node| {
            matches!(node, Node::Host(v) if host.capture_record(v).is_some())
                || matches!(node.data(host), Some(V::Record(_)))
        };
        if records(left) && records(right) {
            let a = entries(left, host).expect("record view");
            let b = entries(right, host).expect("record view");
            if left_d.is_none() && right_d.is_none() && a.len() != b.len() {
                return Err(equality_refusal(location));
            }
            for i in 0..a.len() {
                let (key, value) = a.get(i);
                let label = text(key, host).expect("record label");
                let mut found = false;
                for j in 0..b.len() {
                    budget.charge(1 + label.len(), location.clone())?;
                    let (other, other_value) = b.get(j);
                    if text(other, host) == Some(label) {
                        admit_equality_pair(
                            value,
                            other_value,
                            left_d
                                .map(|d| d.child(i, Some(label), budget, location))
                                .transpose()?
                                .flatten(),
                            right_d
                                .map(|d| d.child(j, Some(label), budget, location))
                                .transpose()?
                                .flatten(),
                            host,
                            budget,
                            location,
                        )?;
                        found = true;
                        break;
                    }
                }
                if !found && left_d.is_none() && right_d.is_none() {
                    return Err(equality_refusal(location));
                }
            }
        }
        Ok(())
    })
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
        let left_data = match left {
            Node::Host(value) if host.constructed_value(value).is_none() => {
                host.ordinary_value(value, context, budget, range(location))?
            }
            _ => None,
        };
        let right_data = match right {
            Node::Host(value) if host.constructed_value(value).is_none() => {
                host.ordinary_value(value, context, budget, range(location))?
            }
            _ => None,
        };
        let left = left_data.as_ref().map_or(left, Node::Data);
        let right = right_data.as_ref().map_or(right, Node::Data);

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
        for (value, kind) in [(a, &a_kind), (b, &b_kind)] {
            crate::admit_value(
                value,
                &DeclaredType::Native(kind.clone()),
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
        let projected = match value {
            Node::Host(value) if host.constructed_value(value).is_none() => {
                host.ordinary_value(value, context, budget, range(location))?
            }
            _ => None,
        };
        let value = projected.as_ref().map_or(value, Node::Data);
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
        let left_data = match left {
            Node::Host(value) if host.constructed_value(value).is_none() => {
                host.ordinary_value(value, context, budget, range(location))?
            }
            _ => None,
        };
        let right_data = match right {
            Node::Host(value) if host.constructed_value(value).is_none() => {
                host.ordinary_value(value, context, budget, range(location))?
            }
            _ => None,
        };
        let left = left_data.as_ref().map_or(left, Node::Data);
        let right = right_data.as_ref().map_or(right, Node::Data);

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
    if let Some(data) = host.ordinary_value(value, context, budget, range(location))? {
        return Ok(crate::admit_constructed(
            &data,
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
    let output = unique_native_values(values, host, context, budget, location.clone())?;
    host.construct_set(output, context, budget, range(&location))
}

/// Retain first equal values in incoming order, preserving their native owners.
///
/// # Errors
/// Invalid values, unsupported identity and exhausted budgets propagate.
pub fn unique_native_values<C, H: FunctionHost<C>>(
    values: Vec<H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<H::Value>, EvaluationError<H::Error>> {
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
    Ok(output)
}

/// Execute full-program unique/tally using structural identity and actual keys.
///
/// # Errors
/// Invalid positional input, owners, declarations or budgets are refused.
pub fn evaluate_native_unique_tally<C, H: FunctionHost<C>>(
    operation: &crate::Stage<()>,
    input: H::Value,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    range: &std::ops::Range<usize>,
) -> Result<H::Value, EvaluationError<H::Error>> {
    let location = Location::Query(range.clone());
    let items = if let Some(values) = host.positional_view(&input) {
        validate(&input, host, context, budget, &location)?;
        crate::projection::admit_slots::<H::Value>(values.len(), budget, range)?;
        let mut copied = Vec::with_capacity(values.len());
        for value in values {
            copied.push(host.copy_value(value, context, budget, range)?);
        }
        copied
    } else {
        let kind = host.kind(&input, budget, range)?;
        host.collection_kind(operation, &kind, budget, range)?;
        host.selection_items(&input, context, budget, range)?
    };
    match operation {
        crate::Stage::Unique => {
            let items = unique_native_values(items, host, context, budget, location)?;
            host.restore_selection(
                input,
                items,
                crate::SelectionShape::Preserve,
                context,
                budget,
                range,
            )
        }
        crate::Stage::Tally => {
            let entries = tally_native_entries(items, host, context, budget, location)?;
            host.restore_dictionary(input, entries, context, budget, range)
        }
        _ => Err(QueryError::query(range.clone(), "expected unique or tally").into()),
    }
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
    let output = tally_native_entries(values, host, context, budget, location.clone())?;
    host.construct_dictionary(output, context, budget, range(&location))
}

#[allow(clippy::type_complexity)]
fn tally_native_entries<C, H: FunctionHost<C>>(
    values: Vec<H::Value>,
    host: &H,
    context: &mut C,
    budget: &mut Budget,
    location: Location,
) -> Result<Vec<(H::Value, H::Value)>, EvaluationError<H::Error>> {
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
    Ok(output)
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
