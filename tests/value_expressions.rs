use std::collections::BTreeMap;

use zirql::{
    ArrayType, Bindings, Budget, ConstructedValue as V, DeclaredType as D, KeyAxis, KeyedArrayType,
    KeyedEntry, Limits, Located, Location, NominalMember, NominalMemberId, NominalTypeId,
    ParameterDeclaration, ParameterInputs, ParameterType, ProgramLimits, SetElementType,
    ValueBinaryOperator as B, ValueConstructor, ValueExpression as E, ValueExpressionKind as K,
    ValueGenerator as G, ValueKey, ValueKeyType, ValueReducer as R, ValueType,
    ValueUnaryOperator as U, admit_constructed, admit_parameters, constructed_value_type,
    copy_constructed_value, evaluate_value, free_value_names, validate_value_expression,
    value_expression_types,
};

fn e(kind: K) -> E {
    Located::new(kind, 0..1)
}
fn literal(value: V) -> E {
    e(K::Literal(value))
}
fn int(value: i64) -> E {
    literal(V::Integer(value))
}
fn name(value: &str) -> E {
    e(K::Name(value.into()))
}
fn binary(operator: B, left: E, right: E) -> E {
    e(K::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    })
}
fn binder(value: &str) -> zirql::Pattern {
    Located::new(
        zirql::PatternKind::Name(Located::new(value.into(), 10..11)),
        10..11,
    )
}
fn clause(value: G) -> Located<G> {
    Located::new(value, 10..30)
}
fn field(value: E, label: &str) -> E {
    e(K::Field {
        value: Box::new(value),
        field: Located::new(label.into(), 5..6),
    })
}
fn access(array: E, index: E) -> E {
    e(K::ArrayAccess {
        array: Box::new(array),
        index: Box::new(index),
    })
}
fn tuple_access(value: E, position: usize) -> E {
    e(K::TupleAccess {
        value: Box::new(value),
        position: Located::new(position, 5..6),
    })
}
fn record(fields: Vec<(&str, E)>) -> E {
    e(K::Record(
        fields
            .into_iter()
            .map(|(label, value)| (Located::new(label.into(), 5..6), value))
            .collect(),
    ))
}
fn budget(work: usize, results: usize, nesting: usize) -> Budget {
    Budget::for_program(ProgramLimits {
        work,
        results,
        nesting,
        stream_items: 100,
    })
}
fn ample() -> Budget {
    budget(1_000_000, 100_000, 64)
}

#[test]
fn declared_report_contracts_check_actual_nested_values_without_changing_raw_values() {
    let contract: D<()> = D::Record(vec![
        (
            "rows".into(),
            D::Sequence(Box::new(D::Tuple(vec![D::Integer, D::String]))),
        ),
        ("cells".into(), D::Keyed(Box::new(D::Float))),
        ("flags".into(), D::Set(SetElementType::Boolean)),
    ]);
    // Stored label order is retained; it need not match declaration order.
    let report = V::Record(vec![
        (
            "flags".into(),
            V::Set {
                element_type: Some(SetElementType::Boolean),
                elements: vec![].into(),
            },
        ),
        (
            "cells".into(),
            V::KeyedArray {
                axes: vec![KeyAxis {
                    key_type: ValueKeyType::Integer,
                    keys: vec![ValueKey::Integer(4)],
                }],
                entries: vec![KeyedEntry {
                    key: vec![ValueKey::Integer(4)],
                    value: V::Float(1.5),
                }],
            },
        ),
        (
            "rows".into(),
            V::Array(vec![V::Tuple(vec![
                V::Integer(2),
                V::String("beta".into()),
            ])]),
        ),
    ]);
    let mut checking = budget(10_000, 0, 16);
    admit_constructed(&report, &contract, &mut checking, Location::Source(3..9)).unwrap();
    assert_eq!(checking.remaining_results(), Some(0));
    let V::Record(fields) = report else {
        panic!("record")
    };
    assert_eq!(fields[0].0, "flags");

    let heterogeneous = V::Record(vec![(
        "rows".into(),
        V::Array(vec![V::Integer(1), V::String("two".into())]),
    )]);
    constructed_value_type(&heterogeneous, &mut ample(), Location::Query(1..8)).unwrap();
    let expected: D<()> = D::Record(vec![("rows".into(), D::Sequence(Box::new(D::Integer)))]);
    let error = admit_constructed(
        &heterogeneous,
        &expected,
        &mut ample(),
        Location::Query(1..8),
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(1..8));
    assert_eq!(
        error.message,
        "field rows: sequence element 2: expected integer"
    );
    let duplicate: D<()> = D::Record(vec![("a".into(), D::Integer), ("a".into(), D::Integer)]);
    assert!(
        admit_constructed(
            &heterogeneous,
            &duplicate,
            &mut ample(),
            Location::Query(1..8)
        )
        .unwrap_err()
        .message
        .contains("duplicate record label")
    );
    assert_eq!(
        admit_constructed(
            &heterogeneous,
            &duplicate,
            &mut budget(3, 0, 16),
            Location::Query(1..8)
        )
        .unwrap_err()
        .category,
        zirql::QueryErrorCategory::WorkLimit
    );
}

#[test]
fn general_value_contracts_admit_nested_dictionary_keys_and_preserve_scalar_set_checks() {
    let location = Location::Source(4..10);
    let members = || V::Set {
        element_type: Some(SetElementType::Integer),
        elements: vec![V::Integer(2), V::Integer(1)].into(),
    };
    let mapping = V::Dictionary(
        vec![
            (
                V::Array(vec![V::Integer(1), V::String("two".into())]),
                V::Tuple(vec![members(), V::Record(vec![])]),
            ),
            (
                members(),
                V::Dictionary(vec![(V::Tuple(vec![V::Boolean(true)]), V::Absent)].into()),
            ),
        ]
        .into(),
    );
    let contract: D<()> = D::Dictionary(Box::new(D::Value), Box::new(D::Value));
    let declared =
        zirql::declare_constructed(mapping, contract.clone(), &mut ample(), location.clone())
            .unwrap();
    let copied = copy_constructed_value(&declared, &mut ample(), location.clone()).unwrap();
    admit_constructed(&copied, &contract, &mut ample(), location.clone()).unwrap();
    admit_constructed(&copied, &D::<()>::Value, &mut ample(), location.clone()).unwrap();
    assert_eq!(copied, declared);

    // Existing string-field declarations also satisfy a homogeneous dictionary contract.
    let record = zirql::declare_constructed(
        V::Record(vec![("count".into(), V::Integer(2))]),
        D::Record(vec![("count".into(), D::Integer)]),
        &mut ample(),
        location.clone(),
    )
    .unwrap();
    admit_constructed(
        &record,
        &D::<()>::Dictionary(Box::new(D::String), Box::new(D::Integer)),
        &mut ample(),
        location.clone(),
    )
    .unwrap();

    let general_set = D::<()>::Set(SetElementType::Value);
    admit_constructed(&members(), &general_set, &mut ample(), location.clone()).unwrap();
    admit_constructed(
        &V::Set {
            element_type: None,
            elements: vec![].into(),
        },
        &general_set,
        &mut ample(),
        location.clone(),
    )
    .unwrap();
    for elements in [
        vec![V::Integer(1), V::String("two".into())],
        vec![V::Array(vec![V::Integer(1)])],
    ] {
        let set = V::Set {
            element_type: Some(SetElementType::Value),
            elements: elements.into(),
        };
        admit_constructed(&set, &general_set, &mut ample(), location.clone()).unwrap();
        let error = admit_constructed(
            &set,
            &D::<()>::Set(SetElementType::Integer),
            &mut ample(),
            location.clone(),
        )
        .unwrap_err();
        assert_eq!(error.location, location);
        assert!(
            error
                .message
                .contains("expected set of declared element type")
        );
    }

    let duplicate = V::Dictionary(
        vec![
            (members(), V::Integer(1)),
            (
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(1), V::Integer(2)].into(),
                },
                V::Integer(2),
            ),
        ]
        .into(),
    );
    let error =
        admit_constructed(&duplicate, &contract, &mut ample(), location.clone()).unwrap_err();
    assert_eq!(error.location, location);
    assert!(error.message.contains("duplicate dictionary key"));
}

#[test]
fn structural_hash_and_order_ignore_unordered_storage_and_signed_zero() {
    use std::cmp::Ordering;
    use zirql::{compare_constructed_values, constructed_value_hash};
    let location = Location::Query(2..8);
    let make = |reversed: bool| {
        let mut members = vec![V::Integer(1), V::Integer(2)];
        let mut fields = vec![
            ("first".into(), V::Integer(1)),
            ("second".into(), V::Integer(2)),
        ];
        if reversed {
            members.reverse();
            fields.reverse();
        }
        let record = if reversed {
            V::Dictionary(
                fields
                    .into_iter()
                    .map(|(key, value)| (V::String(key), value))
                    .collect::<Vec<_>>()
                    .into(),
            )
        } else {
            V::Record(fields)
        };
        let mut entries = vec![
            (
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: members.into(),
                },
                V::Array(vec![V::Tuple(vec![V::Float(if reversed {
                    -0.0
                } else {
                    0.0
                })])]),
            ),
            (V::String("record".into()), record),
        ];
        if reversed {
            entries.reverse();
        }
        V::Dictionary(entries.into())
    };
    let left = make(false);
    let right = make(true);
    assert_eq!(
        constructed_value_hash(&left, &mut ample(), location.clone()).unwrap(),
        constructed_value_hash(&right, &mut ample(), location.clone()).unwrap(),
    );
    assert_eq!(
        compare_constructed_values(&left, &right, &mut ample(), location.clone()).unwrap(),
        Ordering::Equal,
    );
    let declared =
        zirql::declare_constructed(right, D::Value, &mut ample(), location.clone()).unwrap();
    assert_eq!(
        constructed_value_hash(&left, &mut ample(), location.clone()).unwrap(),
        constructed_value_hash(&declared, &mut ample(), location.clone()).unwrap(),
    );
    assert_eq!(
        compare_constructed_values(
            &V::Array(vec![V::Integer(1)]),
            &V::Tuple(vec![V::Integer(1)]),
            &mut ample(),
            location.clone()
        )
        .unwrap(),
        Ordering::Less,
    );
    assert_eq!(
        compare_constructed_values(
            &V::Integer(1),
            &V::Float(1.0),
            &mut ample(),
            location.clone()
        )
        .unwrap(),
        Ordering::Less,
    );
    let mut membership = binary(
        B::In,
        int(1),
        literal(V::Set {
            element_type: Some(SetElementType::Integer),
            elements: vec![V::Integer(1)].into(),
        }),
    );
    assert_eq!(evaluate(&membership).unwrap(), V::Boolean(true));
    let K::Binary { right, .. } = &mut membership.value else {
        unreachable!()
    };
    let K::Literal(V::Set { elements, .. }) = &mut right.value else {
        unreachable!()
    };
    elements[0] = V::Integer(2);
    assert_eq!(evaluate(&membership).unwrap(), V::Boolean(false));

    let a = member(1, 0);
    let mut b = member(1, 0);
    b.member_name = "contradiction".into();
    assert!(
        compare_constructed_values(&V::Member(a), &V::Member(b), &mut ample(), location)
            .unwrap_err()
            .message
            .contains("inconsistent nominal member")
    );
}

#[test]
fn declared_and_deferred_dictionaries_keep_general_bracket_access() {
    let dictionary = || {
        declared(
            V::Dictionary(vec![(V::Integer(1), V::String("one".into()))].into()),
            D::Dictionary(Box::new(D::Integer), Box::new(D::String)),
        )
    };
    assert_eq!(
        evaluate(&access(literal(dictionary()), int(1)))
            .unwrap()
            .data(),
        &V::String("one".into())
    );
    let get = function(
        "get",
        vec![(
            "d",
            D::Dictionary(Box::new(D::Integer), Box::new(D::String)),
        )],
        D::String,
        access(name("d"), int(1)),
    );
    assert_eq!(
        run_functions(
            &[get],
            &call("get", vec![literal(dictionary())]),
            2,
            &mut ample()
        )
        .unwrap()
        .data(),
        &V::String("one".into())
    );
    let key = || e(K::Set(vec![int(1)]));
    let get = function(
        "get",
        vec![("d", D::Value)],
        D::Value,
        access(name("d"), key()),
    );
    let entry = call("get", vec![e(K::Dictionary(vec![(key(), text("one"))]))]);
    assert_eq!(
        run_functions(&[get], &entry, 2, &mut ample())
            .unwrap()
            .data(),
        &V::String("one".into())
    );
}

#[test]
fn common_signatures_read_value_and_general_collection_contracts() {
    let source = "fn identity(items: set<value>, mapping: dict<value, value>, item: value) -> value { item }";
    let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
    let mut host = WordInput {
        value: V::Array(vec![]),
        unknown: false,
    };
    let declaration = zirql::read_function_declaration(
        &mut reader,
        &mut host,
        &mut ample(),
        64,
        |name, _| -> Result<D<()>, zirql::QueryError> {
            Err(zirql::QueryError::query(name.range, "unexpected host type"))
        },
    )
    .unwrap()
    .unwrap();
    assert!(reader.remaining().trim().is_empty());
    assert_eq!(
        declaration.parameters[0].value.declaration,
        D::Set(SetElementType::Value)
    );
    assert_eq!(
        declaration.parameters[1].value.declaration,
        D::Dictionary(Box::new(D::Value), Box::new(D::Value)),
    );
    assert_eq!(declaration.parameters[2].value.declaration, D::Value);
    assert_eq!(declaration.result.value, D::Value);
    zirql::dispose_native_value(declaration.value);

    let mut allowance = ample();
    let expression = read_word_expression("set<value>()", &mut host, &mut allowance);
    let empty =
        zirql::evaluate_query_value(&expression, &BTreeMap::new(), &mut host, &mut allowance)
            .unwrap();
    zirql::dispose_native_value(expression);
    assert_eq!(
        empty,
        V::Set {
            element_type: Some(SetElementType::Value),
            elements: vec![].into()
        }
    );
}

#[test]
fn declared_nominal_types_use_identity_and_keep_empty_set_types() {
    let value = V::Member(member(1, 0));
    admit_constructed(
        &value,
        &D::<()>::Member(NominalTypeId(7)),
        &mut ample(),
        Location::Source(4..10),
    )
    .unwrap();
    let V::Member(mut other) = value else {
        unreachable!()
    };
    other.type_id = NominalTypeId(8);
    let error = admit_constructed(
        &V::Member(other),
        &D::<()>::Member(NominalTypeId(7)),
        &mut ample(),
        Location::Source(4..10),
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Source(4..10));
    assert_eq!(error.message, "expected member of declared nominal type");
    let empty = V::Set {
        element_type: Some(SetElementType::Member(NominalTypeId(8))),
        elements: vec![].into(),
    };
    assert!(
        admit_constructed(
            &empty,
            &D::<()>::Set(SetElementType::Member(NominalTypeId(7))),
            &mut ample(),
            Location::Query(0..1)
        )
        .is_err()
    );
}
fn evaluate(expression: &E) -> Result<V, zirql::QueryError> {
    evaluate_value(expression, &Bindings::default(), &mut ample())
}
fn choose(skipped: E) -> E {
    e(K::Conditional {
        branches: vec![(literal(V::Boolean(false)), skipped)],
        otherwise: Box::new(int(7)),
    })
}

fn parameter(value: &str, ty: ParameterType, start: usize) -> ParameterDeclaration {
    ParameterDeclaration {
        name: Located::new(value.into(), start..start + value.len()),
        ty: Located::new(ty, start + value.len() + 2..start + value.len() + 5),
    }
}

#[test]
fn typed_and_cli_parameters_use_immutable_values_without_source_interpolation() {
    let declarations = [
        parameter("factor", ParameterType::Int, 6),
        parameter("label", ParameterType::String, 28),
        parameter("enabled", ParameterType::Bool, 50),
    ];
    let data = " \"quoted\"\\path=x; report 999; ";
    let typed = [
        ("enabled".into(), V::Boolean(true)),
        ("label".into(), V::String(data.into())),
        ("factor".into(), V::Integer(2)),
    ];
    let cli = [
        "factor=+2".into(),
        "enabled=true".into(),
        format!("label={data}"),
    ];
    let expression = record(vec![
        ("label", name("label")),
        (
            "scaled",
            e(K::Conditional {
                branches: vec![(name("enabled"), binary(B::Multiply, name("factor"), int(3)))],
                otherwise: Box::new(int(0)),
            }),
        ),
    ]);
    let types = declarations
        .iter()
        .map(|declaration| {
            (
                declaration.name.value.clone(),
                ValueType::from(declaration.ty.value),
            )
        })
        .collect();
    free_value_names(&expression, &types, &mut ample()).unwrap();
    for inputs in [ParameterInputs::Typed(&typed), ParameterInputs::Cli(&cli)] {
        let mut allowance = ample();
        let admitted = admit_parameters(&declarations, inputs, &[], 0..80, &mut allowance).unwrap();
        assert_eq!(
            admitted.iter().map(|(name, _)| name).collect::<Vec<_>>(),
            declarations
                .iter()
                .map(|declaration| &declaration.name)
                .collect::<Vec<_>>()
        );
        let bindings = Bindings::new(admitted).unwrap();
        assert_eq!(
            evaluate_value(&expression, &bindings, &mut allowance).unwrap(),
            V::Record(vec![
                ("label".into(), V::String(data.into())),
                ("scaled".into(), V::Integer(6))
            ])
        );
        let shadowing = e(K::Comprehension {
            constructor: ValueConstructor::Array,
            clauses: vec![clause(G::In {
                case: false,
                binders: vec![binder("factor")],
                source: e(K::Array(vec![])),
                filter: None,
            })],
            head: Box::new(name("factor")),
        });
        assert!(evaluate_value(&shadowing, &bindings, &mut allowance).is_err());
        assert!(allowance.remaining_work() < ample().remaining_work());
    }
    let admitted = |declarations: &[ParameterDeclaration], inputs: ParameterInputs<'_>| {
        admit_parameters(declarations, inputs, &[], 0..80, &mut ample()).unwrap()
    };
    assert_eq!(
        admitted(
            &declarations[..1],
            ParameterInputs::Cli(&["factor=-9223372036854775808".into()])
        )[0]
        .1,
        V::Integer(i64::MIN)
    );
    assert_eq!(
        admitted(
            &declarations[..1],
            ParameterInputs::Cli(&["factor=9223372036854775807".into()])
        )[0]
        .1,
        V::Integer(i64::MAX)
    );
    assert_eq!(
        admitted(
            &declarations[1..2],
            ParameterInputs::Cli(&["label=".into()])
        )[0]
        .1,
        V::String(String::new())
    );
}

#[test]
fn parameter_admission_refuses_incomplete_sets_wrong_types_and_exhausted_budgets() {
    let declaration = parameter("factor", ParameterType::Int, 6);
    let declarations = std::slice::from_ref(&declaration);
    let refuse = |inputs: ParameterInputs<'_>| {
        admit_parameters(declarations, inputs, &[], 0..40, &mut ample()).unwrap_err()
    };
    let missing = refuse(ParameterInputs::Typed(&[]));
    assert_eq!(
        missing.location,
        Location::Query(declaration.name.range.clone())
    );
    assert!(missing.message.contains("factor"));
    for values in [
        vec![
            ("factor".into(), V::Integer(2)),
            ("factor".into(), V::Integer(3)),
        ],
        vec![("other".into(), V::Integer(2))],
    ] {
        assert_eq!(
            refuse(ParameterInputs::Typed(&values)).location,
            Location::Query(0..40)
        );
    }
    let wrong = [("factor".into(), V::Boolean(true))];
    assert_eq!(
        refuse(ParameterInputs::Typed(&wrong)).location,
        Location::Query(declaration.ty.range.clone())
    );
    for spelling in ["0x2", "2_000", " 2", "2.0", "9223372036854775808", "+"] {
        assert_eq!(
            refuse(ParameterInputs::Cli(&[format!("factor={spelling}")])).location,
            Location::Query(declaration.ty.range.clone())
        );
    }
    for arguments in [
        vec!["factor".into()],
        vec!["=2".into()],
        vec!["other=2".into()],
        vec!["factor=2".into(), "factor=3".into()],
    ] {
        assert_eq!(
            refuse(ParameterInputs::Cli(&arguments)).location,
            Location::Query(0..40)
        );
    }
    let boolean = parameter("enabled", ParameterType::Bool, 50);
    let error = admit_parameters(
        std::slice::from_ref(&boolean),
        ParameterInputs::Cli(&["enabled=True".into()]),
        &[],
        0..80,
        &mut ample(),
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(boolean.ty.range));
    assert!(
        admit_parameters(
            &[declaration.clone(), declaration.clone()],
            ParameterInputs::Typed(&[]),
            &[],
            0..40,
            &mut ample()
        )
        .is_err()
    );
    assert!(
        admit_parameters(
            declarations,
            ParameterInputs::Typed(&[]),
            &["factor"],
            0..40,
            &mut ample()
        )
        .is_err()
    );

    let typed = [("factor".into(), V::Integer(2))];
    for (mut low, message) in [
        (budget(1_000, 0, 8), "result limit"),
        (budget(1, 100, 8), "work limit"),
    ] {
        let error = admit_parameters(
            declarations,
            ParameterInputs::Typed(&typed),
            &[],
            0..40,
            &mut low,
        )
        .unwrap_err();
        assert!(error.message.contains(message));
    }
    assert!(
        admit_parameters(
            &[],
            ParameterInputs::Typed(&[]),
            &[],
            0..0,
            &mut budget(0, 0, 0)
        )
        .unwrap()
        .is_empty()
    );
    let mut allowance = ample();
    let admitted = admit_parameters(
        declarations,
        ParameterInputs::Typed(&typed),
        &[],
        0..40,
        &mut allowance,
    )
    .unwrap();
    let remaining = allowance.remaining_work();
    allowance
        .charge(remaining, Location::Query(40..40))
        .unwrap();
    let error = evaluate_value(
        &name("factor"),
        &Bindings::new(admitted).unwrap(),
        &mut allowance,
    )
    .unwrap_err();
    assert!(error.message.contains("work limit"));
}

#[test]
fn positional_report_preserves_scopes_order_and_budgeted_host_sequence() {
    let labels = V::Array(vec![V::String("beta".into()), V::String("alpha".into())]);
    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![
            clause(G::In {
                case: false,
                binders: vec![binder("p")],
                source: e(K::IndexSet(Box::new(Located::new(
                    K::Name("labels".into()),
                    20..26,
                )))),
                filter: None,
            }),
            clause(G::In {
                case: false,
                binders: vec![
                    pattern(zirql::PatternKind::Tuple(vec![
                        pattern_name("i"),
                        pattern(zirql::PatternKind::Wildcard),
                    ])),
                    pattern(zirql::PatternKind::Tuple(vec![
                        pattern_name("j"),
                        pattern(zirql::PatternKind::Wildcard),
                    ])),
                ],
                source: e(K::Array(vec![
                    e(K::Tuple(vec![int(2), int(20)])),
                    e(K::Tuple(vec![int(1), int(10)])),
                ])),
                filter: Some(binary(B::NotEqual, name("i"), name("j"))),
            }),
            clause(G::In {
                case: false,
                binders: vec![binder("k")],
                source: e(K::Range {
                    lower: Box::new(int(1)),
                    upper: Box::new(name("p")),
                }),
                filter: Some(binary(B::Equal, name("k"), name("p"))),
            }),
            clause(G::Bind {
                binder: record_pattern(
                    vec![
                        ("label", pattern_name("row_label")),
                        ("triple", pattern_name("triple")),
                    ],
                    false,
                ),
                value: record(vec![
                    ("label", access(name("labels"), name("p"))),
                    ("triple", e(K::Tuple(vec![name("i"), name("j"), name("k")]))),
                ]),
                filter: Some(binary(B::GreaterEqual, name("p"), int(1))),
            }),
            clause(G::In {
                case: false,
                binders: vec![Located::new(zirql::PatternKind::Wildcard, 12..13)],
                source: e(K::Array(vec![literal(V::Boolean(true))])),
                filter: None,
            }),
        ],
        head: Box::new(record(vec![
            ("name", name("row_label")),
            (
                "pair",
                e(K::Tuple(vec![
                    tuple_access(name("triple"), 1),
                    tuple_access(name("triple"), 2),
                ])),
            ),
            ("position", tuple_access(name("triple"), 3)),
        ])),
    });
    let declared = BTreeMap::from([(
        "labels".into(),
        ValueType::Array(ArrayType::Elements(vec![ValueType::String])),
    )]);
    let mut allowance = ample();
    assert_eq!(
        free_value_names(&expression, &declared, &mut allowance).unwrap(),
        vec![Located::new("labels".into(), 20..26)]
    );
    let ty = constructed_value_type(&labels, &mut allowance, Location::Source(2..20)).unwrap();
    validate_value_expression(
        &expression,
        &BTreeMap::from([("labels".into(), ty)]),
        &mut allowance,
    )
    .unwrap();
    let bindings = Bindings::new([(Located::new("labels".into(), 0..6), labels)]).unwrap();
    let result = evaluate_value(&expression, &bindings, &mut allowance).unwrap();
    let rows = |label: &str, position| {
        [vec![2, 1], vec![1, 2]]
            .into_iter()
            .map(|pair| {
                V::Record(vec![
                    ("name".into(), V::String(label.into())),
                    (
                        "pair".into(),
                        V::Tuple(pair.into_iter().map(V::Integer).collect()),
                    ),
                    ("position".into(), V::Integer(position)),
                ])
            })
            .collect::<Vec<_>>()
    };
    let mut expected = rows("beta", 1);
    expected.extend(rows("alpha", 2));
    assert_eq!(result, V::Array(expected));
    let before = allowance.remaining_results().unwrap();
    let snapshot =
        copy_constructed_value(&result, &mut allowance, Location::Query(40..50)).unwrap();
    assert_eq!(snapshot, result);
    assert!(allowance.remaining_results().unwrap() < before);
    assert!(allowance.remaining_work() < 1_000_000);

    let location = Location::Source(2..20);
    let mut shallow = budget(10_000, 10_000, 1);
    assert_eq!(
        copy_constructed_value(&result, &mut shallow, location.clone())
            .unwrap_err()
            .location,
        location
    );
    assert!(shallow.remaining_work() < 10_000);
    assert_eq!(
        copy_constructed_value(&V::Integer(9), &mut shallow, Location::Query(0..1)).unwrap(),
        V::Integer(9)
    );
    let mut low = budget(2, 100, 64);
    assert!(constructed_value_type(&result, &mut low, Location::Query(40..50)).is_err());
    let mut legacy = Budget::new(Limits {
        work: 1,
        items: 100,
        nesting: 64,
    });
    assert!(
        copy_constructed_value(
            &V::String("beta".into()),
            &mut legacy,
            Location::Query(0..1)
        )
        .is_err()
    );
}

#[test]
fn scope_refusals_are_located_before_generator_execution() {
    let unavailable = Located::new(K::NativeQuery(Box::new(())), 40..45);
    assert_eq!(
        evaluate(&unavailable).unwrap_err().location,
        Location::Query(40..45)
    );

    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("x")],
            source: Located::new(K::Name("x".into()), 30..31),
            filter: None,
        })],
        head: Box::new(name("x")),
    });
    assert_eq!(
        evaluate(&expression).unwrap_err().location,
        Location::Query(30..31)
    );
    let shadow = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![Located::new(
                zirql::PatternKind::Name(Located::new("existing".into(), 11..19)),
                11..19,
            )],
            source: e(K::Array(vec![])),
            filter: None,
        })],
        head: Box::new(int(0)),
    });
    let bindings = Bindings::new([(Located::new("existing".into(), 0..8), V::Integer(1))]).unwrap();
    assert_eq!(
        evaluate_value(&shadow, &bindings, &mut ample())
            .unwrap_err()
            .location,
        Location::Query(11..19)
    );
    assert!(
        evaluate(&name("_"))
            .unwrap_err()
            .message
            .contains("discard")
    );
    let undefined = Located::new(K::Name("x".repeat(256)), 35..36);
    let mut legacy = Budget::new(Limits {
        work: 8,
        items: 10,
        nesting: 16,
    });
    let refusal = free_value_names(&undefined, &BTreeMap::new(), &mut legacy).unwrap_err();
    assert_eq!(refusal.location, Location::Query(35..36));
    assert!(refusal.message.contains("work limit"));
    assert!(legacy.remaining_work() < 8);
    let long_binder = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::Bind {
            binder: Located::new(
                zirql::PatternKind::Name(Located::new("x".repeat(256), 37..38)),
                37..38,
            ),
            value: int(1),
            filter: None,
        })],
        head: Box::new(int(1)),
    });
    let mut legacy = Budget::new(Limits {
        work: 100,
        items: 10,
        nesting: 16,
    });
    let refusal = evaluate_value(&long_binder, &Bindings::default(), &mut legacy).unwrap_err();
    assert_eq!(refusal.location, Location::Query(37..38));
    assert!(refusal.message.contains("work limit"));
}

#[test]
fn skipped_runtime_failures_still_receive_bounded_static_validation() {
    let missing_field = Located::new(
        K::Field {
            value: Box::new(record(vec![])),
            field: Located::new("x".repeat(256), 39..40),
        },
        39..41,
    );
    let mut legacy = Budget::new(Limits {
        work: 100,
        items: 10,
        nesting: 16,
    });
    let refusal = evaluate_value(&missing_field, &Bindings::default(), &mut legacy).unwrap_err();
    assert_eq!(refusal.location, Location::Query(39..41));
    assert!(refusal.message.contains("work limit"));
    assert_eq!(
        evaluate(&choose(binary(B::IntegerDivide, int(1), int(0)))).unwrap(),
        V::Integer(7)
    );
    let missing = access(e(K::Array(vec![literal(V::String("x".into()))])), int(0));
    assert_eq!(
        evaluate(&choose(binary(B::Add, missing, int(1)))).unwrap(),
        V::Integer(7)
    );
    validate_value_expression(
        &choose(binary(B::Add, access(name("labels"), int(0)), int(1))),
        &BTreeMap::from([(
            "labels".into(),
            ValueType::Array(ArrayType::Elements(vec![ValueType::String])),
        )]),
        &mut ample(),
    )
    .unwrap();
    assert_eq!(
        evaluate(&choose(field(record(vec![]), "absent"))).unwrap(),
        V::Integer(7)
    );
    let unsupported = Located::new(K::Set(vec![e(K::Tuple(vec![int(1)]))]), 50..55);
    assert_eq!(evaluate(&choose(unsupported)).unwrap(), V::Integer(7));
    let incompatible = Located::new(
        K::Binary {
            operator: B::Add,
            left: Box::new(literal(V::String("x".into()))),
            right: Box::new(int(1)),
        },
        60..66,
    );
    assert_eq!(
        evaluate(&choose(incompatible)).unwrap_err().location,
        Location::Query(60..66)
    );
    // Actual source discovery retains both known heterogeneous possibilities.
    let heterogeneous = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("n")],
            source: name("mixed"),
            filter: Some(literal(V::Boolean(false))),
        })],
        head: Box::new(Located::new(
            K::Binary {
                operator: B::Add,
                left: Box::new(name("n")),
                right: Box::new(int(1)),
            },
            70..75,
        )),
    });
    let mixed = Bindings::new([(
        Located::new("mixed".into(), 0..5),
        V::Array(vec![V::Integer(1), V::String("x".into())]),
    )])
    .unwrap();
    assert_eq!(
        evaluate_value(&heterogeneous, &mixed, &mut ample())
            .unwrap_err()
            .location,
        Location::Query(70..75)
    );
    assert_eq!(
        evaluate(&access(
            e(K::Array(vec![int(1), literal(V::String("x".into()))])),
            int(1)
        ))
        .unwrap(),
        V::Integer(1)
    );
    let skipped = choose(Located::new(K::Array(vec![int(1), int(2), int(3)]), 80..89));
    let mut limited = budget(7, 100, 64);
    let refusal = validate_value_expression(&skipped, &BTreeMap::new(), &mut limited).unwrap_err();
    assert!(refusal.message.contains("work limit"));
    assert_eq!(refusal.location, Location::Query(80..89));
    assert!(limited.remaining_work() < 7);
    let mut nesting = budget(10_000, 1_000, 1);
    assert!(
        validate_value_expression(&skipped, &BTreeMap::new(), &mut nesting)
            .unwrap_err()
            .message
            .contains("nesting")
    );
    validate_value_expression(&e(K::Array(vec![])), &BTreeMap::new(), &mut nesting).unwrap();
    assert_eq!(
        evaluate(&binary(
            B::Or,
            literal(V::Boolean(true)),
            binary(B::Equal, binary(B::IntegerDivide, int(1), int(0)), int(0))
        ))
        .unwrap(),
        V::Boolean(true)
    );
}

#[test]
fn checked_numbers_and_typed_values_keep_responsible_failures() {
    assert_eq!(
        evaluate(&binary(B::IntegerDivide, int(-7), int(3))).unwrap(),
        V::Integer(-2)
    );
    assert_eq!(
        evaluate(&binary(B::Modulo, int(-7), int(3))).unwrap(),
        V::Integer(-1)
    );
    for expression in [
        binary(B::Add, int(i64::MAX), int(1)),
        binary(B::IntegerDivide, int(1), int(0)),
        binary(B::Modulo, int(i64::MIN), int(-1)),
        e(K::Unary {
            operator: U::Negative,
            operand: Box::new(int(i64::MIN)),
        }),
        binary(B::Add, int(i64::MAX), literal(V::Float(1.0))),
        binary(
            B::Multiply,
            literal(V::Float(f64::from_bits(1))),
            literal(V::Float(0.5)),
        ),
    ] {
        let expression = Located::new(expression.value, 90..99);
        assert_eq!(
            evaluate(&expression).unwrap_err().location,
            Location::Query(90..99)
        );
    }
    assert_eq!(
        evaluate(&binary(B::Add, int(i64::MIN), literal(V::Float(0.0)))).unwrap(),
        V::Float(i64::MIN as f64)
    );
    assert_eq!(
        evaluate(&binary(
            B::Subtract,
            literal(V::Float(f64::from_bits(1))),
            literal(V::Float(f64::from_bits(1)))
        ))
        .unwrap(),
        V::Float(0.0)
    );
    assert!(evaluate(&choose(literal(V::Float(f64::INFINITY)))).is_err());
    assert_eq!(
        evaluate(&e(K::Set(vec![int(2), int(1), int(2)]))).unwrap(),
        V::Set {
            element_type: Some(SetElementType::Integer),
            elements: vec![V::Integer(2), V::Integer(1)].into()
        }
    );
    assert_eq!(
        evaluate(&binary(
            B::In,
            int(2),
            e(K::Range {
                lower: Box::new(int(1)),
                upper: Box::new(int(3))
            })
        ))
        .unwrap(),
        V::Boolean(true)
    );
    assert_eq!(
        evaluate(&e(K::Range {
            lower: Box::new(int(2)),
            upper: Box::new(int(1))
        }))
        .unwrap(),
        V::Set {
            element_type: Some(SetElementType::Integer),
            elements: vec![].into()
        }
    );
    assert_eq!(
        evaluate(&binary(
            B::Equal,
            record(vec![
                ("a", int(1)),
                ("b", e(K::Tuple(vec![literal(V::String("x".into()))])))
            ]),
            record(vec![
                ("b", e(K::Tuple(vec![literal(V::String("x".into()))]))),
                ("a", int(1))
            ])
        ))
        .unwrap(),
        V::Boolean(true)
    );
    assert_eq!(
        evaluate(&binary(
            B::Concatenate,
            literal(V::String("data:".into())),
            literal(V::String("beta".into()))
        ))
        .unwrap(),
        V::String("data:beta".into())
    );
}

#[test]
fn grouped_candidates_are_admitted_before_a_false_filter() {
    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![Located::new(
            G::In {
                case: false,
                binders: vec![binder("i"), binder("j")],
                source: e(K::Range {
                    lower: Box::new(int(1)),
                    upper: Box::new(int(100)),
                }),
                filter: Some(literal(V::Boolean(false))),
            },
            100..120,
        )],
        head: Box::new(int(1)),
    });
    let mut allowance = budget(1_000, 10_000, 64);
    let refusal = evaluate_value(&expression, &Bindings::default(), &mut allowance).unwrap_err();
    assert_eq!(refusal.location, Location::Query(100..120));
    assert!(refusal.message.contains("work limit"));
    assert!(allowance.remaining_work() < 1_000);
    assert_eq!(
        evaluate_value(&int(1), &Bindings::default(), &mut allowance).unwrap(),
        V::Integer(1)
    );
}

fn member(id: u64, order: usize) -> NominalMember {
    NominalMember {
        type_id: NominalTypeId(7),
        member_id: NominalMemberId(id),
        declaration_order: order,
        type_name: "Guests".into(),
        member_name: "same".into(),
    }
}
fn member_expression(id: u64, order: usize) -> E {
    literal(V::Member(member(id, order)))
}

fn grouping(rows: E, label: &str) -> E {
    e(K::GroupBy {
        rows: Box::new(rows),
        field: Located::new(label.into(), 5..6),
    })
}

#[test]
fn grouping_preserves_first_seen_keys_all_rows_and_the_public_value_sequence() {
    let row = |machine: &str, demand| {
        V::Record(vec![
            ("machine".into(), V::String(machine.into())),
            ("demand".into(), V::Integer(demand)),
        ])
    };
    let rows = V::Array(vec![row("b", 3), row("a", 5), row("b", 7), row("b", 3)]);
    let expression = grouping(name("rows"), "machine");
    let mut allowance = ample();
    let row_types =
        constructed_value_type(&rows, &mut allowance, Location::Source(10..30)).unwrap();
    let types = BTreeMap::from([("rows".into(), row_types)]);
    assert_eq!(
        free_value_names(&expression, &types, &mut allowance).unwrap(),
        vec![Located::new("rows".into(), 0..1)]
    );
    validate_value_expression(&expression, &types, &mut allowance).unwrap();
    let ValueType::Array(ArrayType::Positions(row_types)) = &types["rows"] else {
        panic!("row positions")
    };
    let facts = value_expression_types(&expression, &types, &mut allowance).unwrap();
    let ValueType::Array(ArrayType::Elements(groups)) = &facts[0] else {
        panic!("group facts")
    };
    let ValueType::Record(fields) = &groups[0] else {
        panic!("group schema")
    };
    assert_eq!(fields[0], ("key".into(), ValueType::String));
    let ValueType::Array(ArrayType::Elements(items)) = &fields[1].1 else {
        panic!("item schemas")
    };
    assert_eq!(items, row_types);
    let bindings = Bindings::new([(Located::new("rows".into(), 0..4), rows)]).unwrap();
    let value = evaluate_value(&expression, &bindings, &mut allowance).unwrap();
    let expected = V::Array(vec![
        V::Record(vec![
            ("key".into(), V::String("b".into())),
            (
                "items".into(),
                V::Array(vec![row("b", 3), row("b", 7), row("b", 3)]),
            ),
        ]),
        V::Record(vec![
            ("key".into(), V::String("a".into())),
            ("items".into(), V::Array(vec![row("a", 5)])),
        ]),
    ]);
    assert_eq!(value, expected);
    assert!(
        matches!(constructed_value_type(&value, &mut allowance, Location::Query(0..1)).unwrap(), ValueType::Array(ArrayType::Positions(groups)) if groups.len() == 2)
    );
    assert_eq!(
        copy_constructed_value(&value, &mut allowance, Location::Query(40..50)).unwrap(),
        value
    );
}

#[test]
fn grouping_retains_empty_and_different_row_schema_facts() {
    let empty = grouping(e(K::Array(vec![])), "kind");
    assert_eq!(evaluate(&empty).unwrap(), V::Array(vec![]));
    assert_eq!(
        value_expression_types(&empty, &BTreeMap::new(), &mut ample()).unwrap(),
        vec![ValueType::Array(ArrayType::Elements(vec![]))]
    );
    let types = BTreeMap::from([(
        "rows".into(),
        ValueType::Array(ArrayType::Elements(vec![ValueType::Record(vec![(
            "kind".into(),
            ValueType::Integer,
        )])])),
    )]);
    assert_eq!(
        value_expression_types(&grouping(name("rows"), "kind"), &types, &mut ample()).unwrap(),
        vec![ValueType::Array(ArrayType::Elements(vec![
            ValueType::Record(vec![
                ("key".into(), ValueType::Integer),
                (
                    "items".into(),
                    ValueType::Array(ArrayType::Elements(vec![ValueType::Record(vec![(
                        "kind".into(),
                        ValueType::Integer
                    )])]))
                ),
            ])
        ]))]
    );
    let expression = grouping(
        e(K::Array(vec![
            record(vec![("kind", int(2)), ("count", int(3))]),
            record(vec![
                ("label", literal(V::String("other".into()))),
                ("kind", int(2)),
            ]),
        ])),
        "kind",
    );
    let facts = value_expression_types(&expression, &BTreeMap::new(), &mut ample()).unwrap();
    let ValueType::Array(ArrayType::Elements(groups)) = &facts[0] else {
        panic!("group facts")
    };
    let ValueType::Record(fields) = &groups[0] else {
        panic!("group schema")
    };
    let ValueType::Array(ArrayType::Elements(rows)) = &fields[1].1 else {
        panic!("item schemas")
    };
    assert_ne!(rows[0], rows[1]);
    assert_eq!(
        evaluate(&expression).unwrap(),
        V::Array(vec![V::Record(vec![
            ("key".into(), V::Integer(2)),
            (
                "items".into(),
                V::Array(vec![
                    V::Record(vec![
                        ("kind".into(), V::Integer(2)),
                        ("count".into(), V::Integer(3))
                    ]),
                    V::Record(vec![
                        ("label".into(), V::String("other".into())),
                        ("kind".into(), V::Integer(2))
                    ]),
                ])
            ),
        ])])
    );
}

#[test]
fn grouping_locates_key_refusals_and_preserves_nominal_identity() {
    let missing = grouping(e(K::Array(vec![record(vec![("other", int(1))])])), "kind");
    validate_value_expression(&missing, &BTreeMap::new(), &mut ample()).unwrap();
    assert_eq!(
        evaluate(&choose(grouping(
            e(K::Array(vec![record(vec![("other", int(1))])])),
            "kind"
        )))
        .unwrap(),
        V::Integer(7)
    );
    let error = evaluate(&missing).unwrap_err();
    assert_eq!(error.location, Location::Query(5..6));
    assert!(error.message.contains("missing record field"));
    let incompatible = grouping(
        e(K::Array(vec![
            record(vec![("kind", int(1))]),
            record(vec![("kind", literal(V::String("1".into())))]),
        ])),
        "kind",
    );
    assert_eq!(
        evaluate(&choose(incompatible)).unwrap_err().location,
        Location::Query(5..6)
    );
    assert_eq!(
        evaluate(&grouping(int(1), "kind")).unwrap_err().location,
        Location::Query(0..1)
    );
    let expression = grouping(
        e(K::Array(vec![
            record(vec![("kind", member_expression(22, 4))]),
            record(vec![("kind", member_expression(11, 1))]),
            record(vec![("kind", member_expression(22, 4))]),
        ])),
        "kind",
    );
    let row = |id, order| V::Record(vec![("kind".into(), V::Member(member(id, order)))]);
    assert_eq!(
        evaluate(&expression).unwrap(),
        V::Array(vec![
            V::Record(vec![
                ("key".into(), V::Member(member(22, 4))),
                ("items".into(), V::Array(vec![row(22, 4), row(22, 4)]))
            ]),
            V::Record(vec![
                ("key".into(), V::Member(member(11, 1))),
                ("items".into(), V::Array(vec![row(11, 1)]))
            ]),
        ])
    );
}

#[test]
fn grouping_and_subsequent_copies_share_finite_allowances() {
    let expression = grouping(
        e(K::Array(vec![
            record(vec![("kind", int(2))]),
            record(vec![("kind", int(1))]),
            record(vec![("kind", int(2))]),
        ])),
        "kind",
    );
    for (mut allowance, limit) in [
        (budget(20, 100, 64), "work"),
        (budget(10_000, 20, 64), "result"),
        (budget(10_000, 100, 4), "nesting"),
    ] {
        assert!(
            evaluate_value(&expression, &Bindings::default(), &mut allowance)
                .unwrap_err()
                .message
                .contains(limit)
        );
    }
    let mut allowance = budget(10_000, 128, 64);
    let value = evaluate_value(&expression, &Bindings::default(), &mut allowance).unwrap();
    let location = Location::Query(40..50);
    let error = loop {
        match copy_constructed_value(&value, &mut allowance, location.clone()) {
            Ok(copy) => assert_eq!(copy, value),
            Err(error) => break error,
        }
    };
    assert_eq!(error.location, location);
    assert!(error.message.contains("result"));
}
fn integer_axis(keys: &[i64]) -> KeyAxis {
    KeyAxis {
        key_type: ValueKeyType::Integer,
        keys: keys.iter().copied().map(ValueKey::Integer).collect(),
    }
}
fn empty_keyed(sources: Option<Vec<E>>) -> E {
    e(K::KeyedComprehension {
        declared_axis_sources: None,
        clauses: vec![
            clause(G::In {
                case: false,
                binders: vec![binder("i")],
                source: e(K::Range {
                    lower: Box::new(int(2)),
                    upper: Box::new(int(1)),
                }),
                filter: None,
            }),
            clause(G::In {
                case: false,
                binders: vec![binder("m")],
                source: name("empty_members"),
                filter: None,
            }),
        ],
        key: Box::new(e(K::Tuple(vec![name("i"), name("m")]))),
        head: Box::new(int(1)),
        empty_axis_sources: sources,
    })
}

#[test]
fn nominal_keyed_report_preserves_identity_axes_order_and_all_five_apis() {
    let mut allowance = ample();
    let members = evaluate_value(
        &e(K::Set(vec![
            member_expression(22, 4),
            member_expression(11, 1),
            member_expression(22, 4),
        ])),
        &Bindings::default(),
        &mut allowance,
    )
    .unwrap();
    assert_eq!(
        members,
        V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(7))),
            elements: vec![V::Member(member(22, 4)), V::Member(member(11, 1))].into()
        }
    );
    assert_eq!(
        evaluate(&binary(
            B::Equal,
            member_expression(11, 1),
            member_expression(22, 4)
        ))
        .unwrap(),
        V::Boolean(false)
    );
    assert_eq!(
        evaluate(&binary(
            B::In,
            member_expression(22, 4),
            literal(
                copy_constructed_value(&members, &mut allowance, Location::Source(1..2)).unwrap()
            )
        ))
        .unwrap(),
        V::Boolean(true)
    );

    let expression = e(K::KeyedComprehension {
        declared_axis_sources: None,
        clauses: vec![
            clause(G::In {
                case: false,
                binders: vec![binder("i")],
                source: e(K::Array(vec![int(9), int(3)])),
                filter: None,
            }),
            clause(G::In {
                case: false,
                binders: vec![binder("m")],
                source: Located::new(K::Name("members".into()), 20..27),
                filter: Some(binary(
                    B::Or,
                    binary(B::Equal, name("i"), int(3)),
                    binary(B::Equal, name("m"), member_expression(11, 1)),
                )),
            }),
        ],
        key: Box::new(e(K::Tuple(vec![name("i"), name("m")]))),
        head: Box::new(name("i")),
        // This arithmetic failure is validated but unused for nonempty output.
        empty_axis_sources: Some(vec![
            e(K::Range {
                lower: Box::new(binary(B::IntegerDivide, int(1), int(0))),
                upper: Box::new(int(0)),
            }),
            name("members"),
        ]),
    });
    let declared = BTreeMap::from([(
        "members".into(),
        ValueType::Set(Some(SetElementType::Member(NominalTypeId(7)))),
    )]);
    assert_eq!(
        free_value_names(&expression, &declared, &mut allowance).unwrap(),
        vec![Located::new("members".into(), 0..1)]
    );
    let ty = constructed_value_type(&members, &mut allowance, Location::Source(2..20)).unwrap();
    validate_value_expression(
        &expression,
        &BTreeMap::from([("members".into(), ty)]),
        &mut allowance,
    )
    .unwrap();
    let bindings = Bindings::new([(Located::new("members".into(), 0..7), members)]).unwrap();
    let result = evaluate_value(&expression, &bindings, &mut allowance).unwrap();
    let expected_axes = || {
        vec![
            integer_axis(&[3, 9]),
            KeyAxis {
                key_type: ValueKeyType::Member(NominalTypeId(7)),
                keys: vec![
                    ValueKey::Member(member(11, 1)),
                    ValueKey::Member(member(22, 4)),
                ],
            },
        ]
    };
    let expected_entries = || {
        [(3, 11, 1), (3, 22, 4), (9, 11, 1)]
            .into_iter()
            .map(|(i, id, order)| KeyedEntry {
                key: vec![ValueKey::Integer(i), ValueKey::Member(member(id, order))],
                value: V::Integer(i),
            })
            .collect()
    };
    assert_eq!(
        result,
        V::KeyedArray {
            axes: expected_axes(),
            entries: expected_entries()
        }
    );
    assert_eq!(
        constructed_value_type(&result, &mut allowance, Location::Query(0..1)).unwrap(),
        ValueType::KeyedArray(KeyedArrayType {
            axes: Some(vec![
                ValueKeyType::Integer,
                ValueKeyType::Member(NominalTypeId(7))
            ]),
            elements: vec![ValueType::Integer, ValueType::Integer, ValueType::Integer]
        })
    );
    let before = allowance.remaining_results().unwrap();
    let copied = copy_constructed_value(&result, &mut allowance, Location::Source(40..50)).unwrap();
    assert!(allowance.remaining_results().unwrap() < before);
    assert_eq!(copied, result);
    if let V::KeyedArray { axes, entries } = &copied {
        let ValueKey::Member(retained) = &axes[1].keys[0] else {
            panic!("member key");
        };
        assert_eq!(
            (
                &retained.type_name,
                &retained.member_name,
                retained.declaration_order
            ),
            (&"Guests".into(), &"same".into(), 1)
        );
        assert_eq!(entries.len(), 3);
    }
    let keyed_bindings = Bindings::new([(Located::new("a".into(), 0..1), copied)]).unwrap();
    assert_eq!(
        evaluate_value(
            &access(
                name("a"),
                e(K::Tuple(vec![int(3), member_expression(22, 4)]))
            ),
            &keyed_bindings,
            &mut allowance
        )
        .unwrap(),
        V::Integer(3)
    );
    assert_eq!(
        evaluate_value(
            &binary(B::Equal, name("a"), name("a")),
            &keyed_bindings,
            &mut allowance
        )
        .unwrap(),
        V::Boolean(true)
    );
    let iterated = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("v")],
            source: name("a"),
            filter: None,
        })],
        head: Box::new(name("v")),
    });
    assert_eq!(
        evaluate_value(&iterated, &keyed_bindings, &mut allowance).unwrap(),
        V::Array(vec![V::Integer(3), V::Integer(3), V::Integer(9)])
    );
    let missing = choose(access(
        name("a"),
        e(K::Tuple(vec![int(9), member_expression(22, 4)])),
    ));
    assert_eq!(
        evaluate_value(&missing, &keyed_bindings, &mut allowance).unwrap(),
        V::Integer(7)
    );

    let supplied = e(K::KeyedArray {
        declared_axis_sources: None,
        axes: Some(vec![
            integer_axis(&[9, 3]),
            KeyAxis {
                key_type: ValueKeyType::Member(NominalTypeId(7)),
                keys: vec![
                    ValueKey::Member(member(22, 4)),
                    ValueKey::Member(member(11, 1)),
                ],
            },
        ]),
        entries: vec![
            (e(K::Tuple(vec![int(3), member_expression(11, 1)])), int(31)),
            (e(K::Tuple(vec![int(9), member_expression(11, 1)])), int(91)),
            (e(K::Tuple(vec![int(9), member_expression(22, 4)])), int(92)),
        ],
    });
    let V::KeyedArray { axes, entries } =
        evaluate_value(&supplied, &Bindings::default(), &mut allowance).unwrap()
    else {
        panic!("keyed array");
    };
    assert_eq!(axes[0], integer_axis(&[9, 3]));
    assert_eq!(
        entries.iter().map(|entry| &entry.value).collect::<Vec<_>>(),
        vec![&V::Integer(92), &V::Integer(91), &V::Integer(31)]
    );
    let mut supplied = supplied;
    if let K::KeyedArray { entries, .. } = &mut supplied.value {
        entries.reverse();
    }
    let aligned = evaluate(&e(K::ZipExact {
        left: Box::new(literal(V::KeyedArray { axes, entries })),
        right: Box::new(supplied),
    }))
    .unwrap();
    let V::KeyedArray { entries, .. } = aligned else {
        panic!("keyed pairs")
    };
    assert_eq!(
        entries[0].value,
        V::Tuple(vec![V::Integer(92), V::Integer(92)])
    );
    let one = e(K::KeyedArray {
        declared_axis_sources: None,
        axes: Some(vec![KeyAxis {
            key_type: ValueKeyType::Member(NominalTypeId(7)),
            keys: vec![
                ValueKey::Member(member(22, 4)),
                ValueKey::Member(member(11, 1)),
            ],
        }]),
        entries: vec![
            (member_expression(11, 1), int(1)),
            (member_expression(22, 4), int(2)),
        ],
    });
    assert_eq!(
        evaluate(&access(one, member_expression(11, 1))).unwrap(),
        V::Integer(1)
    );
    let nested = V::KeyedArray {
        axes: vec![integer_axis(&[9, 3])],
        entries: vec![
            KeyedEntry {
                key: vec![ValueKey::Integer(9)],
                value: V::Array(vec![V::Integer(1)]),
            },
            KeyedEntry {
                key: vec![ValueKey::Integer(3)],
                value: V::Integer(3),
            },
        ],
    };
    let nested_bindings = Bindings::new([(Located::new("nested".into(), 0..6), nested)]).unwrap();
    assert_eq!(
        evaluate_value(
            &binary(B::Equal, name("nested"), name("nested")),
            &nested_bindings,
            &mut allowance
        )
        .unwrap(),
        V::Boolean(true)
    );
    let arrays = literal(V::KeyedArray {
        axes: vec![integer_axis(&[9, 3])],
        entries: vec![KeyedEntry {
            key: vec![ValueKey::Integer(9)],
            value: V::Array(vec![V::Integer(1)]),
        }],
    });
    assert_eq!(
        evaluate(&access(access(arrays, e(K::Tuple(vec![int(9)]))), int(1))).unwrap(),
        V::Integer(1)
    );
    assert_eq!(
        evaluate(&e(K::IndexSet(Box::new(literal(V::KeyedArray {
            axes: vec![integer_axis(&[9, 3])],
            entries: vec![]
        })))))
        .unwrap(),
        V::Set {
            element_type: Some(SetElementType::Integer),
            elements: vec![V::Integer(9), V::Integer(3)].into()
        }
    );
    assert_eq!(
        evaluate(&e(K::IndexSet(Box::new(literal(V::KeyedArray {
            axes: vec![KeyAxis {
                key_type: ValueKeyType::Member(NominalTypeId(7)),
                keys: vec![
                    ValueKey::Member(member(22, 4)),
                    ValueKey::Member(member(11, 1))
                ]
            }],
            entries: vec![]
        })))))
        .unwrap(),
        V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(7))),
            elements: vec![V::Member(member(22, 4)), V::Member(member(11, 1))].into()
        }
    );

    let empty_bindings = Bindings::new([(
        Located::new("empty_members".into(), 0..13),
        V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(7))),
            elements: vec![].into(),
        },
    )])
    .unwrap();
    let empty = empty_keyed(Some(vec![
        e(K::Range {
            lower: Box::new(int(2)),
            upper: Box::new(int(1)),
        }),
        name("empty_members"),
    ]));
    assert_eq!(
        evaluate_value(&empty, &empty_bindings, &mut allowance).unwrap(),
        V::KeyedArray {
            axes: vec![
                integer_axis(&[]),
                KeyAxis {
                    key_type: ValueKeyType::Member(NominalTypeId(7)),
                    keys: vec![]
                }
            ],
            entries: vec![]
        }
    );
    // The second fallback still runs although the first axis/product is empty.
    let later = empty_keyed(Some(vec![
        e(K::Range {
            lower: Box::new(int(2)),
            upper: Box::new(int(1)),
        }),
        e(K::KeyedArray {
            declared_axis_sources: None,
            axes: None,
            entries: vec![(int(9), member_expression(22, 4))],
        }),
    ]));
    assert_eq!(
        evaluate_value(&later, &empty_bindings, &mut allowance).unwrap(),
        V::KeyedArray {
            axes: vec![
                integer_axis(&[]),
                KeyAxis {
                    key_type: ValueKeyType::Member(NominalTypeId(7)),
                    keys: vec![ValueKey::Member(member(22, 4))]
                }
            ],
            entries: vec![]
        }
    );
    let fail_later = empty_keyed(Some(vec![
        e(K::Range {
            lower: Box::new(int(2)),
            upper: Box::new(int(1)),
        }),
        e(K::Array(vec![Located::new(
            K::ArrayAccess {
                array: Box::new(e(K::Array(vec![member_expression(11, 1)]))),
                index: Box::new(int(4)),
            },
            120..130,
        )])),
    ]));
    assert_eq!(
        evaluate_value(&fail_later, &empty_bindings, &mut allowance)
            .unwrap_err()
            .location,
        Location::Query(120..130)
    );
    let skipped_key = e(K::KeyedComprehension {
        declared_axis_sources: None,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("i")],
            source: e(K::Array(vec![int(1)])),
            filter: Some(literal(V::Boolean(false))),
        })],
        key: Box::new(binary(B::IntegerDivide, int(1), int(0))),
        head: Box::new(access(e(K::Array(vec![int(1)])), int(4))),
        empty_axis_sources: Some(vec![e(K::Range {
            lower: Box::new(int(1)),
            upper: Box::new(int(0)),
        })]),
    });
    assert_eq!(
        evaluate(&skipped_key).unwrap(),
        V::KeyedArray {
            axes: vec![integer_axis(&[])],
            entries: vec![]
        }
    );
}

#[test]
fn keyed_helpers_keep_sparse_coordinates_types_and_exact_alignment() {
    let sparse = || V::KeyedArray {
        axes: vec![
            integer_axis(&[9, 3]),
            KeyAxis {
                key_type: ValueKeyType::Member(NominalTypeId(7)),
                keys: vec![
                    ValueKey::Member(member(22, 4)),
                    ValueKey::Member(member(11, 1)),
                ],
            },
        ],
        entries: vec![
            KeyedEntry {
                key: vec![ValueKey::Integer(9), ValueKey::Member(member(22, 4))],
                value: V::Integer(90),
            },
            KeyedEntry {
                key: vec![ValueKey::Integer(3), ValueKey::Member(member(11, 1))],
                value: V::Integer(30),
            },
        ],
    };
    let mut right = sparse();
    if let V::KeyedArray { entries, .. } = &mut right {
        entries[0].value = V::String("ninety".into());
        entries[1].value = V::String("thirty".into());
    }
    let mut allowance = ample();
    let declared = BTreeMap::from([
        (
            "a".into(),
            constructed_value_type(&sparse(), &mut allowance, Location::Source(2..3)).unwrap(),
        ),
        (
            "b".into(),
            constructed_value_type(&right, &mut allowance, Location::Source(4..5)).unwrap(),
        ),
    ]);
    let bindings = Bindings::new([
        (Located::new("a".into(), 2..3), sparse()),
        (Located::new("b".into(), 4..5), right),
    ])
    .unwrap();
    let pair = || {
        e(K::ZipExact {
            left: Box::new(name("a")),
            right: Box::new(name("b")),
        })
    };
    let expression = e(K::Entries(Box::new(pair())));
    validate_value_expression(&expression, &declared, &mut allowance).unwrap();
    assert_eq!(
        free_value_names(&expression, &declared, &mut allowance).unwrap(),
        vec![
            Located::new("a".into(), 0..1),
            Located::new("b".into(), 0..1)
        ]
    );
    let facts = value_expression_types(&expression, &declared, &mut allowance).unwrap();
    let ValueType::Array(ArrayType::Elements(records)) = &facts[0] else {
        panic!("entry facts")
    };
    assert!(!records.is_empty());
    assert!(records.iter().all(|ty| matches!(ty, ValueType::Record(fields)
        if fields[0] == ("key".into(), ValueType::Tuple(vec![ValueType::Integer, ValueType::Member(NominalTypeId(7))]))
        && fields[1] == ("value".into(), ValueType::Tuple(vec![ValueType::Integer, ValueType::String])))));
    let paired = evaluate_value(&pair(), &bindings, &mut allowance).unwrap();
    let V::KeyedArray { axes, entries } = &paired else {
        panic!("paired array")
    };
    assert_eq!(axes[0], integer_axis(&[9, 3]));
    let ValueKey::Member(retained) = &axes[1].keys[0] else {
        panic!("nominal axis")
    };
    assert_eq!(
        (
            retained.type_id,
            retained.member_id,
            retained.declaration_order
        ),
        (NominalTypeId(7), NominalMemberId(22), 4)
    );
    assert_eq!(
        entries[0].value,
        V::Tuple(vec![V::Integer(90), V::String("ninety".into())])
    );
    let records = evaluate_value(&expression, &bindings, &mut allowance).unwrap();
    assert_eq!(
        records,
        V::Array(vec![
            V::Record(vec![
                (
                    "key".into(),
                    V::Tuple(vec![V::Integer(9), V::Member(member(22, 4))])
                ),
                (
                    "value".into(),
                    V::Tuple(vec![V::Integer(90), V::String("ninety".into())])
                )
            ]),
            V::Record(vec![
                (
                    "key".into(),
                    V::Tuple(vec![V::Integer(3), V::Member(member(11, 1))])
                ),
                (
                    "value".into(),
                    V::Tuple(vec![V::Integer(30), V::String("thirty".into())])
                )
            ]),
        ])
    );
    let before = allowance.remaining_results().unwrap();
    assert_eq!(
        copy_constructed_value(&paired, &mut allowance, Location::Query(50..60)).unwrap(),
        paired
    );
    assert!(allowance.remaining_results().unwrap() < before);
    assert_eq!(bindings.get("a", 0..1).unwrap(), &sparse());

    let mut different = sparse();
    if let V::KeyedArray { entries, .. } = &mut different {
        entries[1].key[1] = ValueKey::Member(member(22, 4));
    }
    let mismatch = Located::new(
        K::ZipExact {
            left: Box::new(literal(sparse())),
            right: Box::new(literal(different)),
        },
        70..80,
    );
    assert_eq!(
        evaluate(&mismatch).unwrap_err(),
        zirql::QueryError::query(70..80, "zip_exact needs identical present keys")
    );
    assert_eq!(evaluate(&choose(mismatch)).unwrap(), V::Integer(7));
    let mut reversed = sparse();
    if let V::KeyedArray { axes, entries } = &mut reversed {
        axes[0].keys.reverse();
        entries.reverse();
    }
    assert!(
        evaluate(&e(K::ZipExact {
            left: Box::new(literal(sparse())),
            right: Box::new(literal(reversed))
        }))
        .unwrap_err()
        .message
        .contains("ordered axes")
    );
    let empty = || {
        literal(V::KeyedArray {
            axes: vec![
                integer_axis(&[]),
                KeyAxis {
                    key_type: ValueKeyType::Member(NominalTypeId(7)),
                    keys: vec![],
                },
            ],
            entries: vec![],
        })
    };
    assert_eq!(
        evaluate(&e(K::ZipExact {
            left: Box::new(empty()),
            right: Box::new(empty())
        }))
        .unwrap(),
        V::KeyedArray {
            axes: vec![
                integer_axis(&[]),
                KeyAxis {
                    key_type: ValueKeyType::Member(NominalTypeId(7)),
                    keys: vec![]
                }
            ],
            entries: vec![]
        }
    );
    assert_eq!(
        evaluate(&e(K::Entries(Box::new(empty())))).unwrap(),
        V::Array(vec![])
    );
    for right in [
        literal(V::KeyedArray {
            axes: vec![integer_axis(&[])],
            entries: vec![],
        }),
        literal(V::KeyedArray {
            axes: vec![integer_axis(&[]), integer_axis(&[])],
            entries: vec![],
        }),
    ] {
        assert!(
            evaluate(&choose(e(K::ZipExact {
                left: Box::new(empty()),
                right: Box::new(right)
            })))
            .unwrap_err()
            .message
            .contains("rank and types")
        );
    }
    assert!(
        evaluate(&choose(e(K::Entries(Box::new(int(1))))))
            .unwrap_err()
            .message
            .contains("keyed array")
    );
    let missing = Located::new(K::Name("absent".into()), 90..96);
    assert_eq!(
        evaluate(&choose(e(K::ZipExact {
            left: Box::new(empty()),
            right: Box::new(missing)
        })))
        .unwrap_err()
        .location,
        Location::Query(90..96)
    );
    assert_eq!(
        evaluate(&choose(e(K::Entries(Box::new(access(
            e(K::Array(vec![])),
            int(1)
        ))))))
        .unwrap(),
        V::Integer(7)
    );
    assert!(
        evaluate_value(&expression, &bindings, &mut budget(1, 1000, 64))
            .unwrap_err()
            .message
            .contains("work limit")
    );
    assert!(
        evaluate_value(&expression, &bindings, &mut budget(100_000, 1, 64))
            .unwrap_err()
            .message
            .contains("result limit")
    );
}

#[test]
fn nominal_keyed_duplicate_incompatible_empty_and_budget_refusals_are_located() {
    let duplicate = e(K::KeyedArray {
        declared_axis_sources: None,
        axes: None,
        entries: vec![
            (int(3), int(1)),
            (Located::new(K::Literal(V::Integer(3)), 140..141), int(2)),
        ],
    });
    let refusal = evaluate(&duplicate).unwrap_err();
    assert_eq!(refusal.location, Location::Query(140..141));
    assert!(refusal.message.contains("duplicate"));
    let mut contradictory = member(11, 2);
    contradictory.member_name = "changed".into();
    for kind in [
        K::Set(vec![
            member_expression(11, 1),
            literal(V::Member(contradictory)),
        ]),
        K::Set(vec![member_expression(11, 1), member_expression(22, 1)]),
        K::Binary {
            operator: B::Add,
            left: Box::new(member_expression(11, 1)),
            right: Box::new(int(1)),
        },
        K::KeyedArray {
            declared_axis_sources: None,
            axes: None,
            entries: vec![
                (int(3), int(1)),
                (
                    Located::new(K::Tuple(vec![int(3), int(4)]), 150..160),
                    int(2),
                ),
            ],
        },
        K::IndexSet(Box::new(literal(V::KeyedArray {
            axes: vec![integer_axis(&[]), integer_axis(&[])],
            entries: vec![],
        }))),
    ] {
        assert_eq!(
            evaluate(&Located::new(kind, 150..160))
                .unwrap_err()
                .location,
            Location::Query(150..160)
        );
    }
    let other = NominalMember {
        type_id: NominalTypeId(8),
        ..member(11, 1)
    };
    assert_eq!(
        evaluate(&Located::new(
            K::Binary {
                operator: B::Equal,
                left: Box::new(member_expression(11, 1)),
                right: Box::new(literal(V::Member(other)))
            },
            161..170
        ))
        .unwrap_err()
        .location,
        Location::Query(161..170)
    );
    let wrong = access(
        literal(V::KeyedArray {
            axes: vec![integer_axis(&[3])],
            entries: vec![],
        }),
        member_expression(11, 1),
    );
    assert_eq!(
        evaluate(&Located::new(wrong.value, 171..180))
            .unwrap_err()
            .location,
        Location::Query(171..180)
    );
    let native_bad = V::KeyedArray {
        axes: vec![integer_axis(&[3, 9])],
        entries: vec![
            KeyedEntry {
                key: vec![ValueKey::Integer(9)],
                value: V::Integer(1),
            },
            KeyedEntry {
                key: vec![ValueKey::Integer(3)],
                value: V::Integer(2),
            },
        ],
    };
    assert_eq!(
        constructed_value_type(&native_bad, &mut ample(), Location::Source(181..190))
            .unwrap_err()
            .location,
        Location::Source(181..190)
    );
    let empty_bindings = Bindings::new([(
        Located::new("empty_members".into(), 0..13),
        V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(7))),
            elements: vec![].into(),
        },
    )])
    .unwrap();
    assert_eq!(
        evaluate_value(
            &Located::new(empty_keyed(None).value, 191..200),
            &empty_bindings,
            &mut ample()
        )
        .unwrap_err()
        .location,
        Location::Query(191..200)
    );
    let untyped = empty_keyed(Some(vec![
        Located::new(K::Array(vec![]), 201..210),
        name("empty_members"),
    ]));
    assert_eq!(
        evaluate_value(&untyped, &empty_bindings, &mut ample())
            .unwrap_err()
            .location,
        Location::Query(201..210)
    );
    let own_name = empty_keyed(Some(vec![
        Located::new(K::Name("i".into()), 211..220),
        name("empty_members"),
    ]));
    assert_eq!(
        evaluate_value(&own_name, &empty_bindings, &mut ample())
            .unwrap_err()
            .location,
        Location::Query(211..220)
    );
    let value = V::KeyedArray {
        axes: vec![KeyAxis {
            key_type: ValueKeyType::Member(NominalTypeId(7)),
            keys: vec![ValueKey::Member(member(11, 1))],
        }],
        entries: vec![KeyedEntry {
            key: vec![ValueKey::Member(member(11, 1))],
            value: V::Integer(1),
        }],
    };
    let comparison_data = V::KeyedArray {
        axes: vec![integer_axis(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])],
        entries: vec![],
    };
    let mut comparison_allowance = budget(30, 100, 64);
    let comparison_refusal = constructed_value_type(
        &comparison_data,
        &mut comparison_allowance,
        Location::Source(231..240),
    )
    .unwrap_err();
    assert_eq!(comparison_refusal.location, Location::Source(231..240));
    assert!(comparison_refusal.message.contains("work limit"));
    let analysis = Located::new(
        K::KeyedArray {
            declared_axis_sources: None,
            axes: Some(vec![integer_axis(&[1, 2, 3])]),
            entries: vec![],
        },
        241..250,
    );
    let mut analysis_allowance = budget(3, 100, 64);
    assert_eq!(
        validate_value_expression(&analysis, &BTreeMap::new(), &mut analysis_allowance)
            .unwrap_err()
            .location,
        Location::Query(241..250)
    );
    let location = Location::Source(221..230);
    let mut low = budget(8, 100, 64);
    assert_eq!(
        constructed_value_type(&value, &mut low, location.clone())
            .unwrap_err()
            .location,
        location
    );
    assert!(low.remaining_work() < 8);
    let mut no_results = budget(10_000, 1, 64);
    assert_eq!(
        copy_constructed_value(&value, &mut no_results, location.clone())
            .unwrap_err()
            .location,
        location
    );
    let mut shallow = budget(10_000, 100, 1);
    assert_eq!(
        copy_constructed_value(&value, &mut shallow, location.clone())
            .unwrap_err()
            .location,
        location
    );
    assert_eq!(
        copy_constructed_value(&V::Integer(7), &mut shallow, location.clone()).unwrap(),
        V::Integer(7)
    );
    let mut repeated = budget(10_000, 20, 64);
    let snapshot = copy_constructed_value(&value, &mut repeated, location.clone()).unwrap();
    assert_eq!(snapshot, value);
    let before = repeated.remaining_results().unwrap();
    copy_constructed_value(&value, &mut repeated, location.clone()).unwrap();
    assert!(repeated.remaining_results().unwrap() < before);
    assert!(
        copy_constructed_value(&value, &mut repeated, location)
            .unwrap_err()
            .message
            .contains("result limit")
    );
}

#[test]
fn host_value_policy_refuses_inline_sparse_sources_and_keeps_skipped_branches() {
    use zirql::evaluate_value_with_host_validation;
    let sparse = || {
        Located::new(
            K::KeyedArray {
                declared_axis_sources: None,
                axes: None,
                entries: vec![
                    (e(K::Tuple(vec![int(2), int(1)])), int(21)),
                    (e(K::Tuple(vec![int(1), int(2)])), int(12)),
                    (e(K::Tuple(vec![int(1), int(1)])), int(11)),
                ],
            },
            40..70,
        )
    };
    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("cell")],
            source: sparse(),
            filter: None,
        })],
        head: Box::new(name("cell")),
    });
    // Generic sparse construction remains valid without the host's stronger policy.
    assert_eq!(
        evaluate(&expression).unwrap(),
        V::Array(vec![V::Integer(11), V::Integer(12), V::Integer(21)])
    );
    let validate_cells = |value: &V, budget: &mut Budget, location: &Location| {
        budget.charge(1, location.clone())?;
        if let V::KeyedArray { axes, entries } = value {
            let count = axes
                .iter()
                .try_fold(1usize, |n, axis| n.checked_mul(axis.keys.len()))
                .ok_or_else(|| zirql::QueryError {
                    location: location.clone(),
                    category: zirql::QueryErrorCategory::Other,
                    message: "host product overflow".into(),
                })?;
            if count != entries.len() {
                return Err(zirql::QueryError {
                    location: location.clone(),
                    category: zirql::QueryErrorCategory::Other,
                    message: "host requires complete cells".into(),
                });
            }
        }
        Ok(())
    };
    let mut active = ample();
    let before = active.remaining_work();
    let error = evaluate_value_with_host_validation(
        &expression,
        &Bindings::default(),
        &mut active,
        validate_cells,
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(40..70));
    assert_eq!(error.message, "host requires complete cells");
    assert!(active.remaining_work() < before);
    let error = evaluate_value_with_host_validation(
        &e(K::Entries(Box::new(e(K::ZipExact {
            left: Box::new(sparse()),
            right: Box::new(sparse()),
        })))),
        &Bindings::default(),
        &mut active,
        validate_cells,
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(40..70));
    assert_eq!(error.message, "host requires complete cells");
    assert_eq!(
        evaluate_value_with_host_validation(
            &choose(sparse()),
            &Bindings::default(),
            &mut active,
            validate_cells
        )
        .unwrap(),
        V::Integer(7)
    );
    // The same allowance remains usable after the host error; scopes/depth restore.
    assert_eq!(
        evaluate_value(&int(8), &Bindings::default(), &mut active).unwrap(),
        V::Integer(8)
    );
    let invalid = choose(binary(B::Add, int(1), literal(V::Boolean(false))));
    assert!(
        evaluate_value_with_host_validation(
            &invalid,
            &Bindings::default(),
            &mut ample(),
            validate_cells
        )
        .unwrap_err()
        .message
        .contains("numeric")
    );

    let mut low = ample();
    let error = evaluate_value_with_host_validation(
        &e(K::Array(vec![int(1)])),
        &Bindings::default(),
        &mut low,
        |_, budget, location| budget.charge(1_000_000, location.clone()),
    )
    .unwrap_err();
    assert_eq!(error.location, Location::Query(0..1));
    assert!(error.message.contains("work limit"));
    let mut shallow = budget(10_000, 100, 2);
    let value = evaluate_value_with_host_validation(
        &int(1),
        &Bindings::default(),
        &mut shallow,
        |value, budget, location| {
            constructed_value_type(value, budget, location.clone()).map(|_| ())
        },
    )
    .unwrap();
    assert_eq!(value, V::Integer(1));
    assert_eq!(
        evaluate_value(&int(2), &Bindings::default(), &mut shallow).unwrap(),
        V::Integer(2)
    );
}

fn reduction(operator: R, array: E) -> E {
    e(K::Reduce {
        operator,
        array: Box::new(array),
    })
}
fn requirement(condition: E, value: E) -> E {
    Located::new(
        K::Require {
            condition: Box::new(condition),
            value: Box::new(value),
            message: Located::new("capacity refused".into(), 80..98),
        },
        60..99,
    )
}

#[test]
fn eager_summaries_and_requirement_use_the_public_value_sequence() {
    let expression = requirement(
        literal(V::Boolean(true)),
        record(vec![
            ("length", reduction(R::Length, name("cells"))),
            ("total", reduction(R::Sum, name("cells"))),
            (
                "all",
                reduction(
                    R::Forall,
                    e(K::Array(vec![
                        literal(V::Boolean(true)),
                        literal(V::Boolean(false)),
                    ])),
                ),
            ),
            (
                "some",
                reduction(
                    R::Exists,
                    e(K::Array(vec![
                        literal(V::Boolean(false)),
                        literal(V::Boolean(true)),
                    ])),
                ),
            ),
        ]),
    );
    let cells = V::KeyedArray {
        axes: vec![integer_axis(&[2, 5, 9])],
        entries: vec![
            KeyedEntry {
                key: vec![ValueKey::Integer(2)],
                value: V::Integer(4),
            },
            KeyedEntry {
                key: vec![ValueKey::Integer(9)],
                value: V::Integer(3),
            },
        ],
    };
    let mut allowance = ample();
    let declared = BTreeMap::from([("cells".into(), ValueType::Unestablished)]);
    assert_eq!(
        free_value_names(&expression, &declared, &mut allowance).unwrap(),
        vec![Located::new("cells".into(), 0..1)]
    );
    let ty = constructed_value_type(&cells, &mut allowance, Location::Source(0..20)).unwrap();
    validate_value_expression(
        &expression,
        &BTreeMap::from([("cells".into(), ty)]),
        &mut allowance,
    )
    .unwrap();
    let bindings = Bindings::new([(Located::new("cells".into(), 0..5), cells)]).unwrap();
    let value = evaluate_value(&expression, &bindings, &mut allowance).unwrap();
    assert_eq!(
        value,
        V::Record(vec![
            ("length".into(), V::Integer(2)),
            ("total".into(), V::Integer(7)),
            ("all".into(), V::Boolean(false)),
            ("some".into(), V::Boolean(true)),
        ])
    );
    assert_eq!(
        constructed_value_type(&value, &mut allowance, Location::Query(60..99)).unwrap(),
        ValueType::Record(vec![
            ("length".into(), ValueType::Integer),
            ("total".into(), ValueType::Integer),
            ("all".into(), ValueType::Boolean),
            ("some".into(), ValueType::Boolean),
        ])
    );
    let remaining = allowance.remaining_results().unwrap();
    assert_eq!(
        copy_constructed_value(&value, &mut allowance, Location::Query(60..99)).unwrap(),
        value
    );
    assert!(allowance.remaining_results().unwrap() < remaining);
}

#[test]
fn summaries_keep_empty_identities_overflow_eager_heads_and_limits() {
    let empty = record(vec![
        ("length", reduction(R::Length, e(K::Array(vec![])))),
        ("sum", reduction(R::Sum, e(K::Array(vec![])))),
        ("all", reduction(R::Forall, e(K::Array(vec![])))),
        ("some", reduction(R::Exists, e(K::Array(vec![])))),
    ]);
    assert_eq!(
        evaluate(&empty).unwrap(),
        V::Record(vec![
            ("length".into(), V::Integer(0)),
            ("sum".into(), V::Integer(0)),
            ("all".into(), V::Boolean(true)),
            ("some".into(), V::Boolean(false)),
        ])
    );
    let overflow = Located::new(
        K::Reduce {
            operator: R::Sum,
            array: Box::new(e(K::Array(vec![int(i64::MAX), int(1)]))),
        },
        40..50,
    );
    let failure = evaluate(&overflow).unwrap_err();
    assert_eq!(failure.location, Location::Query(40..50));
    assert!(failure.message.contains("overflow"));
    let division = Located::new(
        K::Binary {
            operator: B::IntegerDivide,
            left: Box::new(int(1)),
            right: Box::new(int(0)),
        },
        30..35,
    );
    let head = e(K::Conditional {
        branches: vec![(
            binary(B::Equal, name("i"), int(1)),
            literal(V::Boolean(false)),
        )],
        otherwise: Box::new(binary(B::Equal, division, int(0))),
    });
    let eager = reduction(
        R::Forall,
        e(K::Comprehension {
            constructor: ValueConstructor::Array,
            clauses: vec![clause(G::In {
                case: false,
                binders: vec![binder("i")],
                source: e(K::Array(vec![int(1), int(2)])),
                filter: None,
            })],
            head: Box::new(head),
        }),
    );
    assert_eq!(
        evaluate(&eager).unwrap_err().location,
        Location::Query(30..35)
    );
    for bad in [
        reduction(R::Sum, e(K::Array(vec![literal(V::Float(1.0))]))),
        reduction(
            R::Exists,
            e(K::Array(vec![literal(V::Boolean(true)), int(0)])),
        ),
        reduction(R::Length, literal(V::String("x".into()))),
    ] {
        assert!(validate_value_expression(&choose(bad), &BTreeMap::new(), &mut ample()).is_err());
    }
    let summary = reduction(R::Sum, e(K::Array(vec![int(1), int(2)])));
    assert!(
        evaluate_value(&summary, &Bindings::default(), &mut budget(2, 100, 64))
            .unwrap_err()
            .message
            .contains("work")
    );
    assert!(
        evaluate_value(&summary, &Bindings::default(), &mut budget(10_000, 0, 64))
            .unwrap_err()
            .message
            .contains("result")
    );
    assert!(
        evaluate_value(&summary, &Bindings::default(), &mut budget(10_000, 100, 1))
            .unwrap_err()
            .message
            .contains("nesting")
    );
}

#[test]
fn false_requirement_skips_runtime_values_but_preflights_names_and_types() {
    let skipped = e(K::Tuple(vec![
        binary(B::IntegerDivide, int(1), int(0)),
        access(e(K::Array(vec![])), int(7)),
    ]));
    let expression = requirement(literal(V::Boolean(false)), skipped);
    let failure = evaluate(&expression).unwrap_err();
    assert_eq!(failure.location, Location::Query(60..99));
    assert_eq!(failure.message, "capacity refused");
    let missing = Located::new(K::Name("missing".into()), 30..37);
    let failure = evaluate(&requirement(literal(V::Boolean(false)), missing)).unwrap_err();
    assert_eq!(failure.location, Location::Query(30..37));
    assert!(failure.message.contains("undefined"));
    assert!(
        evaluate(&requirement(
            literal(V::Boolean(false)),
            reduction(R::Sum, e(K::Array(vec![literal(V::String("x".into()))])))
        ))
        .is_err()
    );
    let condition = Located::new(K::Literal(V::Integer(0)), 10..11);
    assert_eq!(
        evaluate(&requirement(condition, int(7)))
            .unwrap_err()
            .location,
        Location::Query(10..11)
    );
    let arithmetic = Located::new(
        K::Binary {
            operator: B::Multiply,
            left: Box::new(int(i64::MAX)),
            right: Box::new(int(2)),
        },
        20..25,
    );
    assert_eq!(
        evaluate(&requirement(literal(V::Boolean(true)), arithmetic))
            .unwrap_err()
            .location,
        Location::Query(20..25)
    );
    let expression = requirement(literal(V::Boolean(false)), int(7));
    let failure = zirql::evaluate_value_with_host_validation(
        &expression,
        &Bindings::default(),
        &mut ample(),
        |value, _, location| {
            if matches!(value, V::Integer(7)) {
                return Err(zirql::QueryError {
                    location: location.clone(),
                    category: zirql::QueryErrorCategory::Other,
                    message: "host refuses seven".into(),
                });
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(failure.message, "capacity refused");
    assert!(
        evaluate_value(&expression, &Bindings::default(), &mut budget(2, 100, 64))
            .unwrap_err()
            .message
            .contains("work")
    );
}

fn lookup(value: E, key: E) -> E {
    e(K::Lookup {
        value: Box::new(value),
        key: Box::new(key),
    })
}
fn has_key(value: E, key: E) -> E {
    e(K::HasKey {
        value: Box::new(value),
        key: Box::new(key),
    })
}
fn default(value: E, fallback: E) -> E {
    e(K::Default {
        value: Box::new(value),
        fallback: Box::new(fallback),
    })
}
fn text(value: &str) -> E {
    literal(V::String(value.into()))
}
fn declared(value: V, declaration: D<()>) -> V {
    zirql::declare_constructed(value, declaration, &mut ample(), Location::Source(3..9)).unwrap()
}

#[test]
fn optional_report_keeps_declared_omissions_stored_absence_and_sparse_membership() {
    let schema = D::Record(vec![
        ("id".into(), D::Integer),
        ("caption".into(), D::Optional(Box::new(D::String))),
        ("count".into(), D::Optional(Box::new(D::Integer))),
    ]);
    let row = declared(
        V::Record(vec![
            ("id".into(), V::Integer(4)),
            ("count".into(), V::Absent),
        ]),
        schema.clone(),
    );
    let mut row = row;
    for _ in 0..80 {
        row = declared(row, schema.clone());
    }
    let V::Declared { value, .. } = &row else {
        panic!("declared row");
    };
    assert!(matches!(value.as_ref(), V::Record(_)));
    let copied = copy_constructed_value(&row, &mut ample(), Location::Query(1..2)).unwrap();
    assert_eq!(copied, row);
    let cells = declared(
        V::KeyedArray {
            axes: vec![KeyAxis {
                key_type: ValueKeyType::Integer,
                keys: vec![
                    ValueKey::Integer(2),
                    ValueKey::Integer(5),
                    ValueKey::Integer(9),
                ],
            }],
            entries: vec![
                KeyedEntry {
                    key: vec![ValueKey::Integer(2)],
                    value: V::Absent,
                },
                KeyedEntry {
                    key: vec![ValueKey::Integer(9)],
                    value: V::Integer(90),
                },
            ],
        },
        D::Keyed(Box::new(D::Optional(Box::new(D::Integer)))),
    );
    let bindings = Bindings::new([
        (Located::new("row".into(), 0..0), row),
        (Located::new("cells".into(), 0..0), cells),
    ])
    .unwrap();
    let report = record(vec![
        ("omitted", has_key(name("row"), text("caption"))),
        ("stored", has_key(name("row"), text("count"))),
        (
            "caption",
            default(lookup(name("row"), text("caption")), text("untitled")),
        ),
        ("count", default(lookup(name("row"), text("count")), int(7))),
        ("stored_cell", has_key(name("cells"), int(2))),
        ("missing_cell", has_key(name("cells"), int(5))),
        ("outside_axis", has_key(name("cells"), int(8))),
        (
            "cell_default",
            default(lookup(name("cells"), int(5)), int(8)),
        ),
    ]);
    let result = evaluate_value(&report, &bindings, &mut ample()).unwrap();
    let V::Record(fields) = result else {
        panic!("report");
    };
    assert_eq!(
        fields
            .iter()
            .map(|(label, value)| (label.as_str(), value.data()))
            .collect::<Vec<_>>(),
        vec![
            ("omitted", &V::Boolean(false)),
            ("stored", &V::Boolean(true)),
            ("caption", &V::String("untitled".into())),
            ("count", &V::Integer(7)),
            ("stored_cell", &V::Boolean(true)),
            ("missing_cell", &V::Boolean(false)),
            ("outside_axis", &V::Boolean(false)),
            ("cell_default", &V::Integer(8)),
        ]
    );
    assert!(
        evaluate_value(&field(name("row"), "caption"), &bindings, &mut ample())
            .unwrap_err()
            .message
            .contains("missing record field")
    );
    assert!(
        evaluate_value(&access(name("cells"), int(5)), &bindings, &mut ample())
            .unwrap_err()
            .message
            .contains("missing keyed array cell")
    );
    for expression in [
        lookup(name("row"), text("typo")),
        lookup(name("row"), int(1)),
        lookup(name("cells"), text("bad")),
        lookup(name("cells"), e(K::Tuple(vec![int(2), int(3)]))),
    ] {
        assert!(evaluate_value(&expression, &bindings, &mut ample()).is_err());
    }
    for invalid in [
        V::Record(vec![]),
        V::Record(vec![
            ("id".into(), V::Integer(1)),
            ("extra".into(), V::Absent),
        ]),
    ] {
        let error = zirql::declare_constructed(
            invalid,
            schema.clone(),
            &mut ample(),
            Location::Source(3..9),
        )
        .unwrap_err();
        assert_eq!(error.location, Location::Source(3..9));
    }
    let nominal = V::KeyedArray {
        axes: vec![KeyAxis {
            key_type: ValueKeyType::Member(NominalTypeId(7)),
            keys: vec![ValueKey::Member(member(1, 0))],
        }],
        entries: vec![],
    };
    let mut foreign = member(1, 0);
    foreign.type_id = NominalTypeId(8);
    assert!(evaluate(&lookup(literal(nominal), literal(V::Member(foreign)))).is_err());
    // Declared element schemas remain available even when there are no rows.
    let empty = declared(V::Array(vec![]), D::Sequence(Box::new(schema.clone())));
    let invalid = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("r")],
            source: literal(empty),
            filter: None,
        })],
        head: Box::new(lookup(name("r"), text("typo"))),
    });
    assert!(evaluate(&invalid).is_err());
    // A declaration follows a record through a sequence and a generator local.
    let source = declared(V::Array(vec![copied]), D::Sequence(Box::new(schema)));
    let report = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("r")],
            source: literal(source),
            filter: None,
        })],
        head: Box::new(default(
            lookup(name("r"), text("caption")),
            text("untitled"),
        )),
    });
    let V::Array(rows) = evaluate(&report).unwrap() else {
        panic!("rows");
    };
    assert_eq!(rows[0].data(), &V::String("untitled".into()));
}

#[test]
fn retained_lookup_and_positions_keep_child_contracts_and_aliases() {
    let row_type = D::Record(vec![
        ("id".into(), D::Integer),
        ("caption".into(), D::Optional(Box::new(D::String))),
    ]);
    let row = || V::Record(vec![("id".into(), V::Integer(7))]);
    let report = declared(
        V::Record(vec![
            ("rows".into(), V::Array(vec![row()])),
            (
                "cells".into(),
                V::KeyedArray {
                    axes: vec![KeyAxis {
                        key_type: ValueKeyType::Integer,
                        keys: vec![ValueKey::Integer(2), ValueKey::Integer(5)],
                    }],
                    entries: vec![KeyedEntry {
                        key: vec![ValueKey::Integer(2)],
                        value: row(),
                    }],
                },
            ),
        ]),
        D::Record(vec![
            ("rows".into(), D::Sequence(Box::new(row_type.clone()))),
            ("cells".into(), D::Keyed(Box::new(row_type.clone()))),
        ]),
    );
    let bindings = Bindings::new([(Located::new("report".into(), 0..1), report)]).unwrap();
    let expression = block(
        vec![
            (binder("rows"), field(name("report"), "rows")),
            (binder("first"), lookup(name("rows"), int(1))),
        ],
        e(K::Tuple(vec![
            name("first"),
            access(name("rows"), int(1)),
            lookup(field(name("report"), "cells"), int(2)),
            lookup(field(name("report"), "cells"), int(5)),
            lookup(name("rows"), int(0)),
            has_key(name("rows"), int(1)),
            has_key(name("rows"), int(2)),
            default(lookup(name("first"), text("caption")), text("untitled")),
        ])),
    );
    let V::Tuple(mut values) = evaluate_value(&expression, &bindings, &mut ample()).unwrap() else {
        panic!("selected report");
    };
    let optional_row = D::Optional(Box::new(row_type.clone()));
    assert_eq!(values[0], declared(row(), optional_row.clone()));
    assert_eq!(values[1], declared(row(), row_type.clone()));
    assert_eq!(values[2], values[0]);
    assert_eq!(values[3], declared(V::Absent, optional_row.clone()));
    assert_eq!(values[4], declared(V::Absent, optional_row));
    assert_eq!(
        values[5..],
        [
            V::Boolean(true),
            V::Boolean(false),
            declared(V::String("untitled".into()), D::String)
        ]
    );
    let V::Declared { value, .. } = &mut values[0] else {
        panic!("declared row")
    };
    **value = V::Record(vec![]);
    assert_eq!(values[1].data(), &row());
    assert_eq!(values[2].data(), &row());
    assert_eq!(
        evaluate_value(
            &access(field(name("report"), "rows"), int(1)),
            &bindings,
            &mut ample()
        )
        .unwrap()
        .data(),
        &row(),
    );
    let caption = function(
        "caption",
        vec![("rows", D::Sequence(Box::new(row_type)))],
        D::String,
        default(
            lookup(lookup(name("rows"), int(1)), text("caption")),
            text("untitled"),
        ),
    );
    assert_eq!(
        run_functions(
            &[caption],
            &call("caption", vec![literal(V::Array(vec![row()]))]),
            1,
            &mut ample()
        )
        .unwrap()
        .data(),
        &V::String("untitled".into()),
    );
    for (index, message) in [
        (-1, "missing array position"),
        (2, "missing collection position"),
    ] {
        let expression = Located::new(
            access(field(name("report"), "rows"), int(index)).value,
            80..90,
        );
        let error = evaluate_value(&expression, &bindings, &mut ample()).unwrap_err();
        assert_eq!(error.location, Location::Query(80..90));
        assert_eq!(error.message, message);
    }
}

#[test]
fn array_declarations_preserve_storage_and_deferred_element_contracts() {
    let integers = D::Array(Box::new(D::Integer));
    let schema = D::Record(vec![
        ("xs".into(), integers.clone()),
        ("note".into(), D::Optional(Box::new(D::String))),
    ]);
    let keyed = V::KeyedArray {
        axes: vec![KeyAxis {
            key_type: ValueKeyType::Integer,
            keys: vec![ValueKey::Integer(3)],
        }],
        entries: vec![KeyedEntry {
            key: vec![ValueKey::Integer(3)],
            value: V::Integer(7),
        }],
    };
    let rows = declared(
        V::Array(vec![
            V::Record(vec![(
                "xs".into(),
                V::Array(vec![V::Integer(1), V::Integer(2)]),
            )]),
            V::Record(vec![("xs".into(), keyed)]),
            V::Record(vec![("xs".into(), V::Array(vec![]))]),
        ]),
        D::Array(Box::new(schema.clone())),
    );
    let mut bindings = Bindings::new([(Located::new("rows".into(), 0..1), rows)]).unwrap();
    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("r")],
            source: name("rows"),
            filter: None,
        })],
        head: Box::new(e(K::Reduce {
            operator: R::Sum,
            array: Box::new(field(name("r"), "xs")),
        })),
    });
    let V::Array(sums) = evaluate_value(&expression, &bindings, &mut ample()).unwrap() else {
        panic!("sums")
    };
    assert_eq!(sums, [V::Integer(3), V::Integer(7), V::Integer(0)]);
    let value = evaluate_value(
        &field(access(name("rows"), int(2)), "xs"),
        &bindings,
        &mut ample(),
    )
    .unwrap();
    assert!(matches!(value.data(), V::KeyedArray { .. }));
    assert!(
        admit_constructed(
            &value,
            &D::<()>::Sequence(Box::new(D::Integer)),
            &mut ample(),
            Location::Source(3..9)
        )
        .is_err()
    );
    admit_constructed(
        &value,
        &D::<()>::Keyed(Box::new(D::Integer)),
        &mut ample(),
        Location::Source(3..9),
    )
    .unwrap();
    assert!(
        zirql::declare_constructed(
            V::Array(vec![V::Boolean(true)]),
            integers.clone(),
            &mut ample(),
            Location::Source(3..9)
        )
        .is_err()
    );
    // Missing optional arrays retain their element schema without inventing storage.
    let missing = || literal(declared(V::Absent, D::Optional(Box::new(integers.clone()))));
    assert!(evaluate(&default(missing(), record(vec![]))).is_err());
    assert!(
        matches!(evaluate(&default(missing(), literal(V::Array(vec![])))).unwrap().data(), V::Array(values) if values.is_empty())
    );
    let empty = declared(
        V::Array(vec![]),
        D::Array(Box::new(D::Array(Box::new(schema)))),
    );
    bindings = Bindings::new([(Located::new("rows".into(), 0..1), empty)]).unwrap();
    let invalid = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![
            clause(G::In {
                case: false,
                binders: vec![binder("inner")],
                source: name("rows"),
                filter: None,
            }),
            clause(G::In {
                case: false,
                binders: vec![binder("r")],
                source: name("inner"),
                filter: None,
            }),
        ],
        head: Box::new(lookup(name("r"), text("typo"))),
    });
    assert!(evaluate_value(&invalid, &bindings, &mut ample()).is_err());
}

#[test]
fn optional_types_presence_and_lazy_defaults_preserve_failures_and_budget() {
    for value in [
        V::Boolean(false),
        V::Integer(0),
        V::String(String::new()),
        V::Array(vec![]),
        V::Record(vec![]),
    ] {
        let original = copy_constructed_value(&value, &mut ample(), Location::Query(0..1)).unwrap();
        assert_eq!(
            evaluate(&default(literal(value), literal(V::Absent)))
                .unwrap()
                .data(),
            &original
        );
        assert_eq!(
            evaluate(&e(K::Occurs(Box::new(literal(original))))).unwrap(),
            V::Boolean(true)
        );
    }
    assert_eq!(
        evaluate(&e(K::Absent(Box::new(literal(V::Absent))))).unwrap(),
        V::Boolean(true)
    );
    assert_eq!(
        evaluate(&binary(B::Equal, literal(V::Absent), literal(V::Absent))).unwrap(),
        V::Boolean(true)
    );
    assert_eq!(
        evaluate(&binary(B::Equal, literal(V::Absent), int(0))).unwrap(),
        V::Boolean(false)
    );
    let absent_int = || literal(declared(V::Absent, D::Optional(Box::new(D::Integer))));
    let absent_string = || literal(declared(V::Absent, D::Optional(Box::new(D::String))));
    for expression in [
        binary(B::Equal, absent_int(), absent_string()),
        default(absent_int(), absent_string()),
        default(default(absent_int(), literal(V::Absent)), text("wrong")),
        binary(B::Add, literal(V::Absent), int(1)),
        binary(B::Less, literal(V::Absent), int(1)),
    ] {
        assert!(evaluate(&expression).is_err());
    }
    assert!(
        admit_constructed(
            &declared(V::Absent, D::Optional(Box::new(D::Integer))),
            &D::<()>::Optional(Box::new(D::String)),
            &mut ample(),
            Location::Query(9..10)
        )
        .is_err()
    );
    let fail = || {
        Located::new(
            K::Binary {
                operator: B::IntegerDivide,
                left: Box::new(int(1)),
                right: Box::new(int(0)),
            },
            80..90,
        )
    };
    assert_eq!(evaluate(&default(int(4), fail())).unwrap(), V::Integer(4));
    let error = evaluate(&default(literal(V::Absent), fail())).unwrap_err();
    assert_eq!(error.location, Location::Query(80..90));
    assert!(error.message.contains("zero"));
    // Raw fallback data retains its known outer kind without inspecting fields.
    assert!(evaluate(&default(int(4), literal(V::Record(vec![])))).is_err());
    assert!(evaluate(&default(int(4), record(vec![]))).is_err());
    let records =
        Bindings::new([(Located::new("record".into(), 0..0), V::Record(vec![]))]).unwrap();
    assert!(evaluate_value(&default(int(4), name("record")), &records, &mut ample()).is_err());
    let present_record = || record(vec![("id", int(1))]);
    let fallback_record = || literal(V::Record(vec![("id".into(), V::Integer(2))]));
    assert_eq!(
        evaluate(&default(present_record(), fallback_record())).unwrap(),
        V::Record(vec![("id".into(), V::Integer(1))])
    );
    let present_record = declared(
        V::Record(vec![("id".into(), V::Integer(1))]),
        D::Record(vec![("id".into(), D::Integer)]),
    );
    assert_eq!(
        evaluate(&default(literal(present_record), fallback_record()))
            .unwrap()
            .data(),
        &V::Record(vec![("id".into(), V::Integer(1))])
    );
    assert!(evaluate(&default(int(4), name("undefined"))).is_err());
    assert!(evaluate(&default(int(4), binary(B::Add, text("bad"), int(1)))).is_err());
    let bindings = Bindings::new([(
        Located::new("large".into(), 0..0),
        V::Array((0..2000).map(V::Integer).collect()),
    )])
    .unwrap();
    let fallback = || e(K::One(Box::new(name("large"))));
    assert_eq!(
        evaluate_value(
            &default(int(4), fallback()),
            &bindings,
            &mut budget(300, 100, 64)
        )
        .unwrap(),
        V::Integer(4)
    );
    let error = evaluate_value(
        &default(literal(V::Absent), fallback()),
        &bindings,
        &mut budget(300, 100, 64),
    )
    .unwrap_err();
    assert_eq!(error.category, zirql::QueryErrorCategory::WorkLimit);
    let mut shallow = budget(1000, 100, 2);
    assert!(
        evaluate_value(
            &default(literal(V::Absent), fallback()),
            &bindings,
            &mut shallow
        )
        .is_err()
    );
    assert_eq!(
        evaluate_value(&int(1), &Bindings::default(), &mut shallow).unwrap(),
        V::Integer(1)
    );
}

#[test]
fn shared_reads_keep_aliases_admit_new_data_and_release_failed_scopes() {
    let bindings = Bindings::new([(
        Located::new("xs".into(), 0..0),
        V::Array(vec![V::String("first".into()), V::String("second".into())]),
    )])
    .unwrap();
    let expression = e(K::Array(vec![name("xs"), name("xs")]));
    let mut output = evaluate_value(&expression, &bindings, &mut ample()).unwrap();
    let V::Array(copies) = &mut output else {
        unreachable!()
    };
    let V::Array(first) = &mut copies[0] else {
        unreachable!()
    };
    first.clear();
    assert_eq!(
        copies[1],
        V::Array(vec![V::String("first".into()), V::String("second".into())])
    );
    assert_eq!(bindings.get("xs", 0..0).unwrap(), &copies[1]);
    let length = e(K::Reduce {
        operator: R::Length,
        array: Box::new(name("xs")),
    });
    let fresh = Bindings::new([(
        Located::new("xs".into(), 0..0),
        V::Array(vec![V::Integer(1), V::Float(f64::NAN)]),
    )])
    .unwrap();
    assert!(evaluate_value(&length, &fresh, &mut ample()).is_err());
    for expression in [
        lookup(name("xs"), int(1)),
        has_key(name("xs"), int(1)),
        access(name("xs"), int(1)),
    ] {
        assert!(evaluate_value(&expression, &fresh, &mut ample()).is_err());
    }
    let failure = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("n")],
            source: name("xs"),
            filter: None,
        })],
        head: Box::new(binary(B::IntegerDivide, int(1), int(0))),
    });
    let mut allowance = ample();
    assert!(evaluate_value(&failure, &bindings, &mut allowance).is_err());
    let remaining = allowance.remaining_work();
    assert_eq!(
        evaluate_value(&length, &bindings, &mut allowance).unwrap(),
        V::Integer(2)
    );
    assert!(allowance.remaining_work() < remaining);
}

fn call(function: &str, arguments: Vec<E>) -> E {
    Located::new(
        K::Call {
            function: Located::new(function.into(), 30..34),
            arguments,
        },
        30..40,
    )
}
fn function(
    function: &str,
    parameters: Vec<(&str, D<()>)>,
    result: D<()>,
    value: E,
) -> zirql::FunctionDeclaration {
    zirql::FunctionDeclaration {
        name: Located::new(function.into(), 2..6),
        parameters: parameters
            .into_iter()
            .map(|(name, declaration)| {
                Located::new(
                    zirql::FunctionParameter {
                        name: Located::new(name.into(), 7..8),
                        declaration,
                    },
                    7..12,
                )
            })
            .collect(),
        result: Located::new(result, 13..16),
        bindings: vec![],
        value,
    }
}
fn select_branch(condition: E, yes: E, no: E) -> E {
    e(K::Conditional {
        branches: vec![(condition, yes)],
        otherwise: Box::new(no),
    })
}
fn tail_body(next: &str) -> E {
    select_branch(
        binary(B::Equal, name("n"), int(0)),
        name("total"),
        call(
            next,
            vec![
                binary(B::Subtract, name("n"), int(1)),
                binary(B::Add, name("total"), name("n")),
            ],
        ),
    )
}
fn run_functions(
    functions: &[zirql::FunctionDeclaration],
    entry: &E,
    depth: usize,
    budget: &mut Budget,
) -> Result<V, zirql::FunctionError<zirql::QueryError>> {
    zirql::evaluate_functions(
        functions,
        entry,
        &Bindings::default(),
        budget,
        zirql::FunctionLimits {
            active_depth: depth,
        },
    )
}

#[test]
fn typed_functions_reuse_compatible_tail_frames_and_read_old_slots() {
    let mut accumulator = function(
        "acc",
        vec![("n", D::Integer), ("total", D::Integer)],
        D::Integer,
        tail_body("acc"),
    );
    // Locals are fresh on each tail transition and cannot overwrite an argument
    // before the following argument has read the old slots.
    accumulator
        .bindings
        .push((Located::new("old".into(), 20..23), name("n")));
    assert_eq!(
        run_functions(
            &[accumulator],
            &call("acc", vec![int(10_000), int(0)]),
            1,
            &mut budget(20_000_000, 1_000_000, 64)
        )
        .unwrap()
        .data(),
        &V::Integer(50_005_000)
    );
    let a = function(
        "a",
        vec![("n", D::Integer), ("total", D::Integer)],
        D::Integer,
        tail_body("b"),
    );
    let b = function(
        "b",
        vec![("n", D::Integer), ("total", D::Integer)],
        D::Integer,
        tail_body("a"),
    );
    assert_eq!(
        run_functions(
            &[a, b],
            &call("a", vec![int(1000), int(0)]),
            1,
            &mut ample()
        )
        .unwrap()
        .data(),
        &V::Integer(500_500)
    );
    let swap = function(
        "swap",
        vec![("n", D::Integer), ("x", D::Integer), ("y", D::Integer)],
        D::Tuple(vec![D::Integer, D::Integer]),
        select_branch(
            binary(B::Equal, name("n"), int(0)),
            e(K::Tuple(vec![name("x"), name("y")])),
            call(
                "swap",
                vec![binary(B::Subtract, name("n"), int(1)), name("y"), name("x")],
            ),
        ),
    );
    let swapped = run_functions(
        &[swap],
        &call("swap", vec![int(1001), int(3), int(7)]),
        1,
        &mut ample(),
    )
    .unwrap();
    assert!(
        matches!(swapped.data(),V::Tuple(values) if values[0].data()==&V::Integer(7) && values[1].data()==&V::Integer(3))
    );
}

#[test]
fn function_continuations_cover_branches_operands_and_comprehensions() {
    let count = function(
        "count",
        vec![("n", D::Integer)],
        D::Integer,
        select_branch(
            binary(B::Equal, name("n"), int(0)),
            int(0),
            binary(
                B::Add,
                int(1),
                call("count", vec![binary(B::Subtract, name("n"), int(1))]),
            ),
        ),
    );
    assert_eq!(
        run_functions(&[count], &call("count", vec![int(8)]), 16, &mut ample())
            .unwrap()
            .data(),
        &V::Integer(8)
    );
    let left = function(
        "left",
        vec![("n", D::Integer)],
        D::Integer,
        select_branch(
            binary(B::Equal, name("n"), int(0)),
            int(0),
            binary(
                B::Add,
                int(1),
                call("right", vec![binary(B::Subtract, name("n"), int(1))]),
            ),
        ),
    );
    let right = function(
        "right",
        vec![("n", D::Integer)],
        D::Integer,
        call("left", vec![name("n")]),
    );
    assert_eq!(
        run_functions(
            &[left, right],
            &call("left", vec![int(8)]),
            16,
            &mut ample()
        )
        .unwrap()
        .data(),
        &V::Integer(8)
    );
    let count = function(
        "count",
        vec![("n", D::Integer)],
        D::Integer,
        binary(B::Add, int(1), call("count", vec![name("n")])),
    );
    let error =
        run_functions(&[count], &call("count", vec![int(1000)]), 8, &mut ample()).unwrap_err();
    assert!(error.error.message.contains("active function depth"));
    assert_eq!(error.error.location, Location::Query(30..40));
    assert_eq!(error.calls.len(), 8);
    let relay = function(
        "relay",
        vec![("n", D::Integer)],
        D::Integer,
        select_branch(
            binary(B::Equal, name("n"), int(0)),
            int(0),
            e(K::Require {
                condition: Box::new(literal(V::Boolean(true))),
                value: Box::new(e(K::Default {
                    value: Box::new(literal(V::Absent)),
                    fallback: Box::new(call("relay", vec![binary(B::Subtract, name("n"), int(1))])),
                })),
                message: Located::new("unused".into(), 45..51),
            }),
        ),
    );
    let entry = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("i")],
            source: e(K::Array(vec![call("relay", vec![int(500)]), int(2)])),
            filter: Some(binary(B::Equal, call("relay", vec![name("i")]), int(0))),
        })],
        head: Box::new(call("relay", vec![name("i")])),
    });
    let rows = run_functions(&[relay], &entry, 1, &mut ample()).unwrap();
    assert!(
        matches!(rows.data(),V::Array(values) if values.len()==2 && values.iter().all(|value|value.data()==&V::Integer(0)))
    );
    let optional = function(
        "optional",
        vec![],
        D::Optional(Box::new(D::Integer)),
        call("required", vec![]),
    );
    let required = function("required", vec![], D::Integer, call("optional", vec![]));
    let error = run_functions(
        &[optional, required],
        &call("optional", vec![]),
        4,
        &mut ample(),
    )
    .unwrap_err();
    assert!(error.error.message.contains("active function depth"));
    let typed = function(
        "typed",
        vec![],
        D::Integer,
        e(K::Default {
            value: Box::new(literal(
                zirql::declare_constructed(
                    V::Absent,
                    D::Optional(Box::new(D::Integer)),
                    &mut ample(),
                    Location::Query(0..1),
                )
                .unwrap(),
            )),
            fallback: Box::new(call("typed", vec![])),
        }),
    );
    let error = run_functions(&[typed], &call("typed", vec![]), 4, &mut ample()).unwrap_err();
    assert!(error.error.message.contains("active function depth"));
}

#[test]
fn function_limits_types_and_preflight_restore_the_active_budget() {
    let spin = function("spin", vec![], D::Integer, call("spin", vec![]));
    let error = run_functions(
        &[spin],
        &call("spin", vec![]),
        1,
        &mut budget(20_000, 100_000, 64),
    )
    .unwrap_err();
    assert_eq!(error.error.category, zirql::QueryErrorCategory::WorkLimit);
    assert_eq!(error.error.location, Location::Query(30..40));
    assert_eq!(error.calls.len(), 1);
    let branch = function(
        "branch",
        vec![("n", D::Integer)],
        D::Integer,
        select_branch(
            binary(B::Equal, name("n"), int(0)),
            int(1),
            binary(
                B::Add,
                call("branch", vec![binary(B::Subtract, name("n"), int(1))]),
                call("branch", vec![binary(B::Subtract, name("n"), int(1))]),
            ),
        ),
    );
    let error = run_functions(
        &[branch],
        &call("branch", vec![int(12)]),
        16,
        &mut budget(20_000, 100_000, 64),
    )
    .unwrap_err();
    assert_eq!(error.error.category, zirql::QueryErrorCategory::WorkLimit);
    let mut shared = ample();
    let wrong = function(
        "wrong",
        vec![("x", D::Integer)],
        D::Integer,
        literal(V::String("bad".into())),
    );
    let error = run_functions(&[wrong], &call("wrong", vec![int(1)]), 2, &mut shared).unwrap_err();
    assert_eq!(error.error.location, Location::Query(0..1));
    assert_eq!(
        evaluate_value(&int(9), &Bindings::default(), &mut shared).unwrap(),
        V::Integer(9)
    );
    let identity = function("id", vec![("x", D::Integer)], D::Integer, name("x"));
    let argument = Located::new(K::Literal(V::String("bad".into())), 60..65);
    let error =
        run_functions(&[identity], &call("id", vec![argument]), 2, &mut shared).unwrap_err();
    assert_eq!(error.error.location, Location::Query(60..65));
    for bad in [
        function("unused", vec![], D::Integer, name("missing")),
        function(
            "unused",
            vec![],
            D::Integer,
            select_branch(literal(V::Boolean(false)), call("missing", vec![]), int(0)),
        ),
        function("sum", vec![], D::Integer, int(0)),
        function("map_by", vec![], D::Integer, int(0)),
        function(
            "duplicate",
            vec![("x", D::Integer), ("x", D::Integer)],
            D::Integer,
            int(0),
        ),
    ] {
        assert!(run_functions(&[bad], &int(0), 2, &mut ample()).is_err());
    }
    let mut bad = function("shadow", vec![("x", D::Integer)], D::Integer, name("x"));
    bad.bindings
        .push((Located::new("x".into(), 70..71), int(1)));
    assert!(run_functions(&[bad], &int(0), 2, &mut ample()).is_err());
    let bad = function("arity", vec![], D::Integer, call("arity", vec![int(1)]));
    assert!(run_functions(&[bad], &int(0), 2, &mut ample()).is_err());
}

#[test]
fn exact_string_matching_checks_types_and_admits_scans() {
    let text = |value: &str| literal(V::String(value.into()));
    let expression = e(K::Array(vec![
        binary(B::Contains, text("Ångström"), text("ström")),
        binary(B::StartsWith, text("Ångström"), text("ång")),
        binary(B::EndsWith, text("Ångström"), text("")),
        binary(B::Contains, text("é"), text("é")),
        binary(B::StartsWith, text(""), text("")),
    ]));
    assert_eq!(
        evaluate(&expression).unwrap(),
        V::Array(
            vec![true, false, true, false, true]
                .into_iter()
                .map(V::Boolean)
                .collect()
        )
    );
    let wrong_type = Located::new(binary(B::Contains, text("name"), int(1)).value, 40..58);
    let skipped = e(K::Conditional {
        branches: vec![(literal(V::Boolean(false)), wrong_type)],
        otherwise: Box::new(literal(V::Boolean(false))),
    });
    let refusal = evaluate(&skipped).unwrap_err();
    assert_eq!(refusal.location, Location::Query(40..58));
    assert_eq!(refusal.message, "string matching needs two strings");

    let bindings = Bindings::new([(
        Located::new("long".into(), 0..1),
        V::String("a".repeat(4_000)),
    )])
    .unwrap();
    let expression = Located::new(binary(B::Contains, name("long"), text("z")).value, 60..80);
    let refusal = evaluate_value(&expression, &bindings, &mut budget(1_000, 100, 64)).unwrap_err();
    assert_eq!(refusal.location, Location::Query(60..80));
    assert!(refusal.message.contains("work limit"));
    // Prefix and empty-search checks borrow the shared text without copying it.
    let prefix = binary(B::StartsWith, name("long"), text("a"));
    assert_eq!(
        evaluate_value(&prefix, &bindings, &mut budget(1_000, 100, 64)).unwrap(),
        V::Boolean(true)
    );
    let empty = binary(B::Contains, name("long"), text(""));
    assert_eq!(
        evaluate_value(&empty, &bindings, &mut budget(1_000, 100, 64)).unwrap(),
        V::Boolean(true)
    );
}

fn sorting(array: E, field: Option<&str>, descending: bool) -> E {
    Located::new(
        K::Sort {
            array: Box::new(array),
            selector: field
                .map(|field| Located::new(zirql::SortSelector::Field(field.into()), 5..12)),
            descending,
        },
        2..40,
    )
}
#[test]
fn sets_keep_insertion_order_first_representatives_and_membership_after_sorting() {
    let set = || e(K::Set(vec![int(3), int(1), int(3), int(2)]));
    let value = evaluate(&set()).unwrap();
    let expected = V::Set {
        element_type: Some(SetElementType::Integer),
        elements: vec![V::Integer(3), V::Integer(1), V::Integer(2)].into(),
    };
    assert_eq!(value, expected);
    admit_constructed(
        &value,
        &D::<()>::Set(SetElementType::Integer),
        &mut ample(),
        Location::Source(2..8),
    )
    .unwrap();
    assert_eq!(
        copy_constructed_value(&value, &mut ample(), Location::Source(2..8)).unwrap(),
        value
    );
    let sorted = sorting(set(), None, false);
    assert_eq!(
        evaluate(&binary(B::Equal, set(), sorted)).unwrap(),
        V::Boolean(true)
    );
    assert_eq!(
        evaluate(&sorting(set(), None, false)).unwrap(),
        V::Set {
            element_type: Some(SetElementType::Integer),
            elements: vec![V::Integer(1), V::Integer(2), V::Integer(3)].into(),
        }
    );
    let comprehension = e(K::Comprehension {
        constructor: ValueConstructor::Set,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("x")],
            source: e(K::Array(vec![int(3), int(1), int(3), int(2)])),
            filter: None,
        })],
        head: Box::new(name("x")),
    });
    assert_eq!(evaluate(&comprehension).unwrap(), expected);
    let zeros = evaluate(&e(K::Set(vec![
        literal(V::Float(-0.0)),
        literal(V::Float(0.0)),
        literal(V::Float(2.0)),
    ])))
    .unwrap();
    let V::Set { elements, .. } = zeros else {
        panic!("set")
    };
    assert!(matches!(elements[0], V::Float(value) if value.is_sign_negative()));
    assert_eq!(elements.len(), 2);
    assert_eq!(
        evaluate(&sorting(
            e(K::Set(vec![
                literal(V::Boolean(true)),
                literal(V::Boolean(false))
            ])),
            None,
            false
        ))
        .unwrap(),
        V::Set {
            element_type: Some(SetElementType::Boolean),
            elements: vec![V::Boolean(false), V::Boolean(true)].into()
        }
    );
    let empty = || {
        literal(declared(
            V::Set {
                element_type: Some(SetElementType::Member(NominalTypeId(7))),
                elements: vec![].into(),
            },
            D::Set(SetElementType::Member(NominalTypeId(7))),
        ))
    };
    let sorted = evaluate(&sorting(empty(), None, false)).unwrap();
    assert!(
        matches!(sorted, V::Declared { declaration: D::Set(SetElementType::Member(NominalTypeId(7))), value } if matches!(&*value, V::Set { elements, .. } if elements.is_empty()))
    );
    assert!(
        matches!(&value_expression_types(&sorting(empty(), None, false), &BTreeMap::new(), &mut ample()).unwrap()[0], ValueType::Declared { actual, .. } if matches!(actual.as_ref(), ValueType::Set(Some(SetElementType::Member(NominalTypeId(7))))))
    );
    let mut contradictory = member(22, 4);
    contradictory.member_name = "contradiction".into();
    assert!(
        evaluate(&e(K::Set(vec![
            member_expression(22, 4),
            member_expression(11, 1),
            literal(V::Member(contradictory))
        ])))
        .is_err()
    );
}

#[test]
fn dictionaries_keep_kind_stable_current_ties_static_fields_and_first_seen_tally() {
    let run = |source: &str| {
        let mut host = WordInput {
            value: V::Array(vec![]),
            unknown: false,
        };
        let expression = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(source, 0).unwrap(),
            &mut host,
            &mut ample(),
            64,
        )
        .unwrap();
        zirql::evaluate_query_value(&expression, &BTreeMap::new(), &mut host, &mut ample())
    };
    let entries = || record(vec![("z", int(2)), ("a", int(2)), ("m", int(1))]);
    let original = evaluate(&entries()).unwrap();
    assert_eq!(
        original,
        V::Record(vec![
            ("z".into(), V::Integer(2)),
            ("a".into(), V::Integer(2)),
            ("m".into(), V::Integer(1))
        ])
    );
    for (source, expected) in [
        (r#"sort({"z": 2, "a": 2, "m": 1})"#, ["a", "m", "z"]),
        (
            r#"sort_by({"z": 2, "a": 2, "m": 1}, value)"#,
            ["m", "z", "a"],
        ),
        (
            r#"sort_by({"z": 2, "a": 2, "m": 1}, value, descending=true)"#,
            ["z", "a", "m"],
        ),
        (
            r#"sort_by({"z": {"value": 2}, "a": {"value": 2}, "m": {"value": 1}}, "value", descending=true)"#,
            ["z", "a", "m"],
        ),
    ] {
        let V::Record(fields) = run(source).unwrap() else {
            panic!("dictionary")
        };
        assert_eq!(
            fields
                .iter()
                .map(|(label, _)| label.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
    assert_eq!(
        evaluate(&binary(
            B::Equal,
            entries(),
            sorting(entries(), None, false)
        ))
        .unwrap(),
        V::Boolean(true)
    );
    assert_eq!(
        run(r#"sort({"z": true, "a": [1]})"#).unwrap(),
        V::Record(vec![
            ("a".into(), V::Array(vec![V::Integer(1)])),
            ("z".into(), V::Boolean(true))
        ])
    );
    assert_eq!(run("sort({})").unwrap(), V::Record(vec![]));
    assert_eq!(run("sort_by({}, value)").unwrap(), V::Record(vec![]));
    assert_eq!(run("sort([])").unwrap(), V::Array(vec![]));
    assert_eq!(
        run(r#"["z", "a", "z", "m", "a"] | tally"#).unwrap(),
        V::Record(vec![
            ("z".into(), V::Integer(2)),
            ("a".into(), V::Integer(2)),
            ("m".into(), V::Integer(1))
        ])
    );
    let missing = r#"sort_by({"z": {"other": 1}}, "value")"#;
    assert_eq!(run(missing).unwrap_err().location, Location::Query(29..36));
    assert!(run(r#"sort_by({"z": {"value": 1}}, value)"#).is_err());
    let empty_boolean = sorting(
        literal(declared(
            V::Array(vec![]),
            D::Sequence(Box::new(D::Boolean)),
        )),
        None,
        false,
    );
    assert!(evaluate(&choose(empty_boolean)).is_err());
    let incompatible_selector = e(K::Sort {
        array: Box::new(e(K::Array(vec![]))),
        selector: Some(Located::new(zirql::SortSelector::Value, 5..10)),
        descending: false,
    });
    assert_eq!(
        evaluate(&choose(incompatible_selector))
            .unwrap_err()
            .location,
        Location::Query(5..10)
    );
}

fn extremum(operator: zirql::ValueExtremum, array: E, field: Option<&str>) -> E {
    Located::new(
        K::Extremum {
            operator,
            array: Box::new(array),
            field: field.map(|field| Located::new(field.into(), 5..12)),
        },
        2..40,
    )
}

#[test]
fn report_ordering_keeps_original_rows_stable_ties_and_optional_extrema() {
    use zirql::ValueExtremum::{Max, Min};
    let row = |id: &str, score| {
        V::Record(vec![
            ("id".into(), V::String(id.into())),
            ("score".into(), V::Integer(score)),
            (
                "payload".into(),
                V::Tuple(vec![V::Member(member(9, 3)), V::Absent]),
            ),
        ])
    };
    let rows = V::Array(vec![row("A", 2), row("B", 1), row("C", 2), row("D", 1)]);
    let types = BTreeMap::from([(
        "rows".into(),
        constructed_value_type(&rows, &mut ample(), Location::Source(0..4)).unwrap(),
    )]);
    let ordered = sorting(name("rows"), Some("score"), false);
    validate_value_expression(&ordered, &types, &mut ample()).unwrap();
    assert_eq!(
        free_value_names(&ordered, &types, &mut ample()).unwrap(),
        vec![Located::new("rows".into(), 0..1)]
    );
    let facts = value_expression_types(&ordered, &types, &mut ample()).unwrap();
    let ValueType::Array(ArrayType::Elements(elements)) = &facts[0] else {
        panic!("reordered element facts")
    };
    let ValueType::Array(ArrayType::Positions(original)) = &types["rows"] else {
        panic!("input positions")
    };
    assert_eq!(elements, original);
    let report = record(vec![
        ("ascending", ordered),
        ("descending", sorting(name("rows"), Some("score"), true)),
        ("min", extremum(Min, name("rows"), Some("score"))),
        ("max", extremum(Max, name("rows"), Some("score"))),
        (
            "scalar",
            sorting(
                e(K::Array(vec![int(2), int(1), int(2), int(1)])),
                None,
                false,
            ),
        ),
        ("empty_sort", sorting(e(K::Array(vec![])), None, true)),
        ("empty_min", extremum(Min, e(K::Array(vec![])), None)),
        (
            "empty_max",
            default(extremum(Max, e(K::Array(vec![])), Some("score")), int(7)),
        ),
    ]);
    let bindings = Bindings::new([(Located::new("rows".into(), 0..0), rows)]).unwrap();
    let result = evaluate_value(&report, &bindings, &mut ample()).unwrap();
    assert_eq!(
        result,
        V::Record(vec![
            (
                "ascending".into(),
                V::Array(vec![row("B", 1), row("D", 1), row("A", 2), row("C", 2)])
            ),
            (
                "descending".into(),
                V::Array(vec![row("A", 2), row("C", 2), row("B", 1), row("D", 1)])
            ),
            ("min".into(), row("B", 1)),
            ("max".into(), row("A", 2)),
            (
                "scalar".into(),
                V::Array(vec![
                    V::Integer(1),
                    V::Integer(1),
                    V::Integer(2),
                    V::Integer(2)
                ])
            ),
            ("empty_sort".into(), V::Array(vec![])),
            ("empty_min".into(), V::Absent),
            ("empty_max".into(), V::Integer(7)),
        ])
    );
    assert!(
        matches!(bindings.get("rows", 0..0).unwrap().data(), V::Array(rows) if rows[0] == row("A", 2))
    );
}

#[test]
fn ordering_uses_exact_scalar_types_nominal_order_and_retained_declarations() {
    use zirql::ValueExtremum::{Max, Min};
    assert_eq!(
        evaluate(&sorting(
            e(K::Array(vec![
                text("é"),
                text("a"),
                text("A"),
                text("a\u{301}")
            ])),
            None,
            false
        ))
        .unwrap(),
        V::Array(vec![
            V::String("A".into()),
            V::String("a".into()),
            V::String("a\u{301}".into()),
            V::String("é".into())
        ])
    );
    assert_eq!(
        evaluate(&sorting(
            e(K::Array(vec![
                literal(V::Float(3.5)),
                literal(V::Float(-0.0)),
                literal(V::Float(0.0)),
                literal(V::Float(-2.5))
            ])),
            None,
            true
        ))
        .unwrap(),
        V::Array(vec![
            V::Float(3.5),
            V::Float(-0.0),
            V::Float(0.0),
            V::Float(-2.5)
        ])
    );
    assert_eq!(
        evaluate(&sorting(
            e(K::Array(vec![
                member_expression(30, 2),
                member_expression(10, 0),
                member_expression(20, 1)
            ])),
            None,
            false
        ))
        .unwrap(),
        V::Array(vec![
            V::Member(member(10, 0)),
            V::Member(member(20, 1)),
            V::Member(member(30, 2))
        ])
    );
    assert_eq!(
        evaluate(&extremum(
            Max,
            e(K::Array(vec![text("A"), text("é")])),
            None
        ))
        .unwrap(),
        V::String("é".into())
    );
    assert_eq!(
        evaluate(&extremum(
            Min,
            e(K::Array(vec![
                literal(V::Float(3.5)),
                literal(V::Float(-2.5))
            ])),
            None
        ))
        .unwrap(),
        V::Float(-2.5)
    );
    let schema = D::Record(vec![
        ("score".into(), D::Integer),
        ("caption".into(), D::Optional(Box::new(D::String))),
    ]);
    let rows = declared(
        V::Array(vec![
            V::Record(vec![("score".into(), V::Integer(2))]),
            V::Record(vec![("score".into(), V::Integer(1))]),
        ]),
        D::Sequence(Box::new(schema.clone())),
    );
    let bindings = Bindings::new([(Located::new("rows".into(), 0..0), rows)]).unwrap();
    let selected = evaluate_value(
        &extremum(Min, name("rows"), Some("score")),
        &bindings,
        &mut ample(),
    )
    .unwrap();
    assert!(matches!(&selected, V::Declared { declaration, .. } if declaration == &schema));
    let sorted = evaluate_value(
        &sorting(name("rows"), Some("score"), false),
        &bindings,
        &mut ample(),
    )
    .unwrap();
    assert!(
        matches!(&sorted, V::Declared { declaration: D::Sequence(child), .. } if child.as_ref() == &schema)
    );
    let expression = lookup(
        default(
            extremum(Min, name("rows"), Some("score")),
            literal(declared(
                V::Record(vec![("score".into(), V::Integer(0))]),
                schema,
            )),
        ),
        text("caption"),
    );
    let omitted = evaluate_value(&expression, &bindings, &mut ample()).unwrap();
    assert_eq!(omitted.data(), &V::Absent);
    admit_constructed(
        &omitted,
        &D::<()>::Optional(Box::new(D::String)),
        &mut ample(),
        Location::Query(0..1),
    )
    .unwrap();
    let empty = literal(declared(
        V::Array(vec![]),
        D::Sequence(Box::new(D::Integer)),
    ));
    let selected = evaluate(&extremum(Max, empty, None)).unwrap();
    assert!(
        matches!(selected, V::Declared { declaration: D::Optional(child), value } if *child == D::Integer && *value == V::Absent)
    );
    let projected = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("row")],
            source: e(K::Array(vec![
                record(vec![]),
                record(vec![("score", int(2))]),
            ])),
            filter: None,
        })],
        head: Box::new(record(vec![(
            "score",
            default(lookup(name("row"), text("score")), int(0)),
        )])),
    });
    assert_eq!(
        evaluate(&extremum(Min, projected, Some("score"))).unwrap(),
        V::Record(vec![("score".into(), V::Integer(0))])
    );
}

#[test]
fn ordering_refuses_later_bad_keys_and_known_incompatible_skipped_types_at_the_selector() {
    use zirql::ValueExtremum::Min;
    for bad in [
        record(vec![]),
        record(vec![("score", literal(V::Absent))]),
        record(vec![("score", text("1"))]),
    ] {
        let expression = extremum(
            Min,
            e(K::Array(vec![record(vec![("score", int(1))]), bad])),
            Some("score"),
        );
        assert_eq!(
            evaluate(&expression).unwrap_err().location,
            Location::Query(5..12)
        );
    }
    let missing = sorting(e(K::Array(vec![record(vec![])])), Some("score"), false);
    assert_eq!(evaluate(&choose(missing)).unwrap(), V::Integer(7));
    let incompatible = sorting(
        e(K::Array(vec![record(vec![(
            "score",
            literal(V::Boolean(true)),
        )])])),
        Some("score"),
        false,
    );
    assert_eq!(
        evaluate(&choose(incompatible)).unwrap_err().location,
        Location::Query(5..12)
    );
    let mut foreign = member(10, 0);
    foreign.type_id = NominalTypeId(8);
    assert!(
        evaluate(&sorting(
            e(K::Array(vec![
                member_expression(10, 0),
                literal(V::Member(foreign))
            ])),
            None,
            false
        ))
        .is_err()
    );
    assert!(
        evaluate(&sorting(
            e(K::Array(vec![int(1), literal(V::Float(2.0))])),
            None,
            false
        ))
        .is_err()
    );
    assert!(
        evaluate(&extremum(
            Min,
            literal(V::Array(vec![V::Float(f64::INFINITY)])),
            None
        ))
        .is_err()
    );
    let expression = sorting(
        e(K::Array(vec![
            text("common-prefix-b"),
            text("common-prefix-a"),
        ])),
        None,
        false,
    );
    let mut allowance = ample();
    let failure = zirql::evaluate_value_with_host_validation(
        &expression,
        &Bindings::default(),
        &mut allowance,
        |value, budget, location| {
            if matches!(value, V::Array(_)) {
                // A host scan can leave too little allowance for sort scratch.
                budget.charge(budget.remaining_work().saturating_sub(2), location.clone())?;
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(failure.category, zirql::QueryErrorCategory::WorkLimit);
    assert_eq!(failure.location, Location::Query(2..40));
}

#[test]
fn scalar_pipelines_keep_stable_equality_declarations_and_nonpositional_facts() {
    let input = || e(K::Array(vec![text("ash"), text("apple"), text("apple")]));
    let unique = e(K::Unique(Box::new(input())));
    assert_eq!(
        evaluate(&unique).unwrap(),
        V::Array(vec![V::String("ash".into()), V::String("apple".into())])
    );
    assert_eq!(
        evaluate(&e(K::Unique(Box::new(sorting(input(), None, false))))).unwrap(),
        V::Array(vec![V::String("apple".into()), V::String("ash".into())])
    );
    let first_ash = || declared(V::String("ash".into()), D::String);
    let first_apple = || declared(V::String("apple".into()), D::Optional(Box::new(D::String)));
    let annotated = declared(
        V::Array(vec![
            first_ash(),
            first_apple(),
            V::String("ash".into()),
            declared(V::String("apple".into()), D::String),
            V::String("birch".into()),
            declared(V::String("birch".into()), D::String),
        ]),
        D::Sequence(Box::new(D::String)),
    );
    assert_eq!(
        evaluate(&e(K::Unique(Box::new(literal(annotated))))).unwrap(),
        declared(
            V::Array(vec![first_ash(), first_apple(), V::String("birch".into())]),
            D::Sequence(Box::new(D::String)),
        )
    );
    let tally = e(K::Tally(Box::new(input())));
    let value = evaluate(&tally).unwrap();
    assert_eq!(
        value,
        V::Record(vec![
            ("ash".into(), V::Integer(1)),
            ("apple".into(), V::Integer(2))
        ])
    );
    assert_eq!(
        copy_constructed_value(&value, &mut ample(), Location::Query(0..1)).unwrap(),
        value
    );
    assert_eq!(
        evaluate(&e(K::Unique(Box::new(literal(V::Array(vec![])))))).unwrap(),
        V::Array(vec![])
    );
    assert_eq!(
        evaluate(&e(K::Tally(Box::new(literal(V::Array(vec![])))))).unwrap(),
        V::Record(vec![])
    );
    let zeros = evaluate(&e(K::Unique(Box::new(literal(V::Array(vec![
        V::Float(-0.0),
        V::Float(0.0),
        V::Float(2.0),
    ]))))))
    .unwrap();
    assert_eq!(zeros, V::Array(vec![V::Float(-0.0), V::Float(2.0)]));
    assert!(
        matches!(&zeros, V::Array(values) if matches!(values[0], V::Float(value) if value.is_sign_negative()))
    );
    assert_eq!(
        evaluate(&e(K::Unique(Box::new(e(K::Array(vec![
            member_expression(20, 1),
            member_expression(10, 0),
            member_expression(20, 1)
        ]))))))
        .unwrap(),
        V::Array(vec![V::Member(member(20, 1)), V::Member(member(10, 0))])
    );
    let declared_empty = || literal(declared(V::Array(vec![]), D::Sequence(Box::new(D::String))));
    let expression = e(K::Unique(Box::new(declared_empty())));
    assert!(
        matches!(evaluate(&expression).unwrap(), V::Declared { declaration: D::Sequence(child), .. } if *child == D::String)
    );
    let facts = value_expression_types(&unique, &BTreeMap::new(), &mut ample()).unwrap();
    assert!(
        matches!(&facts[..], [ValueType::Array(ArrayType::Elements(elements))] if elements.iter().all(|t| *t == ValueType::String))
    );
    let expression = e(K::Unique(Box::new(name("words"))));
    assert_eq!(
        free_value_names(
            &expression,
            &BTreeMap::from([("words".into(), ValueType::Unestablished)]),
            &mut ample()
        )
        .unwrap()[0]
            .value,
        "words"
    );
    assert!(
        validate_value_expression(
            &expression,
            &BTreeMap::from([(
                "words".into(),
                ValueType::Array(ArrayType::Elements(vec![ValueType::String]))
            )]),
            &mut ample()
        )
        .is_ok()
    );
}

struct WordInput {
    value: V,
    unknown: bool,
}
impl zirql::ValueReaderHost for WordInput {
    type Stage = ();
    type SetOperator = ();
    type Emitter = ();
    type Predicate = ();
    fn field_pipeline_source(&self) -> zirql::FieldPipelineSource {
        zirql::FieldPipelineSource::Constructed
    }
    fn is_binding(&self, name: &str) -> bool {
        name == "words"
    }
    fn is_stage(&self, _: &str) -> bool {
        false
    }
    fn parse_stage(
        &mut self,
        reader: &mut zirql::ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<zirql::ExpressionStep<(), ()>, zirql::QueryError> {
        Err(reader.error("no native stages"))
    }
    fn parse_predicate_leaf(
        &mut self,
        reader: &mut zirql::ArgumentReader<'_>,
        _: &mut Budget,
    ) -> Result<(), zirql::QueryError> {
        Err(reader.error("no native predicates"))
    }
    fn filter(&mut self, _: zirql::Predicate<()>, _: std::ops::Range<usize>) {}
    fn projection(
        &mut self,
        _: zirql::Projection<zirql::ReadExpression<Self>>,
        _: std::ops::Range<usize>,
    ) {
    }
    fn set_operator(&self, _: &str) -> Option<()> {
        None
    }
}
impl<Q> zirql::ValueQueryHost<Q> for WordInput {
    type Error = zirql::QueryError;
    fn current_input_types(
        &mut self,
        range: &std::ops::Range<usize>,
        budget: &mut Budget,
    ) -> Result<Vec<ValueType>, Self::Error> {
        Ok(vec![if self.unknown {
            ValueType::Unestablished
        } else {
            constructed_value_type(&self.value, budget, Location::Query(range.clone()))?
        }])
    }
    fn read_current_input(
        &mut self,
        range: &std::ops::Range<usize>,
        budget: &mut Budget,
    ) -> Result<V, Self::Error> {
        copy_constructed_value(&self.value, budget, Location::Query(range.clone()))
    }
    fn query_types(
        &mut self,
        _: &Q,
        _: &[(&str, Vec<ValueType>)],
        _: &mut Budget,
        range: &std::ops::Range<usize>,
    ) -> Result<Vec<ValueType>, Self::Error> {
        Err(zirql::QueryError::query(range.clone(), "no native queries"))
    }
    fn evaluate_query(&mut self, _: &Q, _: &[(&str, V)], _: &mut Budget) -> Result<V, Self::Error> {
        unreachable!()
    }
    fn read_binding(
        &mut self,
        _: &str,
        range: &std::ops::Range<usize>,
        budget: &mut Budget,
    ) -> Result<V, Self::Error> {
        copy_constructed_value(&self.value, budget, Location::Query(range.clone()))
    }
}

fn read_word_expression(
    source: &str,
    host: &mut WordInput,
    allowance: &mut Budget,
) -> E<zirql::ReadExpression<WordInput>> {
    let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
    let expression = zirql::read_value_expression(&mut reader, host, allowance, 64).unwrap();
    assert!(reader.remaining().trim().is_empty());
    expression
}

#[test]
fn string_sets_preserve_first_spelling_and_share_scalar_set_operations() {
    let mut host = WordInput {
        value: V::Array(vec![]),
        unknown: false,
    };
    let run = |source: &str, host: &mut WordInput| {
        let mut allowance = ample();
        let expression = read_word_expression(source, host, &mut allowance);
        let result =
            zirql::evaluate_query_value(&expression, &BTreeMap::new(), host, &mut allowance);
        zirql::dispose_native_value(expression);
        result
    };
    let expected = V::Set {
        element_type: Some(SetElementType::String),
        elements: vec![V::String("birch".into()), V::String("apple".into())].into(),
    };
    let value = run(r#"{"birch", "apple", "birch"}"#, &mut host).unwrap();
    assert_eq!(value, expected);
    admit_constructed(
        &value,
        &D::<()>::Set(SetElementType::String),
        &mut ample(),
        Location::Source(0..1),
    )
    .unwrap();
    assert_eq!(
        copy_constructed_value(&value, &mut ample(), Location::Source(0..1)).unwrap(),
        value
    );
    assert_eq!(
        constructed_value_type(&value, &mut ample(), Location::Source(0..1)).unwrap(),
        ValueType::Set(Some(SetElementType::String))
    );
    assert_eq!(
        run(r#"{w | w in ["birch", "apple", "birch"]}"#, &mut host).unwrap(),
        expected
    );
    assert_eq!(run(r#"["apple" in {"birch", "apple"}, {"birch", "apple"} == {"apple", "birch"}, "Apple" in {"apple"}]"#, &mut host).unwrap(), V::Array(vec![V::Boolean(true), V::Boolean(true), V::Boolean(false)]));
    assert_eq!(
        run(r#"sort({"birch", "apple"})"#, &mut host).unwrap(),
        V::Set {
            element_type: Some(SetElementType::String),
            elements: vec![V::String("apple".into()), V::String("birch".into())].into()
        }
    );
    assert_eq!(
        run("set<string>()", &mut host).unwrap(),
        V::Set {
            element_type: Some(SetElementType::String),
            elements: vec![].into()
        }
    );
    assert!(matches!(
        run(r#"{"apple", 1}"#, &mut host).unwrap(),
        V::Set {
            element_type: Some(SetElementType::Value),
            ..
        }
    ));
}

#[test]
fn neighbouring_word_sets_tally_unordered_keys_in_first_seen_order() {
    const QUERY: &str =
        "[{words[i], words[i + 1]} | i in index_set(words) where i < length(words)] | tally";
    let group = |words: &[&str], count| {
        (
            V::Set {
                element_type: Some(SetElementType::String),
                elements: words.iter().map(|word| V::String((*word).into())).collect(),
            },
            V::Integer(count),
        )
    };
    for (words, expected) in [
        (
            vec!["ash", "apple", "birch", "apple"],
            V::Dictionary(vec![group(&["ash", "apple"], 1), group(&["apple", "birch"], 2)].into()),
        ),
        (
            vec!["apple", "apple", "apple"],
            V::Dictionary(vec![group(&["apple"], 2)].into()),
        ),
        (vec!["ash"], V::Dictionary(vec![].into())),
        (vec![], V::Dictionary(vec![].into())),
    ] {
        let mut allowance = ample();
        let mut host = WordInput {
            value: V::Array(
                words
                    .into_iter()
                    .map(|word| V::String(word.into()))
                    .collect(),
            ),
            unknown: false,
        };
        let expression = read_word_expression(QUERY, &mut host, &mut allowance);
        let types = BTreeMap::from([(
            "words".into(),
            vec![ValueType::Array(ArrayType::Elements(vec![
                ValueType::String,
            ]))],
        )]);
        let discovered =
            zirql::query_value_types(&expression, &types, &mut host, &mut allowance).unwrap();
        assert_eq!(
            discovered,
            vec![ValueType::Dictionary {
                keys: vec![ValueType::Set(Some(SetElementType::String))],
                values: vec![ValueType::Integer],
            }]
        );
        assert_eq!(
            zirql::evaluate_query_value(&expression, &types, &mut host, &mut allowance).unwrap(),
            expected
        );
        zirql::dispose_native_value(expression);
    }
    for (query, count) in [
        ("[{w for w in words if false}] | tally", 1),
        ("[{w for w in words if false}, set<string>()] | tally", 2),
    ] {
        let mut allowance = ample();
        let mut host = WordInput {
            value: V::Array(vec![V::String("ash".into())]),
            unknown: false,
        };
        let expression = read_word_expression(query, &mut host, &mut allowance);
        let types = BTreeMap::from([(
            "words".into(),
            vec![ValueType::Array(ArrayType::Elements(vec![
                ValueType::String,
            ]))],
        )]);
        let value =
            zirql::evaluate_query_value(&expression, &types, &mut host, &mut allowance).unwrap();
        assert!(
            matches!(value, V::Dictionary(ref entries) if entries.len() == 1 && entries[0].1 == V::Integer(count) && matches!(entries[0].0, V::Set { element_type: Some(SetElementType::String), ref elements } if elements.is_empty()))
        );
        zirql::dispose_native_value(expression);
    }
    let unestablished = e(K::Tally(Box::new(e(K::Array(vec![literal(V::Set {
        element_type: None,
        elements: vec![].into(),
    })])))));
    assert!(matches!(
        evaluate(&unestablished).unwrap(),
        V::Dictionary(_)
    ));
    let empty_strings = literal(declared(V::Array(vec![]), D::Sequence(Box::new(D::String))));
    assert_eq!(
        evaluate(&e(K::Tally(Box::new(empty_strings)))).unwrap(),
        V::Record(vec![])
    );
    let empty_sets = literal(declared(
        V::Array(vec![]),
        D::Sequence(Box::new(D::Set(SetElementType::String))),
    ));
    assert_eq!(
        evaluate(&e(K::Tally(Box::new(empty_sets)))).unwrap(),
        V::Dictionary(vec![].into())
    );
    assert_eq!(
        evaluate(&e(K::Tally(Box::new(e(K::Array(vec![])))))).unwrap(),
        V::Record(vec![])
    );
    let empty_set = || {
        literal(V::Set {
            element_type: Some(SetElementType::String),
            elements: vec![].into(),
        })
    };
    let repeated_empty = e(K::Tally(Box::new(e(K::Array(vec![
        empty_set(),
        empty_set(),
    ])))));
    assert_eq!(
        evaluate(&repeated_empty).unwrap(),
        V::Dictionary(vec![group(&[], 2)].into())
    );
    let mixed = e(K::Tally(Box::new(e(K::Array(vec![
        e(K::Set(vec![text("ash")])),
        text("ash"),
    ])))));
    assert!(matches!(evaluate(&mixed).unwrap(), V::Dictionary(ref entries) if entries.len() == 2));
}
#[test]
fn general_collection_keys_lookup_deduplicate_and_sort_structurally() {
    let run = |source: &str| {
        let mut host = WordInput {
            value: V::Array(vec![]),
            unknown: false,
        };
        let mut budget = ample();
        let expression = read_word_expression(source, &mut host, &mut budget);
        let result =
            zirql::evaluate_query_value(&expression, &BTreeMap::new(), &mut host, &mut budget);
        zirql::dispose_native_value(expression);
        result
    };
    let query = r#"[
        {{"ash", "apple"}: 7}[{"apple", "ash"}],
        {[1, {"x": 2}]: 9}[[1, {"x": 2}]],
        {(1, [2]): 3}[(1, [2])],
        {{"a": 1, "b": 2}: 4}[{"b": 2, "a": 1}],
        {2, 1} in {{1, 2}, {3}},
        {[1], (1, 2), {"a": 1}, {{2, 1}}, {{1, 2}}} ==
            {{{1, 2}}, {"a": 1}, (1, 2), [1]}
    ]"#;
    assert_eq!(
        run(query).unwrap(),
        V::Array(vec![
            V::Integer(7),
            V::Integer(9),
            V::Integer(3),
            V::Integer(4),
            V::Boolean(true),
            V::Boolean(true)
        ])
    );
    assert_eq!(run("lookup({[x]: 1 for x in []}, [1])").unwrap(), V::Absent);
    assert!(
        matches!(run("{[x]: 1 for x in []}").unwrap(), V::Dictionary(ref entries) if entries.is_empty())
    );
    assert_eq!(
        run(r#"{x: 1 for x in ["one"] if false}"#).unwrap(),
        V::Record(vec![])
    );
    let empty_string_dictionary = e(K::DictionaryComprehension {
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("k")],
            source: literal(declared(V::Array(vec![]), D::Sequence(Box::new(D::String)))),
            filter: None,
        })],
        key: Box::new(name("k")),
        head: Box::new(int(1)),
    });
    assert_eq!(
        value_expression_types(&empty_string_dictionary, &BTreeMap::new(), &mut ample()).unwrap(),
        vec![ValueType::RecordKind]
    );
    assert_eq!(
        evaluate(&empty_string_dictionary).unwrap(),
        V::Record(vec![])
    );
    let declared_string_dictionary = e(K::Dictionary(vec![(
        literal(declared(V::String("one".into()), D::String)),
        int(1),
    )]));
    assert_eq!(
        value_expression_types(&declared_string_dictionary, &BTreeMap::new(), &mut ample())
            .unwrap(),
        vec![ValueType::RecordKind]
    );
    assert_eq!(
        evaluate(&declared_string_dictionary).unwrap(),
        V::Record(vec![("one".into(), V::Integer(1))])
    );
    let tally = run("[{1, 2}, {2, 1}, {1, 1}, {1}] | tally").unwrap();
    assert!(
        matches!(tally, V::Dictionary(ref entries) if entries.len() == 2 && entries.iter().all(|(_, count)| *count == V::Integer(2)))
    );
    assert_eq!(
        run("sort({{2}, {1}})").unwrap(),
        V::Set {
            element_type: Some(SetElementType::Value),
            elements: vec![
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(1)].into()
                },
                V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![V::Integer(2)].into()
                }
            ]
            .into()
        }
    );
    let ordered = run("sort_by({{2}: 2, {1}: 1}, value)").unwrap();
    assert!(matches!(ordered, V::Dictionary(ref entries) if entries[0].1 == V::Integer(1)));
    let error = run(r#"{[{"a": 1}]: 0, [{"a": 1}]: 1 div 0}"#).unwrap_err();
    assert!(error.message.contains("duplicate dictionary key"));
    assert_eq!(run(r#"lookup({[1]: 2}, [9])"#).unwrap(), V::Absent);
    assert!(
        run(r#"{[1]: 2}[[9]]"#)
            .unwrap_err()
            .message
            .contains("missing dictionary key")
    );
}

#[test]
fn brace_collections_share_order_scope_and_strict_dictionary_access() {
    let run = |source: &str| {
        let mut host = WordInput {
            value: V::Array(vec![]),
            unknown: false,
        };
        let mut allowance = ample();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let parsed = zirql::read_value_expression(&mut reader, &mut host, &mut allowance, 64)?;
        assert!(reader.remaining().trim().is_empty());
        let result =
            zirql::evaluate_query_value(&parsed, &BTreeMap::new(), &mut host, &mut allowance);
        zirql::dispose_native_value(parsed);
        result
    };
    let ordered_set = |values: &[i64]| V::Set {
        element_type: Some(SetElementType::Integer),
        elements: values.iter().copied().map(V::Integer).collect(),
    };
    assert_eq!(
        run("{3,1,3}").unwrap(),
        evaluate(&e(K::Set(vec![int(3), int(1), int(3)]))).unwrap()
    );
    let source = r#"{k: {n mod 3 for n in 1..5} for w in ["pear","apple"] for k in [w,w ++ "!"] if ends_with(k,"!")}"#;
    let report = V::Record(vec![
        ("pear!".into(), ordered_set(&[1, 2, 0])),
        ("apple!".into(), ordered_set(&[1, 2, 0])),
    ]);
    assert_eq!(run(source).unwrap(), report);
    let typed = e(K::DictionaryComprehension {
        clauses: vec![
            clause(G::In {
                case: false,
                binders: vec![binder("w")],
                source: e(K::Array(vec![text("pear"), text("apple")])),
                filter: None,
            }),
            clause(G::In {
                case: false,
                binders: vec![binder("k")],
                source: e(K::Array(vec![
                    name("w"),
                    binary(B::Concatenate, name("w"), text("!")),
                ])),
                filter: Some(binary(B::EndsWith, name("k"), text("!"))),
            }),
        ],
        key: Box::new(name("k")),
        head: Box::new(e(K::Comprehension {
            constructor: ValueConstructor::Set,
            clauses: vec![clause(G::In {
                case: false,
                binders: vec![binder("n")],
                source: e(K::Range {
                    lower: Box::new(int(1)),
                    upper: Box::new(int(5)),
                }),
                filter: None,
            })],
            head: Box::new(binary(B::Modulo, name("n"), int(3))),
        })),
    });
    assert_eq!(evaluate(&typed).unwrap(), report);
    assert!(
        free_value_names(&typed, &BTreeMap::new(), &mut ample())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        value_expression_types(&typed, &BTreeMap::new(), &mut ample()).unwrap(),
        vec![ValueType::RecordKind]
    );
    assert_eq!(
        copy_constructed_value(&report, &mut ample(), Location::Query(0..1)).unwrap(),
        report
    );
    assert_eq!(run("sort({3,1,3})").unwrap(), ordered_set(&[1, 3]));
    assert_eq!(
        run("if true { {3,1,3} } else { set<int>() }").unwrap(),
        ordered_set(&[3, 1])
    );
    assert_eq!(
        run(r#"if true { let k = "pear"; {k: 7}[k] } else { 0 }"#).unwrap(),
        V::Integer(7)
    );
    assert_eq!(run(r#"{"pear": 7}["pear"]"#).unwrap(), V::Integer(7));
    assert_eq!(run(r#"{"pear": <>}["pear"]"#).unwrap(), V::Absent);
    assert!(
        run(r#"{"pear": 7}["missing"]"#)
            .unwrap_err()
            .message
            .contains("missing record field")
    );
    assert_eq!(run("{}").unwrap(), V::Record(vec![]));
    assert_eq!(
        run("[set<int>(),set<bool>(),set<float>()]").unwrap(),
        V::Array(vec![
            ordered_set(&[]),
            V::Set {
                element_type: Some(SetElementType::Boolean),
                elements: vec![].into()
            },
            V::Set {
                element_type: Some(SetElementType::Float),
                elements: vec![].into()
            },
        ])
    );
    let duplicate = r#"if true { let k = "pear"; {k: 1, (k ++ ""): 1 div 0} } else { {} }"#;
    let error = run(duplicate).unwrap_err();
    assert!(
        error.message.contains("duplicate dictionary key"),
        "{error:?}"
    );
    assert_eq!(error.location, Location::Query(33..42));
    for source in ["{missing:1}", "{n for n in n}", "{n for n in [] if 1}"] {
        assert!(run(source).is_err(), "{source}");
    }
    let schema_value = literal(declared(
        V::Record(vec![("pear".into(), ordered_set(&[]))]),
        D::Record(vec![("pear".into(), D::Set(SetElementType::Integer))]),
    ));
    assert!(matches!(
        value_expression_types(
            &access(schema_value, text("pear")),
            &BTreeMap::new(),
            &mut ample()
        )
        .unwrap()[0],
        ValueType::Declared { .. }
    ));
    let functions = [function(
        "make",
        vec![("key", D::String)],
        D::Record(vec![("pear".into(), D::Set(SetElementType::Integer))]),
        e(K::Dictionary(vec![(
            name("key"),
            e(K::Set(vec![int(3), int(1), int(3)])),
        )])),
    )];
    let selected = run_functions(
        &functions,
        &access(call("make", vec![text("pear")]), text("pear")),
        2,
        &mut ample(),
    )
    .unwrap();
    assert_eq!(selected.data(), &ordered_set(&[3, 1]));
    assert!(matches!(
        selected,
        V::Declared {
            declaration: D::Set(SetElementType::Integer),
            ..
        }
    ));
    let nominal_empty = V::Set {
        element_type: Some(SetElementType::Member(NominalTypeId(7))),
        elements: vec![].into(),
    };
    let bindings = Bindings::new([(Located::new("empty".into(), 0..5), nominal_empty)]).unwrap();
    assert!(
        matches!(evaluate_value(&name("empty"), &bindings, &mut ample()).unwrap(), V::Set { element_type: Some(SetElementType::Member(NominalTypeId(7))), elements } if elements.is_empty())
    );
    let missing = literal(declared(V::Record(vec![]), D::Record(vec![])));
    assert!(
        value_expression_types(
            &access(missing, text("pear")),
            &BTreeMap::new(),
            &mut ample()
        )
        .is_err()
    );
    let mut host = WordInput {
        value: V::Array(vec![]),
        unknown: false,
    };
    let parsed = zirql::read_value_expression(
        &mut zirql::ArgumentReader::new("{n for n in 1..101 if false}", 0).unwrap(),
        &mut host,
        &mut ample(),
        64,
    )
    .unwrap();
    assert!(
        zirql::evaluate_query_value(
            &parsed,
            &BTreeMap::new(),
            &mut host,
            &mut budget(1000, 100, 64)
        )
        .is_err()
    );
    zirql::dispose_native_value(parsed);
}

#[test]
fn constructed_current_input_summaries_share_the_reader_and_callable_nodes() {
    let mut host = WordInput {
        value: V::Array(
            ["ash", "apple", "birch", "apple"]
                .into_iter()
                .map(|s| V::String(s.into()))
                .collect(),
        ),
        unknown: false,
    };
    let typed = record(vec![(
        "beginning_with_a",
        reduction(R::Length, prefix_filter(e(K::CurrentInput), "a")),
    )]);
    let types = BTreeMap::new();
    assert_eq!(
        zirql::evaluate_query_value(&typed, &types, &mut host, &mut ample()).unwrap(),
        V::Record(vec![("beginning_with_a".into(), V::Integer(3))])
    );
    assert!(
        zirql::free_query_value_names(&typed, &types, &mut host, &mut ample())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        &zirql::query_value_types(&e(K::CurrentInput), &types, &mut host, &mut ample()).unwrap()[..],
        [ValueType::Array(_)]
    ));
    for (source, expected) in [
        (
            r#"{"beginning_with_a": filter(starts_with("a")) | count}"#,
            V::Record(vec![("beginning_with_a".into(), V::Integer(3))]),
        ),
        (
            r#"{"beginning_with_a": filter(starts_with("a")) | count, "total": count}"#,
            V::Record(vec![
                ("beginning_with_a".into(), V::Integer(3)),
                ("total".into(), V::Integer(4)),
            ]),
        ),
        (
            r#"filter(starts_with("a"))"#,
            V::Array(vec![
                V::String("ash".into()),
                V::String("apple".into()),
                V::String("apple".into()),
            ]),
        ),
        (
            r#"{"explicit": ["apple", "ash", "birch", "apple"] | filter(starts_with("a")) | sort | unique | count, "total": count}"#,
            V::Record(vec![
                ("explicit".into(), V::Integer(2)),
                ("total".into(), V::Integer(4)),
            ]),
        ),
    ] {
        let mut allowance = ample();
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let parsed =
            zirql::read_value_expression(&mut reader, &mut host, &mut allowance, 64).unwrap();
        assert!(reader.remaining().trim().is_empty());
        assert_eq!(
            zirql::evaluate_query_value(&parsed, &types, &mut host, &mut allowance).unwrap(),
            expected
        );
        zirql::dispose_native_value(parsed);
    }
    let filter = prefix_filter(e(K::CurrentInput), "a");
    for value in [
        V::Integer(1),
        V::Array(vec![V::Integer(1)]),
        declared(V::Array(vec![]), D::Sequence(Box::new(D::Integer))),
    ] {
        host.value = value;
        assert_eq!(
            zirql::evaluate_query_value(&filter, &types, &mut host, &mut ample())
                .unwrap_err()
                .location,
            Location::Query(0..1)
        );
    }
    host.unknown = true;
    assert_eq!(
        zirql::evaluate_query_value(&filter, &types, &mut host, &mut ample()).unwrap(),
        declared(V::Array(vec![]), D::Sequence(Box::new(D::Integer)))
    );
    host.value = V::Array(vec![V::String("ash".into()), V::Integer(1)]);
    assert_eq!(
        zirql::evaluate_query_value(&filter, &types, &mut host, &mut ample())
            .unwrap_err()
            .location,
        Location::Query(0..1)
    );
    host.value = V::Array((0..101).map(|_| V::String("ash".into())).collect());
    assert_eq!(
        zirql::evaluate_query_value(&e(K::CurrentInput), &types, &mut host, &mut ample())
            .unwrap_err()
            .category,
        zirql::QueryErrorCategory::ItemLimit
    );
    {
        let source = r#"filter(contains("a"))"#;
        let error = zirql::read_value_expression(
            &mut zirql::ArgumentReader::new(source, 0).unwrap(),
            &mut host,
            &mut ample(),
            64,
        )
        .unwrap_err();
        assert!(matches!(error.location, Location::Query(range) if range.start > 0));
    }
    let source = "[] | count | sort";
    let parsed = zirql::read_value_expression(
        &mut zirql::ArgumentReader::new(source, 0).unwrap(),
        &mut host,
        &mut ample(),
        64,
    )
    .unwrap();
    let error = zirql::evaluate_query_value(&parsed, &types, &mut host, &mut ample()).unwrap_err();
    assert_eq!(error.location, Location::Query(13..17));
    zirql::dispose_native_value(parsed);
    for (mut allowance, nesting) in [
        (budget(1, 1000, 64), 64),
        (budget(1000, 1, 64), 64),
        (budget(1000, 1000, 64), 1),
    ] {
        assert!(
            zirql::read_value_expression(
                &mut zirql::ArgumentReader::new(r#"filter(starts_with("a")) | count"#, 0).unwrap(),
                &mut host,
                &mut allowance,
                nesting
            )
            .is_err()
        );
    }
}

fn prefix_filter(array: E, prefix: &str) -> E {
    e(K::Filter {
        array: Box::new(array),
        predicate: zirql::PredicateArgument {
            argument: None,
            body: Box::new(e(K::Binary {
                operator: B::StartsWith,
                left: Box::new(e(K::PredicateSubject)),
                right: Box::new(e(K::Literal(V::String(prefix.into())))),
            })),
        },
    })
}

#[test]
fn prefix_filter_keeps_order_duplicates_empty_contracts_and_shared_value_analysis() {
    let selected = prefix_filter(
        e(K::Array(vec![
            text("ash"),
            text("apple"),
            text("birch"),
            text("apple"),
        ])),
        "a",
    );
    assert_eq!(
        evaluate(&selected).unwrap(),
        V::Array(vec![
            V::String("ash".into()),
            V::String("apple".into()),
            V::String("apple".into())
        ])
    );
    let facts = value_expression_types(&selected, &BTreeMap::new(), &mut ample()).unwrap();
    assert!(
        matches!(&facts[..], [ValueType::Array(ArrayType::Elements(elements))] if elements.iter().all(|t| *t == ValueType::String))
    );
    let explicit = prefix_filter(name("words"), "a");
    let types = BTreeMap::from([(
        "words".into(),
        ValueType::Array(ArrayType::Elements(vec![ValueType::String])),
    )]);
    validate_value_expression(&explicit, &types, &mut ample()).unwrap();
    assert_eq!(
        free_value_names(&explicit, &types, &mut ample()).unwrap()[0].value,
        "words"
    );
    let empty = prefix_filter(
        literal(declared(V::Array(vec![]), D::Sequence(Box::new(D::String)))),
        "a",
    );
    assert_eq!(
        evaluate(&empty).unwrap(),
        declared(V::Array(vec![]), D::Sequence(Box::new(D::String)))
    );
    for source in [e(K::Array(vec![])), e(K::Array(vec![text("birch")]))] {
        assert_eq!(
            evaluate(&reduction(R::Length, prefix_filter(source, "a"))).unwrap(),
            V::Integer(0)
        );
    }
    assert_eq!(
        evaluate(&prefix_filter(
            e(K::Array(vec![text("ash"), text("Ash")])),
            ""
        ))
        .unwrap(),
        V::Array(vec![V::String("ash".into()), V::String("Ash".into())])
    );
    assert_eq!(
        evaluate(&prefix_filter(e(K::Array(vec![text("Ash")])), "a")).unwrap(),
        V::Array(vec![])
    );
    for source in [
        int(1),
        literal(V::KeyedArray {
            axes: vec![KeyAxis {
                key_type: ValueKeyType::Integer,
                keys: vec![],
            }],
            entries: vec![],
        }),
        e(K::Array(vec![int(1)])),
        literal(declared(
            V::Array(vec![]),
            D::Sequence(Box::new(D::Integer)),
        )),
    ] {
        assert_eq!(
            evaluate(&choose(prefix_filter(source, "a")))
                .unwrap_err()
                .location,
            Location::Query(0..1)
        );
    }
    let scalar = e(K::Sort {
        array: Box::new(reduction(R::Length, e(K::Array(vec![])))),
        selector: None,
        descending: false,
    });
    assert!(
        evaluate(&scalar)
            .unwrap_err()
            .message
            .contains("sort needs an array, set or dictionary")
    );
    let missing = evaluate(&e(K::CurrentInput)).unwrap_err();
    assert_eq!(missing.location, Location::Query(0..1));
    assert_eq!(missing.message, "constructed current input is unavailable");
}

#[test]
fn scalar_pipeline_refusals_preserve_preflight_runtime_and_budget_boundaries() {
    for bad in [
        e(K::Array(vec![int(1), text("bad")])),
        e(K::Array(vec![record(vec![])])),
        literal(declared(
            V::Array(vec![]),
            D::Sequence(Box::new(D::Tuple(vec![D::Integer]))),
        )),
    ] {
        let expression = e(K::Unique(Box::new(bad)));
        let error = evaluate(&choose(expression)).unwrap_err();
        assert_eq!(error.location, Location::Query(0..1));
        assert_eq!(error.category, zirql::QueryErrorCategory::Other);
    }
    let contradictory = e(K::Unique(Box::new(e(K::Array(vec![
        member_expression(20, 1),
        member_expression(20, 2),
    ])))));
    assert!(
        evaluate(&contradictory)
            .unwrap_err()
            .message
            .contains("inconsistent nominal member")
    );
    let expression = e(K::Tally(Box::new(literal(declared(
        V::Array(vec![]),
        D::Sequence(Box::new(D::Integer)),
    )))));
    assert_eq!(evaluate(&choose(expression)).unwrap(), V::Integer(7));

    struct UnknownWords;
    impl zirql::ValueQueryHost<()> for UnknownWords {
        type Error = zirql::QueryError;
        fn query_types(
            &mut self,
            _: &(),
            _: &[(&str, Vec<ValueType>)],
            _: &mut Budget,
            _: &std::ops::Range<usize>,
        ) -> Result<Vec<ValueType>, Self::Error> {
            Ok(vec![ValueType::Unestablished])
        }
        fn evaluate_query(
            &mut self,
            _: &(),
            _: &[(&str, V)],
            _: &mut Budget,
        ) -> Result<V, Self::Error> {
            unreachable!()
        }
        fn read_binding(
            &mut self,
            _: &str,
            _: &std::ops::Range<usize>,
            _: &mut Budget,
        ) -> Result<V, Self::Error> {
            Ok(V::Array(vec![V::String("ash".into()), V::Integer(2)]))
        }
    }
    let unknown = BTreeMap::from([("words".into(), vec![ValueType::Unestablished])]);
    let expression: zirql::ValueExpression<()> = e(K::Tally(Box::new(name("words"))));
    assert!(
        matches!(zirql::evaluate_query_value(&expression, &unknown, &mut UnknownWords, &mut ample()).unwrap(), V::Dictionary(ref entries) if entries.len() == 2)
    );
    let expression = e(K::Unique(Box::new(name("words"))));
    let error = zirql::evaluate_query_value(&expression, &unknown, &mut UnknownWords, &mut ample())
        .unwrap_err();
    assert_eq!(error.message, "unique needs homogeneous scalar elements");
    assert_eq!(error.location, Location::Query(0..1));

    for (expression, result_refusal) in [
        (
            prefix_filter(e(K::Array(vec![text("ash"), text("ash")])), "a"),
            false,
        ),
        (
            prefix_filter(e(K::Array(vec![text("ash"), text("ash")])), "a"),
            true,
        ),
        (
            e(K::Unique(Box::new(e(K::Array(vec![
                text("ash"),
                text("ash"),
            ]))))),
            false,
        ),
        (
            e(K::Tally(Box::new(e(K::Array(vec![
                text("ash"),
                text("ash"),
            ]))))),
            true,
        ),
        (
            e(K::Tally(Box::new(e(K::Array(vec![
                e(K::Set(vec![text("ash"), text("apple")])),
                e(K::Set(vec![text("apple"), text("ash")])),
            ]))))),
            false,
        ),
        (
            e(K::Tally(Box::new(e(K::Array(vec![e(K::Set(vec![
                text("ash"),
                text("apple"),
            ]))]))))),
            true,
        ),
    ] {
        let mut allowance = ample();
        let error = zirql::evaluate_value_with_host_validation(
            &expression,
            &Bindings::default(),
            &mut allowance,
            |value, budget, location| {
                if matches!(value, V::Array(_)) {
                    if result_refusal {
                        budget.charge_results(
                            budget.remaining_results().unwrap().saturating_sub(1),
                            location.clone(),
                        )?;
                    } else {
                        budget
                            .charge(budget.remaining_work().saturating_sub(1), location.clone())?;
                    }
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(
            error.category,
            if result_refusal {
                zirql::QueryErrorCategory::ResultLimit
            } else {
                zirql::QueryErrorCategory::WorkLimit
            }
        );
        assert_eq!(error.location, Location::Query(0..1));
        assert!(allowance.remaining_work() < 1_000_000);
    }
    let nested = e(K::Unique(Box::new(e(K::Unique(Box::new(e(K::Array(
        vec![int(1)],
    ))))))));
    let mut allowance = budget(100_000, 100_000, 2);
    assert!(evaluate_value(&nested, &Bindings::default(), &mut allowance).is_err());
    assert_eq!(
        evaluate_value(&int(4), &Bindings::default(), &mut allowance).unwrap(),
        V::Integer(4)
    );
}

fn pattern(kind: zirql::PatternKind) -> zirql::Pattern {
    Located::new(kind, 70..80)
}
fn pattern_name(value: &str) -> zirql::Pattern {
    pattern(zirql::PatternKind::Name(Located::new(value.into(), 71..72)))
}
fn record_pattern(fields: Vec<(&str, zirql::Pattern)>, ignore_rest: bool) -> zirql::Pattern {
    pattern(zirql::PatternKind::Record {
        fields: fields
            .into_iter()
            .map(|(s, p)| (Located::new(s.into(), 73..74), p))
            .collect(),
        ignore_rest,
    })
}
fn block(bindings: Vec<(zirql::Pattern, E)>, value: E) -> E {
    e(K::Block {
        bindings,
        value: Box::new(value),
    })
}
fn arm(pattern: zirql::Pattern, guard: Option<E>, value: E) -> zirql::ValueMatchArm {
    zirql::ValueMatchArm {
        pattern,
        guard,
        value,
    }
}
fn value_match(subject: E, arms: Vec<zirql::ValueMatchArm>) -> E {
    e(K::ValueMatch {
        subject: Box::new(subject),
        arms,
    })
}
fn fallback(value: E) -> zirql::ValueMatchArm {
    arm(pattern(zirql::PatternKind::Wildcard), None, value)
}

#[test]
fn lexical_patterns_select_nested_reports_without_copying_ignored_data() {
    use zirql::PatternKind as P;
    let sources = Bindings::new([(
        Located::new("report".into(), 1..2),
        V::Record(vec![
            ("name".into(), V::String("alpha".into())),
            (
                "metrics".into(),
                V::Tuple(vec![V::Boolean(true), V::Integer(8)]),
            ),
            (
                "ignored".into(),
                V::Array((0..5000).map(V::Integer).collect()),
            ),
        ]),
    )])
    .unwrap();
    let report = block(
        vec![(
            record_pattern(
                vec![
                    ("name", pattern_name("label")),
                    (
                        "metrics",
                        pattern(P::Tuple(vec![pattern(P::Wildcard), pattern_name("cost")])),
                    ),
                ],
                true,
            ),
            name("report"),
        )],
        record(vec![("label", name("label")), ("cost", name("cost"))]),
    );
    let result = evaluate_value(&report, &sources, &mut budget(1_000_000, 100, 64)).unwrap();
    assert_eq!(
        result,
        V::Record(vec![
            ("label".into(), V::String("alpha".into())),
            ("cost".into(), V::Integer(8))
        ])
    );
    assert_eq!(
        free_value_names(
            &report,
            &BTreeMap::from([("report".into(), ValueType::Unestablished)]),
            &mut ample()
        )
        .unwrap()[0]
            .value,
        "report"
    );
    for bad in [
        block(
            vec![(
                record_pattern(vec![("name", pattern_name("label"))], false),
                name("report"),
            )],
            name("label"),
        ),
        block(
            vec![(
                record_pattern(vec![("missing", pattern_name("x"))], true),
                name("report"),
            )],
            name("x"),
        ),
        block(
            vec![(
                pattern(P::Tuple(vec![pattern_name("x")])),
                e(K::Array(vec![int(1)])),
            )],
            name("x"),
        ),
    ] {
        let error = evaluate_value(&bad, &sources, &mut ample()).unwrap_err();
        assert_eq!(error.location, Location::Query(70..80));
        assert!(error.message.contains("pattern did not match"));
    }
    let declared = zirql::declare_constructed(
        V::Record(vec![(
            "row".into(),
            V::Record(vec![("cost".into(), V::Integer(8))]),
        )]),
        D::Record(vec![(
            "row".into(),
            D::Record(vec![("cost".into(), D::Integer)]),
        )]),
        &mut ample(),
        Location::Query(4..5),
    )
    .unwrap();
    let retained = block(
        vec![(
            record_pattern(vec![("row", pattern_name("row"))], false),
            literal(declared),
        )],
        field(name("row"), "cost"),
    );
    let retained = evaluate_value(&retained, &Bindings::default(), &mut ample()).unwrap();
    assert_eq!(retained.data(), &V::Integer(8));
    assert!(matches!(
        constructed_value_type(&retained, &mut ample(), Location::Query(4..5)).unwrap(),
        ValueType::Declared {
            declaration: D::Integer,
            ..
        }
    ));
    let escaped = e(K::Tuple(vec![report, name("label")]));
    assert!(
        evaluate_value(&escaped, &sources, &mut ample())
            .unwrap_err()
            .message
            .contains("undefined binding")
    );
}

#[test]
fn guarded_presence_match_uses_written_order_and_only_the_selected_value() {
    use zirql::PatternKind as P;
    let classify = |subject| {
        value_match(
            subject,
            vec![
                arm(pattern(P::Absent), None, int(0)),
                arm(
                    pattern(P::Present(Box::new(pattern_name("cost")))),
                    Some(binary(B::Less, name("cost"), int(0))),
                    binary(B::IntegerDivide, int(1), int(0)),
                ),
                arm(
                    pattern(P::Present(Box::new(pattern_name("cost")))),
                    Some(binary(B::GreaterEqual, name("cost"), int(0))),
                    block(
                        vec![(pattern_name("answer"), binary(B::Add, name("cost"), int(1)))],
                        name("answer"),
                    ),
                ),
                fallback(binary(B::IntegerDivide, int(1), int(0))),
            ],
        )
    };
    assert_eq!(
        evaluate_value(&classify(int(8)), &Bindings::default(), &mut ample()).unwrap(),
        V::Integer(9)
    );
    assert_eq!(
        evaluate_value(
            &classify(literal(V::Absent)),
            &Bindings::default(),
            &mut ample()
        )
        .unwrap(),
        V::Integer(0)
    );
    let absent = zirql::declare_constructed(
        V::Absent,
        D::Optional(Box::new(D::Integer)),
        &mut ample(),
        Location::Query(4..5),
    )
    .unwrap();
    assert_eq!(
        evaluate_value(
            &classify(literal(absent)),
            &Bindings::default(),
            &mut ample()
        )
        .unwrap(),
        V::Integer(0)
    );
    // A declared present value keeps its underlying child contract.
    let functions = [zirql::PatternFunctionDeclaration {
        name: Located::new("classify".into(), 1..9),
        parameters: vec![Located::new(
            zirql::PatternParameter {
                pattern: pattern_name("input"),
                declaration: D::Optional(Box::new(D::Integer)),
            },
            10..15,
        )],
        result: Located::new(D::Integer, 16..20),
        value: classify(name("input")),
    }];
    let value = zirql::evaluate_pattern_functions(
        &functions,
        &call("classify", vec![int(8)]),
        &Bindings::default(),
        &mut ample(),
        zirql::FunctionLimits { active_depth: 1 },
    )
    .unwrap();
    assert_eq!(value.data(), &V::Integer(9));
    let bad_guard = value_match(
        int(8),
        vec![
            arm(pattern_name("x"), Some(int(1)), name("x")),
            fallback(int(0)),
        ],
    );
    assert!(
        evaluate_value(&bad_guard, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("Boolean")
    );
    let failure = value_match(
        int(8),
        vec![
            arm(
                pattern_name("x"),
                Some(binary(
                    B::Equal,
                    binary(B::IntegerDivide, int(1), int(0)),
                    int(0),
                )),
                name("x"),
            ),
            fallback(int(0)),
        ],
    );
    assert!(
        evaluate_value(&failure, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("zero")
    );
}

#[test]
fn parameter_patterns_admit_all_whole_arguments_before_binding_and_tail_reuse() {
    use zirql::PatternKind as P;
    let functions = [zirql::PatternFunctionDeclaration {
        name: Located::new("accumulate".into(), 1..9),
        parameters: vec![Located::new(
            zirql::PatternParameter {
                pattern: pattern(P::Tuple(vec![pattern_name("n"), pattern_name("total")])),
                declaration: D::Tuple(vec![D::Integer, D::Integer]),
            },
            10..15,
        )],
        result: Located::new(D::Integer, 16..20),
        value: value_match(
            name("n"),
            vec![
                arm(pattern(P::Literal(V::Integer(0))), None, name("total")),
                fallback(select_branch(
                    literal(V::Boolean(true)),
                    block(
                        vec![(pattern_name("old"), name("n"))],
                        call(
                            "accumulate",
                            vec![e(K::Tuple(vec![
                                binary(B::Subtract, name("n"), int(1)),
                                binary(B::Add, name("total"), name("old")),
                            ]))],
                        ),
                    ),
                    int(-1),
                )),
            ],
        ),
    }];
    let result = zirql::evaluate_pattern_functions(
        &functions,
        &call("accumulate", vec![e(K::Tuple(vec![int(10_000), int(0)]))]),
        &Bindings::default(),
        &mut budget(30_000_000, 1_000_000, 64),
        zirql::FunctionLimits { active_depth: 1 },
    )
    .unwrap();
    assert_eq!(result.data(), &V::Integer(50_005_000));
    let refused = zirql::PatternFunctionDeclaration {
        name: Located::new("refused".into(), 1..9),
        parameters: vec![
            Located::new(
                zirql::PatternParameter {
                    pattern: pattern(P::Literal(V::Integer(99))),
                    declaration: D::Integer,
                },
                10..15,
            ),
            Located::new(
                zirql::PatternParameter {
                    pattern: pattern_name("second"),
                    declaration: D::Boolean,
                },
                20..25,
            ),
        ],
        result: Located::new(D::Integer, 30..35),
        value: int(0),
    };
    let declarations = [refused];
    let mut allowance = ample();
    let error = zirql::evaluate_pattern_functions(
        &declarations,
        &call(
            "refused",
            vec![int(1), Located::new(K::Literal(V::Integer(2)), 90..95)],
        ),
        &Bindings::default(),
        &mut allowance,
        zirql::FunctionLimits { active_depth: 1 },
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(90..95));
    let error = zirql::evaluate_pattern_functions(
        &declarations,
        &call("refused", vec![int(1), literal(V::Boolean(true))]),
        &Bindings::default(),
        &mut allowance,
        zirql::FunctionLimits { active_depth: 1 },
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(70..80));
    assert!(error.error.message.contains("parameter pattern"));
    assert_eq!(
        evaluate_value(&int(3), &Bindings::default(), &mut allowance).unwrap(),
        V::Integer(3)
    );
}

#[test]
fn unused_patterns_cannot_hide_scope_or_fallback_errors() {
    use zirql::PatternKind as P;
    for arms in [
        vec![arm(pattern_name("x"), None, int(0))],
        vec![arm(
            pattern(P::Wildcard),
            Some(literal(V::Boolean(true))),
            int(0),
        )],
        vec![fallback(int(1)), fallback(int(2))],
        vec![
            arm(
                pattern(P::Tuple(vec![pattern_name("x"), pattern_name("x")])),
                None,
                int(0),
            ),
            fallback(int(1)),
        ],
        vec![
            arm(
                record_pattern(
                    vec![("a", pattern_name("x")), ("a", pattern_name("y"))],
                    true,
                ),
                None,
                int(0),
            ),
            fallback(int(1)),
        ],
    ] {
        assert!(
            evaluate_value(
                &value_match(int(0), arms),
                &Bindings::default(),
                &mut ample()
            )
            .is_err()
        );
    }
    let bindings = Bindings::new([(Located::new("outer".into(), 1..2), V::Integer(1))]).unwrap();
    let unused = value_match(
        int(0),
        vec![
            arm(
                pattern_name("outer"),
                Some(literal(V::Boolean(false))),
                int(1),
            ),
            fallback(int(2)),
        ],
    );
    assert!(
        evaluate_value(&unused, &bindings, &mut ample())
            .unwrap_err()
            .message
            .contains("shadows")
    );
    let shadow = block(
        vec![(pattern_name("x"), int(1))],
        value_match(
            int(2),
            vec![arm(pattern_name("x"), None, name("x")), fallback(int(0))],
        ),
    );
    assert!(
        evaluate_value(&shadow, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("already defined")
    );
}

#[test]
fn pattern_and_guard_work_share_host_validation_allowance() {
    let binding = block(vec![(pattern_name("x"), int(23))], name("x"));
    let error = zirql::evaluate_value_with_host_validation(
        &binding,
        &Bindings::default(),
        &mut ample(),
        |value, budget, location| {
            if value == &V::Integer(23) {
                budget.charge(budget.remaining_work(), location.clone())?;
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.category, zirql::QueryErrorCategory::WorkLimit);
    assert_eq!(error.location, Location::Query(70..80));
    let guarded = value_match(
        int(23),
        vec![
            arm(
                pattern_name("x"),
                Some(Located::new(K::Literal(V::Boolean(true)), 85..86)),
                name("x"),
            ),
            fallback(int(0)),
        ],
    );
    let error = zirql::evaluate_value_with_host_validation(
        &guarded,
        &Bindings::default(),
        &mut ample(),
        |value, budget, location| {
            if value == &V::Boolean(true) {
                budget.charge(budget.remaining_work(), location.clone())?;
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.category, zirql::QueryErrorCategory::WorkLimit);
    assert_eq!(error.location, Location::Query(85..86));
}

fn accumulate(items: E, initial: E, step: &str, scan: bool) -> E {
    let step = Located::new(step.into(), 70..74);
    e(if scan {
        K::Scan {
            items: Box::new(items),
            initial: Box::new(initial),
            step,
        }
    } else {
        K::Fold {
            items: Box::new(items),
            initial: Box::new(initial),
            step,
        }
    })
}

#[test]
fn named_accumulators_preserve_order_empty_contracts_and_bounded_calls() {
    let functions = [function(
        "decimal",
        vec![("s", D::Integer), ("x", D::Integer)],
        D::Integer,
        binary(B::Add, binary(B::Multiply, name("s"), int(10)), name("x")),
    )];
    for scan in [false, true] {
        let result = run_functions(
            &functions,
            &accumulate(
                literal(V::Array(vec![V::Integer(1), V::Integer(2), V::Integer(3)])),
                int(0),
                "decimal",
                scan,
            ),
            1,
            &mut ample(),
        )
        .unwrap();
        if scan {
            let V::Array(values) = result.data() else {
                panic!("scan")
            };
            assert_eq!(
                values.iter().map(V::data).collect::<Vec<_>>(),
                vec![&V::Integer(1), &V::Integer(12), &V::Integer(123)]
            );
        } else {
            assert_eq!(result.data(), &V::Integer(123));
        }
        let empty = run_functions(
            &functions,
            &accumulate(literal(V::Array(vec![])), int(9), "decimal", scan),
            1,
            &mut ample(),
        )
        .unwrap();
        if scan {
            assert_eq!(empty.data(), &V::Array(vec![]));
            admit_constructed(
                &empty,
                &D::<()>::Sequence(Box::new(D::Integer)),
                &mut ample(),
                Location::Query(0..1),
            )
            .unwrap();
        } else {
            assert_eq!(empty.data(), &V::Integer(9));
        }
    }
    let add = [function(
        "add",
        vec![("s", D::Integer), ("x", D::Integer)],
        D::Integer,
        binary(B::Add, name("s"), name("x")),
    )];
    let mut large = Budget::for_program(ProgramLimits {
        work: 20_000_000,
        results: 1_000_000,
        nesting: 64,
        stream_items: 20_000,
    });
    let expression = accumulate(
        literal(V::Array((0..10_000).map(|_| V::Integer(1)).collect())),
        int(0),
        "add",
        false,
    );
    // The input length does not become active function depth.
    assert_eq!(
        run_functions(&add, &expression, 1, &mut large)
            .unwrap()
            .data(),
        &V::Integer(10_000)
    );
    assert!(evaluate_value(&expression, &Bindings::default(), &mut ample()).is_err());
}

#[test]
fn named_scan_retains_structured_snapshots_and_aliased_payloads() {
    let state = D::Record(vec![
        ("total".into(), D::Integer),
        ("payload".into(), D::Sequence(Box::new(D::Integer))),
    ]);
    let step = function(
        "advance",
        vec![("s", state.clone()), ("x", D::Integer)],
        state,
        record(vec![
            (
                "total",
                binary(B::Add, field(name("s"), "total"), name("x")),
            ),
            ("payload", field(name("s"), "payload")),
        ]),
    );
    let initial = || {
        V::Record(vec![
            ("total".into(), V::Integer(0)),
            (
                "payload".into(),
                V::Array((0..32).map(|_| V::Integer(7)).collect()),
            ),
        ])
    };
    let bindings = Bindings::new([
        (Located::new("initial".into(), 0..1), initial()),
        (
            Located::new("items".into(), 0..1),
            V::Array(vec![V::Integer(2), V::Integer(3)]),
        ),
    ])
    .unwrap();
    let result = zirql::evaluate_functions(
        &[step],
        &accumulate(name("items"), name("initial"), "advance", true),
        &bindings,
        &mut ample(),
        zirql::FunctionLimits { active_depth: 1 },
    )
    .unwrap();
    let V::Array(states) = result.data() else {
        panic!("scan states")
    };
    for (state, expected) in states.iter().zip([2, 5]) {
        let V::Record(fields) = state.data() else {
            panic!("state record")
        };
        assert_eq!(fields[0].1.data(), &V::Integer(expected));
        assert_eq!(
            fields[1].1.data(),
            &V::Array((0..32).map(|_| V::Integer(7)).collect())
        );
    }
    assert_eq!(bindings.get("initial", 0..1).unwrap(), &initial());
    assert_eq!(
        bindings.get("items", 0..1).unwrap(),
        &V::Array(vec![V::Integer(2), V::Integer(3)])
    );
}

#[test]
fn named_accumulators_preflight_steps_and_check_every_actual_state() {
    let empty = || literal(V::Array(vec![]));
    let expression = |step: &str| accumulate(empty(), int(0), step, false);
    let wrong = function(
        "wrong",
        vec![("s", D::Integer), ("x", D::Integer)],
        D::String,
        literal(V::String("bad".into())),
    );
    let arity = function("arity", vec![("s", D::Integer)], D::Integer, name("s"));
    for (functions, step) in [
        (vec![], "unknown"),
        (vec![wrong], "wrong"),
        (vec![arity], "arity"),
    ] {
        let error = run_functions(&functions, &expression(step), 1, &mut ample()).unwrap_err();
        assert_eq!(error.error.location, Location::Query(70..74));
    }
    let optional_result = function(
        "maybe",
        vec![("s", D::Integer), ("x", D::Integer)],
        D::Optional(Box::new(D::Integer)),
        call("missing", vec![]),
    );
    let missing = function(
        "missing",
        vec![],
        D::Optional(Box::new(D::Integer)),
        literal(V::Absent),
    );
    let mut spending = ample();
    let error = run_functions(
        &[optional_result, missing],
        &accumulate(
            literal(V::Array(vec![V::Integer(1)])),
            int(0),
            "maybe",
            true,
        ),
        1,
        &mut spending,
    )
    .unwrap_err();
    assert_eq!(error.calls[0].function, "missing");
    assert_eq!(error.error.location, Location::Query(0..1));
    assert_eq!(
        run_functions(&[], &int(8), 1, &mut spending).unwrap(),
        V::Integer(8)
    );
    let optional_state = function(
        "maybe",
        vec![("s", D::Optional(Box::new(D::Integer))), ("x", D::Integer)],
        D::Optional(Box::new(D::Integer)),
        literal(V::Absent),
    );
    let result = run_functions(
        &[optional_state],
        &accumulate(
            literal(V::Array(vec![V::Integer(1)])),
            int(0),
            "maybe",
            true,
        ),
        1,
        &mut ample(),
    )
    .unwrap();
    let V::Array(states) = result.data() else {
        panic!("optional states")
    };
    assert_eq!(states[0].data(), &V::Absent);
    let add = [function(
        "add",
        vec![("s", D::Integer), ("x", D::Integer)],
        D::Integer,
        binary(B::Add, name("s"), name("x")),
    )];
    assert!(
        run_functions(
            &add,
            &accumulate(empty(), literal(V::Absent), "add", false),
            1,
            &mut ample()
        )
        .is_err()
    );
    assert!(
        run_functions(
            &add,
            &accumulate(
                literal(V::Array(vec![V::String("bad".into())])),
                int(0),
                "add",
                false
            ),
            1,
            &mut ample()
        )
        .is_err()
    );
    let keyed = e(K::KeyedArray {
        declared_axis_sources: None,
        axes: Some(vec![KeyAxis {
            key_type: ValueKeyType::Integer,
            keys: vec![],
        }]),
        entries: vec![],
    });
    assert!(
        run_functions(
            &add,
            &accumulate(keyed, int(0), "add", false),
            1,
            &mut ample()
        )
        .is_err()
    );
    let failure = |range| {
        Located::new(
            K::Binary {
                operator: B::Divide,
                left: Box::new(int(1)),
                right: Box::new(int(0)),
            },
            range,
        )
    };
    let error = run_functions(
        &add,
        &accumulate(failure(81..83), failure(91..93), "add", false),
        1,
        &mut ample(),
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(81..83));
    let division = [function(
        "divide",
        vec![("s", D::Integer), ("x", D::Integer)],
        D::Integer,
        failure(101..105),
    )];
    let error = run_functions(
        &division,
        &accumulate(
            literal(V::Array(vec![V::Integer(1)])),
            int(0),
            "divide",
            false,
        ),
        1,
        &mut ample(),
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(101..105));
    assert_eq!(error.calls[0].function, "divide");
    let items = literal(V::Array((0..20).map(|_| V::Integer(1)).collect()));
    let request = accumulate(items, int(0), "add", true);
    let error = run_functions(&add, &request, 1, &mut budget(1000, 100_000, 64)).unwrap_err();
    assert_eq!(error.error.category, zirql::QueryErrorCategory::WorkLimit);
    let error = run_functions(&add, &request, 1, &mut budget(100_000, 50, 64)).unwrap_err();
    assert_eq!(error.error.category, zirql::QueryErrorCategory::ResultLimit);
}

#[test]
fn named_accumulators_keep_declared_absence_and_empty_element_types() {
    let optional_int = || D::Optional(Box::new(D::Integer));
    let optional_string = || D::Optional(Box::new(D::String));
    let foreign = || function("foreign", vec![], optional_string(), literal(V::Absent));
    let keep = || {
        function(
            "keep",
            vec![("s", optional_int()), ("x", D::Integer)],
            optional_int(),
            name("s"),
        )
    };
    let empty = || literal(V::Array(vec![]));
    let mut spending = ample();
    let error = run_functions(
        &[foreign(), keep()],
        &accumulate(empty(), call("foreign", vec![]), "keep", false),
        2,
        &mut spending,
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(30..40));
    assert_eq!(
        error.error.message,
        "incompatible declared underlying types"
    );
    // A direct declaration and a function's retained declaration have the same meaning.
    let initial = Located::new(
        K::Literal(V::Declared {
            value: Box::new(V::Absent),
            declaration: optional_string(),
        }),
        90..94,
    );
    let error = run_functions(
        &[keep()],
        &accumulate(empty(), initial, "keep", false),
        1,
        &mut spending,
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(90..94));
    let wrong_return = function(
        "keep",
        vec![("s", optional_int()), ("x", D::Integer)],
        optional_int(),
        call("foreign", vec![]),
    );
    let error = run_functions(
        &[foreign(), wrong_return],
        &accumulate(literal(V::Array(vec![V::Integer(1)])), int(0), "keep", true),
        2,
        &mut spending,
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(30..40));
    assert_eq!(error.calls[0].function, "keep");

    // A field view inherits its empty sequence declaration from the shared parent.
    let outer = V::Declared {
        value: Box::new(V::Record(vec![("items".into(), V::Array(vec![]))])),
        declaration: D::Record(vec![("items".into(), D::Sequence(Box::new(D::String)))]),
    };
    let bindings = Bindings::new([(Located::new("outer".into(), 0..1), outer)]).unwrap();
    let source = Located::new(field(name("outer"), "items").value, 80..84);
    let error = zirql::evaluate_functions(
        &[keep()],
        &accumulate(source, int(0), "keep", false),
        &bindings,
        &mut spending,
        zirql::FunctionLimits { active_depth: 1 },
    )
    .unwrap_err();
    assert_eq!(error.error.location, Location::Query(80..84));
    assert_eq!(
        error.error.message,
        "incompatible declared underlying types"
    );
    let V::Record(fields) = bindings.get("outer", 0..1).unwrap().data() else {
        panic!("retained parent")
    };
    assert_eq!(fields[0].1.data(), &V::Array(vec![]));

    // Unannotated absence and empty collections still acquire their legal context.
    assert_eq!(
        run_functions(
            &[keep()],
            &accumulate(empty(), literal(V::Absent), "keep", false),
            1,
            &mut spending
        )
        .unwrap()
        .data(),
        &V::Absent
    );
    let sequence = D::Sequence(Box::new(D::Integer));
    let keep_sequence = function(
        "keep",
        vec![("s", sequence.clone()), ("x", D::Integer)],
        sequence,
        name("s"),
    );
    assert_eq!(
        run_functions(
            &[keep_sequence],
            &accumulate(empty(), empty(), "keep", false),
            1,
            &mut spending
        )
        .unwrap()
        .data(),
        &V::Array(vec![])
    );
}

fn state_loop(states: Vec<(&str, E)>, body: E) -> E {
    e(K::Loop {
        states: states
            .into_iter()
            .map(|(s, v)| (Located::new(s.into(), 80..81), v))
            .collect(),
        body: Box::new(body),
    })
}
fn exit(value: E) -> E {
    e(K::Break(Box::new(value)))
}
fn advance(values: Vec<E>) -> E {
    e(K::Continue(values))
}

#[test]
fn loops_accumulate_beyond_call_depth_and_replace_complete_old_state() {
    let accumulator = function(
        "acc",
        vec![("input", D::Integer)],
        D::Integer,
        state_loop(
            vec![("remaining", name("input")), ("total", int(0))],
            block(
                vec![(pattern_name("old"), name("remaining"))],
                select_branch(
                    binary(B::Equal, name("remaining"), int(0)),
                    exit(name("total")),
                    advance(vec![
                        binary(B::Subtract, name("remaining"), int(1)),
                        binary(B::Add, name("total"), name("old")),
                    ]),
                ),
            ),
        ),
    );
    assert_eq!(
        run_functions(
            &[accumulator],
            &call("acc", vec![int(10_000)]),
            1,
            &mut budget(20_000_000, 2_000_000, 64)
        )
        .unwrap()
        .data(),
        &V::Integer(50_005_000)
    );
    let swap = state_loop(
        vec![("n", int(3)), ("x", int(2)), ("y", int(9))],
        select_branch(
            binary(B::Equal, name("n"), int(0)),
            exit(e(K::Tuple(vec![name("x"), name("y")]))),
            advance(vec![
                binary(B::Subtract, name("n"), int(1)),
                name("y"),
                name("x"),
            ]),
        ),
    );
    let result = evaluate_value(&swap, &Bindings::default(), &mut ample())
        .unwrap()
        .into_data();
    assert!(
        matches!(result, V::Tuple(v) if v[0].data()==&V::Integer(9) && v[1].data()==&V::Integer(2))
    );
}

#[test]
fn loop_scopes_and_nested_match_transfers_stay_lexical() {
    let inner = state_loop(
        vec![("inner", int(1))],
        select_branch(
            binary(B::Equal, name("inner"), int(0)),
            exit(int(2)),
            advance(vec![int(0)]),
        ),
    );
    let expression = block(
        vec![(pattern_name("seed"), int(3))],
        state_loop(
            vec![("n", name("seed")), ("total", int(0))],
            block(
                vec![(pattern_name("step"), inner)],
                value_match(
                    name("n"),
                    vec![
                        arm(
                            pattern(zirql::PatternKind::Literal(V::Integer(0))),
                            None,
                            exit(name("total")),
                        ),
                        fallback(advance(vec![
                            binary(B::Subtract, name("n"), int(1)),
                            binary(B::Add, name("total"), name("step")),
                        ])),
                    ],
                ),
            ),
        ),
    );
    assert_eq!(
        evaluate_value(&expression, &Bindings::default(), &mut ample())
            .unwrap()
            .data(),
        &V::Integer(6)
    );
    assert!(
        free_value_names(&expression, &BTreeMap::new(), &mut ample())
            .unwrap()
            .is_empty()
    );
    let leaked = block(
        vec![(
            pattern_name("result"),
            state_loop(vec![("private", int(1))], exit(name("private"))),
        )],
        name("private"),
    );
    assert!(
        evaluate_value(&leaked, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("undefined binding")
    );
    let forward = state_loop(vec![("a", int(1)), ("b", name("a"))], exit(name("b")));
    assert!(
        evaluate_value(&forward, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("undefined binding")
    );
    let duplicate = state_loop(vec![("a", int(1)), ("a", int(2))], exit(name("a")));
    assert!(
        validate_value_expression(&duplicate, &BTreeMap::new(), &mut ample())
            .unwrap_err()
            .message
            .contains("already defined")
    );
    let shadow = block(
        vec![(pattern_name("a"), int(1))],
        state_loop(vec![("a", int(2))], exit(name("a"))),
    );
    assert!(
        evaluate_value(&shadow, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("already defined")
    );
}

#[test]
fn loop_preparation_rejects_fallthrough_ordinary_transfers_and_function_escape() {
    let fallthrough = state_loop(
        vec![],
        select_branch(literal(V::Boolean(true)), exit(int(1)), int(2)),
    );
    assert!(
        evaluate_value(&fallthrough, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("must end")
    );
    let operand_transfer = state_loop(vec![], exit(binary(B::Add, int(1), exit(int(2)))));
    assert!(
        evaluate_value(&operand_transfer, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("terminal loop body")
    );
    let wrong_arity = state_loop(vec![("a", int(0))], advance(vec![]));
    assert!(
        evaluate_value(&wrong_arity, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("argument count")
    );
    let helper = function("escape", vec![], D::Integer, exit(int(1)));
    assert!(
        run_functions(
            &[helper],
            &state_loop(vec![], exit(call("escape", vec![]))),
            1,
            &mut ample()
        )
        .unwrap_err()
        .error
        .message
        .contains("terminal loop body")
    );
    let outside = advance(vec![]);
    assert!(validate_value_expression(&outside, &BTreeMap::new(), &mut ample()).is_err());
    let discard = state_loop(vec![("_", int(0))], exit(int(1)));
    assert!(
        evaluate_value(&discard, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("discard")
    );
}

#[test]
fn loop_contracts_keep_optional_empty_and_structured_state_fixed() {
    let optional = literal(V::Declared {
        value: Box::new(V::Integer(1)),
        declaration: D::Optional(Box::new(D::Integer)),
    });
    let expression = state_loop(
        vec![("value", optional)],
        select_branch(
            e(K::Occurs(Box::new(name("value")))),
            advance(vec![literal(V::Absent)]),
            exit(e(K::Default {
                value: Box::new(name("value")),
                fallback: Box::new(int(7)),
            })),
        ),
    );
    assert_eq!(
        evaluate_value(&expression, &Bindings::default(), &mut ample())
            .unwrap()
            .data(),
        &V::Integer(7)
    );
    let shrink = state_loop(
        vec![("values", e(K::Array(vec![int(1)])))],
        select_branch(
            binary(
                B::Equal,
                e(K::Reduce {
                    operator: R::Length,
                    array: Box::new(name("values")),
                }),
                int(0),
            ),
            exit(name("values")),
            advance(vec![e(K::Array(vec![]))]),
        ),
    );
    let result = evaluate_value(&shrink, &Bindings::default(), &mut ample()).unwrap();
    assert!(matches!(result.data(), V::Array(v) if v.is_empty()));
    assert!(matches!(
        result,
        V::Declared {
            declaration: D::Sequence(_),
            ..
        }
    ));
    let typed_empty = state_loop(
        vec![(
            "values",
            literal(V::Declared {
                value: Box::new(V::Array(vec![])),
                declaration: D::Sequence(Box::new(D::Integer)),
            }),
        )],
        exit(name("values")),
    );
    assert!(
        matches!(evaluate_value(&typed_empty, &Bindings::default(), &mut ample()).unwrap().data(), V::Array(v) if v.is_empty())
    );
    let wrong_scalar = state_loop(
        vec![("value", int(1))],
        advance(vec![literal(V::Boolean(false))]),
    );
    assert!(
        evaluate_value(&wrong_scalar, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("integer")
    );
    let wrong_optional = state_loop(
        vec![(
            "value",
            literal(V::Declared {
                value: Box::new(V::Absent),
                declaration: D::Optional(Box::new(D::Integer)),
            }),
        )],
        advance(vec![literal(V::String("wrong".into()))]),
    );
    assert!(
        evaluate_value(&wrong_optional, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("integer")
    );
    let wrong = state_loop(
        vec![("pair", e(K::Tuple(vec![int(1), literal(V::Boolean(true))])))],
        advance(vec![e(K::Tuple(vec![int(2), int(3)]))]),
    );
    assert!(
        evaluate_value(&wrong, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("Boolean")
    );
    let bare = state_loop(vec![("value", literal(V::Absent))], exit(int(0)));
    assert!(
        evaluate_value(&bare, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("established type")
    );
    let empty = state_loop(vec![("value", e(K::Array(vec![])))], exit(int(0)));
    assert!(
        evaluate_value(&empty, &Bindings::default(), &mut ample())
            .unwrap_err()
            .message
            .contains("explicit element contract")
    );
    let absent_string = function(
        "missing_text",
        vec![],
        D::Optional(Box::new(D::String)),
        literal(V::Absent),
    );
    let typed_absence = state_loop(
        vec![(
            "value",
            literal(V::Declared {
                value: Box::new(V::Absent),
                declaration: D::Optional(Box::new(D::Integer)),
            }),
        )],
        advance(vec![call("missing_text", vec![])]),
    );
    assert!(
        run_functions(&[absent_string], &typed_absence, 1, &mut ample())
            .unwrap_err()
            .error
            .message
            .contains("incompatible declared")
    );
    let absent_int = function(
        "missing_int",
        vec![],
        D::Optional(Box::new(D::Integer)),
        literal(V::Absent),
    );
    let borrowed = function(
        "keep_int",
        vec![("input", D::Optional(Box::new(D::Integer)))],
        D::Integer,
        state_loop(
            vec![("value", name("input"))],
            advance(vec![call("missing_text", vec![])]),
        ),
    );
    let absent_string = function(
        "missing_text",
        vec![],
        D::Optional(Box::new(D::String)),
        literal(V::Absent),
    );
    assert!(
        run_functions(
            &[absent_int, borrowed, absent_string],
            &call("keep_int", vec![call("missing_int", vec![])]),
            2,
            &mut ample()
        )
        .unwrap_err()
        .error
        .message
        .contains("incompatible declared")
    );
    let wrong_exit = function(
        "wrong",
        vec![],
        D::Integer,
        state_loop(vec![], exit(literal(V::Boolean(true)))),
    );
    assert!(
        run_functions(&[wrong_exit], &call("wrong", vec![]), 1, &mut ample())
            .unwrap_err()
            .error
            .message
            .contains("integer")
    );
}

#[test]
fn loop_failures_release_scope_and_keep_spent_budget_and_depth_limits() {
    let mut allowance = ample();
    let failing = state_loop(
        vec![("x", int(1)), ("y", int(2))],
        advance(vec![int(3), binary(B::IntegerDivide, int(1), int(0))]),
    );
    let remaining = allowance.remaining_work();
    assert!(
        evaluate_value(&failing, &Bindings::default(), &mut allowance)
            .unwrap_err()
            .message
            .contains("zero")
    );
    assert!(allowance.remaining_work() < remaining);
    assert_eq!(
        evaluate_value(&int(8), &Bindings::default(), &mut allowance).unwrap(),
        V::Integer(8)
    );
    let infinite = state_loop(vec![], advance(vec![]));
    assert_eq!(
        evaluate_value(&infinite, &Bindings::default(), &mut budget(2000, 1000, 16))
            .unwrap_err()
            .category,
        zirql::QueryErrorCategory::WorkLimit
    );
    let nested = function(
        "nested",
        vec![],
        D::Integer,
        state_loop(vec![], exit(binary(B::Add, int(1), call("nested", vec![])))),
    );
    assert!(
        run_functions(&[nested], &call("nested", vec![]), 1, &mut ample())
            .unwrap_err()
            .error
            .message
            .contains("active function depth")
    );
    let tail = function(
        "tail",
        vec![("n", D::Integer)],
        D::Integer,
        state_loop(
            vec![],
            exit(select_branch(
                binary(B::Equal, name("n"), int(0)),
                int(0),
                call("tail", vec![binary(B::Subtract, name("n"), int(1))]),
            )),
        ),
    );
    assert_eq!(
        run_functions(
            &[tail],
            &call("tail", vec![int(1000)]),
            1,
            &mut budget(5_000_000, 500_000, 64)
        )
        .unwrap()
        .data(),
        &V::Integer(0)
    );
}

#[test]
fn loops_inside_named_steps_keep_the_fixed_state_return_contract() {
    let optional = D::Optional(Box::new(D::Integer));
    let step = function(
        "loop_step",
        vec![("s", D::Integer), ("x", D::Integer)],
        optional.clone(),
        state_loop(vec![], exit(call("selected", vec![]))),
    );
    let selected = function("selected", vec![], optional, literal(V::Absent));
    let request = accumulate(e(K::Array(vec![int(1)])), int(0), "loop_step", false);
    let error = run_functions(&[step, selected], &request, 1, &mut ample()).unwrap_err();
    assert_eq!(error.calls[0].function, "selected");
    assert!(error.error.message.contains("integer"));
}

#[test]
fn rejected_literal_payloads_have_stack_safe_cleanup() {
    std::thread::spawn(|| {
        let deep = || {
            let mut value = V::Integer(1);
            let mut declaration = D::Integer;
            for _ in 0..20_000 {
                value = V::Tuple(vec![value]);
                declaration = D::Optional(Box::new(declaration));
            }
            V::Declared {
                value: Box::new(value),
                declaration,
            }
        };
        let expression: E<zirql::Expression<(), (), ()>> = Located::new(K::Literal(deep()), 0..1);
        zirql::dispose_native_value(expression);
        zirql::dispose_pattern(Located::new(zirql::PatternKind::Literal(deep()), 0..1));
    })
    .join()
    .unwrap();
}

#[test]
fn declared_domains_keep_sparse_order_unused_keys_and_typed_empty_identity() {
    let guests = V::Array(vec![
        V::Member(member(22, 4)),
        V::Member(member(11, 1)),
        V::Member(member(33, 7)),
    ]);
    let guest_axis = || KeyAxis {
        key_type: ValueKeyType::Member(NominalTypeId(7)),
        keys: vec![
            ValueKey::Member(member(22, 4)),
            ValueKey::Member(member(11, 1)),
            ValueKey::Member(member(33, 7)),
        ],
    };
    let sources = || vec![name("guests"), e(K::Array(vec![int(8), int(1), int(4)]))];
    let indexed = e(K::KeyedArray {
        axes: None,
        declared_axis_sources: Some(sources()),
        entries: vec![
            (e(K::Tuple(vec![member_expression(11, 1), int(1)])), int(1)),
            (e(K::Tuple(vec![member_expression(22, 4), int(8)])), int(8)),
            (e(K::Tuple(vec![member_expression(11, 1), int(8)])), int(8)),
        ],
    });
    let comprehension = e(K::KeyedComprehension {
        clauses: vec![
            clause(G::In {
                case: false,
                binders: vec![binder("g")],
                source: name("guests"),
                filter: Some(binary(B::NotEqual, name("g"), member_expression(33, 7))),
            }),
            clause(G::In {
                case: false,
                binders: vec![binder("j")],
                source: e(K::Array(vec![int(1), int(8)])),
                filter: Some(binary(
                    B::Or,
                    binary(B::Equal, name("j"), int(8)),
                    binary(B::Equal, name("g"), member_expression(11, 1)),
                )),
            }),
        ],
        key: Box::new(e(K::Tuple(vec![name("g"), name("j")]))),
        head: Box::new(name("j")),
        declared_axis_sources: Some(sources()),
        empty_axis_sources: None,
    });
    let expected = V::KeyedArray {
        axes: vec![guest_axis(), integer_axis(&[8, 1, 4])],
        entries: vec![
            KeyedEntry {
                key: vec![ValueKey::Member(member(22, 4)), ValueKey::Integer(8)],
                value: V::Integer(8),
            },
            KeyedEntry {
                key: vec![ValueKey::Member(member(11, 1)), ValueKey::Integer(8)],
                value: V::Integer(8),
            },
            KeyedEntry {
                key: vec![ValueKey::Member(member(11, 1)), ValueKey::Integer(1)],
                value: V::Integer(1),
            },
        ],
    };
    let types = BTreeMap::from([(
        "guests".into(),
        constructed_value_type(&guests, &mut ample(), Location::Source(0..1)).unwrap(),
    )]);
    let bindings = Bindings::new([(Located::new("guests".into(), 0..1), guests)]).unwrap();
    for expression in [&indexed, &comprehension] {
        assert_eq!(
            free_value_names(expression, &types, &mut ample()).unwrap(),
            vec![Located::new("guests".into(), 0..1)]
        );
        validate_value_expression(expression, &types, &mut ample()).unwrap();
        let discovered = value_expression_types(expression, &types, &mut ample()).unwrap();
        let [ValueType::KeyedArray(discovered)] = discovered.as_slice() else {
            panic!("keyed type")
        };
        assert_eq!(
            discovered.axes,
            Some(vec![
                ValueKeyType::Member(NominalTypeId(7)),
                ValueKeyType::Integer
            ])
        );
        assert!(
            !discovered.elements.is_empty()
                && discovered
                    .elements
                    .iter()
                    .all(|ty| *ty == ValueType::Integer)
        );
        let result = evaluate_value(expression, &bindings, &mut ample()).unwrap();
        assert_eq!(result, expected);
        assert_eq!(
            copy_constructed_value(&result, &mut ample(), Location::Source(3..9)).unwrap(),
            expected
        );
        assert_eq!(
            copy_constructed_value(&result, &mut budget(100_000, 5, 64), Location::Source(3..9))
                .unwrap_err()
                .location,
            Location::Source(3..9)
        );
        assert!(
            evaluate_value(expression, &bindings, &mut budget(100_000, 5, 64))
                .unwrap_err()
                .message
                .contains("result limit")
        );
    }

    for (domains, expected_axes) in [
        (
            vec![
                literal(V::Set {
                    element_type: Some(SetElementType::Integer),
                    elements: vec![].into(),
                }),
                e(K::Array(vec![int(8), int(1)])),
            ],
            vec![integer_axis(&[]), integer_axis(&[8, 1])],
        ),
        (
            vec![literal_member_set(), e(K::Array(vec![int(8), int(1)]))],
            vec![
                KeyAxis {
                    key_type: ValueKeyType::Member(NominalTypeId(7)),
                    keys: vec![],
                },
                integer_axis(&[8, 1]),
            ],
        ),
        (
            vec![
                typed_empty_array(D::Integer),
                e(K::Array(vec![int(8), int(1)])),
            ],
            vec![integer_axis(&[]), integer_axis(&[8, 1])],
        ),
        (
            vec![
                typed_empty_array(D::Member(NominalTypeId(7))),
                e(K::Array(vec![int(8), int(1)])),
            ],
            vec![
                KeyAxis {
                    key_type: ValueKeyType::Member(NominalTypeId(7)),
                    keys: vec![],
                },
                integer_axis(&[8, 1]),
            ],
        ),
        (sources(), vec![guest_axis(), integer_axis(&[8, 1, 4])]),
    ] {
        let empty = e(K::KeyedArray {
            axes: None,
            declared_axis_sources: Some(domains),
            entries: vec![],
        });
        let result = evaluate_value(&empty, &bindings, &mut ample()).unwrap();
        assert_eq!(
            result,
            V::KeyedArray {
                axes: expected_axes,
                entries: vec![]
            }
        );
        assert_eq!(
            copy_constructed_value(&result, &mut ample(), Location::Source(3..9)).unwrap(),
            result
        );
    }

    fn typed_empty_array(element: D<()>) -> E {
        literal(
            zirql::declare_constructed(
                V::Array(vec![]),
                D::Sequence(Box::new(element)),
                &mut ample(),
                Location::Query(0..1),
            )
            .unwrap(),
        )
    }

    fn literal_member_set() -> E {
        literal(V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(7))),
            elements: vec![].into(),
        })
    }
}

#[test]
fn declared_sources_fail_before_cells_and_generators_with_original_locations() {
    let failing_source = |range| {
        Located::new(
            K::Range {
                lower: Box::new(Located::new(
                    K::Binary {
                        operator: B::IntegerDivide,
                        left: Box::new(int(1)),
                        right: Box::new(int(0)),
                    },
                    range,
                )),
                upper: Box::new(int(1)),
            },
            20..40,
        )
    };
    let failing_cell = || {
        Located::new(
            K::Binary {
                operator: B::IntegerDivide,
                left: Box::new(int(2)),
                right: Box::new(int(0)),
            },
            80..90,
        )
    };
    let domains = || vec![failing_source(21..26), failing_source(31..36)];
    let indexed = e(K::KeyedArray {
        axes: None,
        declared_axis_sources: Some(domains()),
        entries: vec![(e(K::Tuple(vec![int(2), int(1)])), failing_cell())],
    });
    let comprehension = e(K::KeyedComprehension {
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("i")],
            source: e(K::Array(vec![failing_cell()])),
            filter: None,
        })],
        key: Box::new(e(K::Tuple(vec![name("i"), int(1)]))),
        head: Box::new(int(0)),
        declared_axis_sources: Some(domains()),
        empty_axis_sources: None,
    });
    let other_member = NominalMember {
        type_id: NominalTypeId(8),
        ..member(11, 1)
    };
    let wrong_identity = e(K::KeyedArray {
        axes: None,
        declared_axis_sources: Some(vec![literal(V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(8))),
            elements: vec![V::Member(other_member)].into(),
        })]),
        entries: vec![(
            Located::new(K::Literal(V::Member(member(11, 1))), 71..72),
            int(1),
        )],
    });
    assert_eq!(
        evaluate(&wrong_identity).unwrap_err().location,
        Location::Query(71..72)
    );
    for expression in [&indexed, &comprehension] {
        assert_eq!(
            evaluate(expression).unwrap_err().location,
            Location::Query(21..26)
        );
    }
    let later = e(K::KeyedArray {
        axes: None,
        declared_axis_sources: Some(vec![
            literal(V::Set {
                element_type: Some(SetElementType::Integer),
                elements: vec![].into(),
            }),
            failing_source(31..36),
        ]),
        entries: vec![],
    });
    assert_eq!(
        evaluate(&later).unwrap_err().location,
        Location::Query(31..36)
    );
    let own_name = e(K::KeyedComprehension {
        clauses: vec![clause(G::In {
            case: false,
            binders: vec![binder("i")],
            source: e(K::Array(vec![int(1)])),
            filter: None,
        })],
        key: Box::new(name("i")),
        head: Box::new(int(0)),
        declared_axis_sources: Some(vec![Located::new(K::Name("i".into()), 12..13)]),
        empty_axis_sources: None,
    });
    assert_eq!(
        evaluate(&own_name).unwrap_err().location,
        Location::Query(12..13)
    );
    for expression in [
        e(K::KeyedArray {
            axes: Some(vec![integer_axis(&[1])]),
            declared_axis_sources: Some(vec![e(K::Array(vec![int(1)]))]),
            entries: vec![],
        }),
        e(K::KeyedComprehension {
            clauses: vec![],
            key: Box::new(int(1)),
            head: Box::new(int(0)),
            declared_axis_sources: Some(vec![e(K::Array(vec![int(1)]))]),
            empty_axis_sources: Some(vec![e(K::Array(vec![int(1)]))]),
        }),
    ] {
        assert!(
            evaluate(&expression)
                .unwrap_err()
                .message
                .contains("conflicting")
        );
    }
}

#[test]
fn scoped_filters_match_comprehensions_and_rebind_only_the_unnamed_subject() {
    let rows = V::Array(vec![
        V::Record(vec![
            ("id".into(), V::Integer(2)),
            ("active".into(), V::String("yes".into())),
        ]),
        V::Record(vec![
            ("id".into(), V::Integer(3)),
            ("active".into(), V::String("no".into())),
        ]),
        V::Record(vec![
            ("id".into(), V::Integer(2)),
            ("active".into(), V::String("yes".into())),
        ]),
    ]);
    let run = |source: &str, value: V, allowance: &mut Budget| {
        let mut host = WordInput {
            value,
            unknown: false,
        };
        let mut reader = zirql::ArgumentReader::new(source, 0).unwrap();
        let expression = zirql::read_value_expression(&mut reader, &mut host, allowance, 64)?;
        assert!(reader.remaining().trim().is_empty(), "{source}");
        let types = BTreeMap::from([(
            "words".into(),
            vec![constructed_value_type(&host.value, allowance, Location::Query(0..1)).unwrap()],
        )]);
        let result = zirql::evaluate_query_value(&expression, &types, &mut host, allowance);
        zirql::dispose_native_value(expression);
        result
    };
    let expected = run(
        r#"[p for p in words if p.active == "yes"]"#,
        copy_constructed_value(&rows, &mut ample(), Location::Query(0..1)).unwrap(),
        &mut ample(),
    )
    .unwrap();
    for source in [
        r#"filter(p => p.active == "yes")"#,
        r#"filter(_.active == "yes" and _.id < 3)"#,
        r#"[p for p in words] | filter(p => length([1,2] | filter(_ == p.id)) > 0 and if _.active == "yes" { true } else { false })"#,
    ] {
        assert_eq!(
            run(
                source,
                copy_constructed_value(&rows, &mut ample(), Location::Query(0..1)).unwrap(),
                &mut ample()
            )
            .unwrap(),
            expected
        );
    }
    assert!(run("[1 for absent in [1]]", V::Array(vec![]), &mut ample()).is_err());
    assert!(
        run(
            "filter(absent => true)",
            V::Array(vec![V::Integer(1)]),
            &mut ample()
        )
        .is_err()
    );
    assert_eq!(
        run(
            "filter(absent => true)",
            V::Array(vec![V::Absent]),
            &mut ample()
        )
        .unwrap(),
        V::Array(vec![V::Absent])
    );
    assert_eq!(
        run(
            "if true { let absent = null; 1 } else { 0 }",
            V::Array(vec![]),
            &mut ample()
        )
        .unwrap(),
        V::Integer(1)
    );
    assert!(
        run(
            "if true { let absent = 1; true } else { false }",
            V::Array(vec![]),
            &mut ample()
        )
        .is_err()
    );
    let lists = || {
        V::Array(vec![
            V::Array(vec![V::Integer(1), V::Integer(3), V::Integer(99)]),
            V::Array(vec![V::Integer(0), V::Integer(2)]),
        ])
    };
    let projected = run(
        "[[a,b] for [a,b,..] in words if a > 0]",
        lists(),
        &mut ample(),
    )
    .unwrap();
    assert_eq!(
        projected,
        V::Array(vec![V::Array(vec![V::Integer(1), V::Integer(3)])])
    );
    let selected = run(
        "filter([a,b,..] => length(_) >= 2 and length([a] | filter(_ == a)) == 1 and a + b > 3)",
        lists(),
        &mut ample(),
    )
    .unwrap();
    assert_eq!(
        selected,
        V::Array(vec![V::Array(vec![
            V::Integer(1),
            V::Integer(3),
            V::Integer(99)
        ])])
    );
    assert_eq!(
        run(
            "filter((a,_) => _.1 == a and _.2 > 0)",
            V::Array(vec![V::Tuple(vec![V::Integer(1), V::Integer(2)])]),
            &mut ample()
        )
        .unwrap(),
        V::Array(vec![V::Tuple(vec![V::Integer(1), V::Integer(2)])])
    );
    assert_eq!(
        run(
            "match ([1]) { [a,b,..] => a + b, [x] => x, _ => 0 }",
            lists(),
            &mut ample()
        )
        .unwrap(),
        V::Integer(1)
    );
    let short = "filter([a,b,..] => true)";
    let error = run(
        short,
        V::Array(vec![V::Array(vec![V::Integer(1)])]),
        &mut ample(),
    )
    .unwrap_err();
    assert!(
        matches!(error.location, Location::Query(ref range) if &short[range.clone()] == "[a,b,..]")
    );
    for source in [
        "filter([a,a] => true)",
        "filter([a,..tail] => true)",
        "filter([..,a] => true)",
        "[a for [a] in a]",
        "filter([a,..] => a + 1)",
    ] {
        assert!(run(source, lists(), &mut ample()).is_err(), "{source}");
    }
    let empty_lists = declared(
        V::Array(vec![]),
        D::Sequence(Box::new(D::Sequence(Box::new(D::String)))),
    );
    assert!(run("filter([first,..] => first > 0)", empty_lists, &mut ample()).is_err());
    let typed_empty = declared(
        V::Array(vec![]),
        D::Sequence(Box::new(D::Record(vec![("active".into(), D::String)]))),
    );
    let empty_fields = || {
        declared(
            V::Array(vec![]),
            D::Sequence(Box::new(D::Record(vec![(
                "items".into(),
                D::Sequence(Box::new(D::Integer)),
            )]))),
        )
    };
    assert_eq!(
        run(
            "[p.items | filter(_ > 1) for p in words]",
            empty_fields(),
            &mut ample()
        )
        .unwrap(),
        V::Array(vec![])
    );
    let invalid = "[p.missing | filter(_ > 1) for p in words]";
    let error = run(invalid, empty_fields(), &mut ample()).unwrap_err();
    assert!(
        matches!(error.location, Location::Query(ref range) if &invalid[range.clone()] == "p.missing"),
        "{error}"
    );
    for source in [
        r#"filter(_.active)"#,
        "filter(false and missing == 1)",
        "filter(not_declared(_))",
    ] {
        assert!(
            run(
                source,
                copy_constructed_value(&typed_empty, &mut ample(), Location::Query(0..1)).unwrap(),
                &mut ample()
            )
            .is_err(),
            "{source}"
        );
    }
    let source = "filter(1 div (_.id - 2) == 0)";
    let error = run(
        source,
        copy_constructed_value(&rows, &mut ample(), Location::Query(0..1)).unwrap(),
        &mut ample(),
    )
    .unwrap_err();
    assert!(
        matches!(error.location, Location::Query(ref range) if &source[range.clone()] == "1 div (_.id - 2)")
    );
    assert!(
        run(
            "filter(_ == 1)",
            V::Array((0..100).map(|_| V::Integer(1)).collect()),
            &mut budget(1000, 10000, 64)
        )
        .is_err()
    );
    assert!(run("_", rows, &mut ample()).is_err());
    // A supplied ordinary binding cannot create an unnamed predicate scope.
    let bindings = Bindings::new([(Located::new("_".into(), 0..1), V::Boolean(true))]).unwrap();
    assert!(evaluate_value(&e(K::PredicateSubject), &bindings, &mut ample()).is_err());
    let predicate = zirql::PredicateArgument {
        argument: None,
        body: Box::new(binary(B::Equal, e(K::PredicateSubject), int(4))),
    };
    assert_eq!(evaluate(&predicate.bind(int(4))).unwrap(), V::Boolean(true));
}

#[test]
fn scalar_predicate_facts_reject_required_structure_before_values_exist() {
    use zirql::PatternKind as P;
    let prepare = |argument, body, ty| {
        let expression = zirql::PredicateArgument {
            argument,
            body: Box::new(body),
        }
        .bind(name("scalar"));
        value_expression_types(
            &expression,
            &BTreeMap::from([("scalar".into(), ty)]),
            &mut ample(),
        )
    };
    let list = || {
        pattern(P::List {
            elements: vec![pattern_name("first")],
            ignore_rest: false,
        })
    };
    for header in [
        list(),
        pattern(P::Tuple(vec![pattern_name("first")])),
        record_pattern(vec![("first", pattern_name("first"))], false),
    ] {
        let error = prepare(
            Some(header),
            literal(V::Boolean(true)),
            ValueType::ScalarKind,
        )
        .unwrap_err();
        assert_eq!(error.location, Location::Query(70..80));
    }
    assert!(prepare(Some(list()), literal(V::Boolean(true)), ValueType::Integer).is_err());
    // Required admission follows the same recursion through established children.
    assert!(
        prepare(
            Some(pattern(P::Tuple(vec![list()]))),
            literal(V::Boolean(true)),
            ValueType::Tuple(vec![ValueType::ScalarKind]),
        )
        .is_err()
    );
    for body in [
        field(e(K::PredicateSubject), "field"),
        field(name("scalar_name"), "field"),
        access(e(K::PredicateSubject), int(1)),
        tuple_access(e(K::PredicateSubject), 1),
        field(binary(B::Add, e(K::PredicateSubject), int(1)), "field"),
        binary(B::Concatenate, e(K::PredicateSubject), e(K::Array(vec![]))),
    ] {
        assert!(
            prepare(
                Some(pattern_name("scalar_name")),
                body,
                ValueType::ScalarKind
            )
            .is_err()
        );
    }
    assert_eq!(
        value_expression_types(
            &binary(B::Add, name("scalar"), int(1)),
            &BTreeMap::from([("scalar".into(), ValueType::ScalarKind)]),
            &mut ample(),
        )
        .unwrap(),
        vec![ValueType::ScalarKind],
    );
    let numeric = || binary(B::Greater, binary(B::Add, name("n"), int(1)), int(0));
    assert_eq!(
        prepare(Some(pattern_name("n")), numeric(), ValueType::ScalarKind).unwrap(),
        vec![ValueType::Boolean],
    );
    assert!(prepare(Some(pattern_name("n")), numeric(), ValueType::String).is_err());
    assert!(
        prepare(
            Some(pattern_name("n")),
            numeric(),
            ValueType::Declared {
                actual: Box::new(ValueType::ScalarKind),
                declaration: D::String,
            },
        )
        .is_err()
    );
    let matched = value_match(int(3), vec![arm(list(), None, int(0)), fallback(int(7))]);
    assert_eq!(evaluate(&matched).unwrap(), V::Integer(7));
}

#[test]
fn scalar_predicate_operands_defer_compatible_kinds_and_keep_container_constraints() {
    let prepare = |body, scalar, container| {
        let expression = zirql::PredicateArgument {
            argument: None,
            body: Box::new(body),
        }
        .bind(name("scalar"));
        value_expression_types(
            &expression,
            &BTreeMap::from([("scalar".into(), scalar), ("container".into(), container)]),
            &mut ample(),
        )
    };
    let membership = || binary(B::In, e(K::PredicateSubject), name("container"));
    assert_eq!(
        prepare(
            membership(),
            ValueType::ScalarKind,
            ValueType::Set(Some(SetElementType::Integer))
        )
        .unwrap(),
        vec![ValueType::Boolean]
    );
    assert!(
        prepare(
            membership(),
            ValueType::String,
            ValueType::Set(Some(SetElementType::Integer))
        )
        .is_err()
    );
    assert!(
        prepare(
            membership(),
            ValueType::ScalarKind,
            ValueType::Set(Some(SetElementType::Member(NominalTypeId(1))))
        )
        .is_err()
    );

    let lookup = || {
        binary(
            B::Greater,
            access(name("container"), e(K::PredicateSubject)),
            int(0),
        )
    };
    let dictionary = || ValueType::Record(vec![("foo".into(), ValueType::Integer)]);
    assert_eq!(
        prepare(lookup(), ValueType::ScalarKind, dictionary()).unwrap(),
        vec![ValueType::Boolean]
    );
    assert!(prepare(lookup(), ValueType::Integer, dictionary()).is_err());

    let keyed = |axes| {
        ValueType::KeyedArray(KeyedArrayType {
            axes: Some(axes),
            elements: vec![ValueType::Integer],
        })
    };
    assert_eq!(
        prepare(
            lookup(),
            ValueType::ScalarKind,
            keyed(vec![ValueKeyType::Integer])
        )
        .unwrap(),
        vec![ValueType::Boolean]
    );
    assert!(
        prepare(
            lookup(),
            ValueType::String,
            keyed(vec![ValueKeyType::Integer])
        )
        .is_err()
    );
    assert!(
        prepare(
            lookup(),
            ValueType::ScalarKind,
            keyed(vec![ValueKeyType::Member(NominalTypeId(1))])
        )
        .is_err()
    );
    assert!(
        prepare(
            lookup(),
            ValueType::ScalarKind,
            keyed(vec![ValueKeyType::Integer, ValueKeyType::Integer])
        )
        .is_err()
    );
}

fn run_query(source: &str, value: V, allowance: &mut Budget) -> Result<V, zirql::QueryError> {
    let mut host = WordInput {
        value,
        unknown: false,
    };
    let mut reader = zirql::ArgumentReader::new(source, 0)?;
    let expression = zirql::read_value_expression(&mut reader, &mut host, allowance, 64)?;
    assert!(reader.remaining().trim().is_empty(), "{source}");
    let types = BTreeMap::from([(
        "words".into(),
        vec![constructed_value_type(
            &host.value,
            allowance,
            Location::Query(0..1),
        )?],
    )]);
    let result = zirql::evaluate_query_value(&expression, &types, &mut host, allowance);
    zirql::dispose_native_value(expression);
    result
}

#[test]
fn scoped_mapping_constructs_ordered_capacity_reports() {
    let capacities = || {
        V::Array(vec![
            V::Record(vec![
                ("name".into(), V::String("cpu".into())),
                ("limit".into(), V::Integer(8)),
                (
                    "demands".into(),
                    V::Array(vec![V::Integer(2), V::Integer(3)]),
                ),
            ]),
            V::Record(vec![
                ("name".into(), V::String("gpu".into())),
                ("limit".into(), V::Integer(6)),
                ("demands".into(), V::Array(vec![V::Integer(4)])),
            ]),
        ])
    };
    let report = || {
        record(vec![
            ("capacity", field(name("c"), "name")),
            ("demand", e(K::PredicateSubject)),
            (
                "remaining",
                binary(
                    B::Subtract,
                    field(name("c"), "limit"),
                    e(K::PredicateSubject),
                ),
            ),
        ])
    };
    let expression = e(K::FlatMap {
        array: Box::new(literal(capacities())),
        argument: zirql::PredicateArgument {
            argument: Some(binder("c")),
            body: Box::new(e(K::Map {
                array: Box::new(field(e(K::PredicateSubject), "demands")),
                argument: zirql::PredicateArgument {
                    argument: None,
                    body: Box::new(report()),
                },
            })),
        },
    });
    let expected = V::Array(
        [("cpu", 2, 6), ("cpu", 3, 5), ("gpu", 4, 2)]
            .into_iter()
            .map(|(capacity, demand, remaining)| {
                V::Record(vec![
                    ("capacity".into(), V::String(capacity.into())),
                    ("demand".into(), V::Integer(demand)),
                    ("remaining".into(), V::Integer(remaining)),
                ])
            })
            .collect(),
    );
    assert_eq!(evaluate(&expression).unwrap(), expected);
    assert!(
        free_value_names(&expression, &BTreeMap::new(), &mut ample())
            .unwrap()
            .is_empty()
    );
    assert_eq!(run_query(r#"flat_map(c => _.demands | map({"capacity": c.name, "demand": _, "remaining": c.limit - _}))"#, capacities(), &mut ample()).unwrap(), expected);
    assert_eq!(
        run_query(
            "map([_]) | flat_map(_) | map(demand => demand + 1)",
            V::Array(vec![V::Integer(2), V::Integer(3)]),
            &mut ample()
        )
        .unwrap(),
        V::Array(vec![V::Integer(3), V::Integer(4)])
    );
}

#[test]
fn mapping_preflights_empty_bodies_and_refuses_non_sequences_and_excess_expansion() {
    let empty = || {
        declared(
            V::Array(vec![]),
            D::Sequence(Box::new(D::Record(vec![("demand".into(), D::Integer)]))),
        )
    };
    assert_eq!(
        run_query("map(_.demand + 1)", empty(), &mut ample()).unwrap(),
        V::Array(vec![])
    );
    let error = run_query("map(_.missing)", empty(), &mut ample()).unwrap_err();
    assert!(
        matches!(error.location, Location::Query(ref range) if *range == (4..13)),
        "{error}"
    );
    let error = run_query("flat_map(_.demand)", empty(), &mut ample()).unwrap_err();
    assert!(error.message.contains("positional"), "{error}");
    let error = run_query(
        "map([first, second] => first)",
        V::Array(vec![V::Array(vec![V::Integer(1)])]),
        &mut ample(),
    )
    .unwrap_err();
    assert!(error.message.contains("pattern"), "{error}");
    let error = run_query("[1] | flat_map({2, 3})", V::Array(vec![]), &mut ample()).unwrap_err();
    assert!(error.message.contains("positional"), "{error}");
    let error = run_query("{1, 2} | map(_)", V::Array(vec![]), &mut ample()).unwrap_err();
    assert!(error.message.contains("positional"), "{error}");
    let error = run_query("[1] | flat_map([2: 3])", V::Array(vec![]), &mut ample()).unwrap_err();
    assert!(error.message.contains("positional"), "{error}");
    assert!(
        run_query(
            "[1] | flat_map([i for i in 1..16])",
            V::Array(vec![]),
            &mut budget(100_000, 150, 64)
        )
        .is_ok()
    );
    let error = run_query(
        "[1, 2] | flat_map([i for i in 1..16])",
        V::Array(vec![]),
        &mut budget(100_000, 150, 64),
    )
    .unwrap_err();
    assert_eq!(error.category, zirql::QueryErrorCategory::ResultLimit);
    let mut limited = Budget::for_program(ProgramLimits {
        stream_items: 3,
        work: 100_000,
        results: 10_000,
        nesting: 64,
    });
    let error = run_query("[1, 2] | flat_map([_, _])", V::Array(vec![]), &mut limited).unwrap_err();
    assert_eq!(error.category, zirql::QueryErrorCategory::ItemLimit);
}

#[test]
fn case_generators_select_present_measurements_through_reader_and_callable_api() {
    let measurements = || {
        V::Array(vec![
            V::Record(vec![
                ("kind".into(), V::String("measurement".into())),
                ("name".into(), V::String("ash".into())),
                ("reading".into(), V::Integer(12)),
                ("unit".into(), V::String("cm".into())),
            ]),
            V::Record(vec![
                ("kind".into(), V::String("note".into())),
                ("name".into(), V::String("memo".into())),
                ("reading".into(), V::Integer(99)),
            ]),
            V::Record(vec![
                ("kind".into(), V::String("measurement".into())),
                ("name".into(), V::String("birch".into())),
                ("reading".into(), V::Absent),
            ]),
            V::Integer(5),
            V::Record(vec![
                ("kind".into(), V::String("measurement".into())),
                ("name".into(), V::String("oak".into())),
                ("reading".into(), V::Integer(7)),
            ]),
        ])
    };
    let expected = V::Array(vec![
        V::Record(vec![
            ("name".into(), V::String("ash".into())),
            ("reading".into(), V::Integer(12)),
        ]),
        V::Record(vec![
            ("name".into(), V::String("oak".into())),
            ("reading".into(), V::Integer(7)),
        ]),
    ]);
    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![clause(G::In {
            case: true,
            binders: vec![record_pattern(
                vec![
                    (
                        "kind",
                        pattern(zirql::PatternKind::Literal(V::String("measurement".into()))),
                    ),
                    ("name", binder("name")),
                    (
                        "reading",
                        pattern(zirql::PatternKind::Present(Box::new(binder("reading")))),
                    ),
                ],
                true,
            )],
            source: literal(measurements()),
            filter: None,
        })],
        head: Box::new(record(vec![
            ("name", name("name")),
            ("reading", name("reading")),
        ])),
    });
    assert_eq!(evaluate(&expression).unwrap(), expected);
    for source in [
        r#"[{"name": name, "reading": reading} for case {kind: "measurement", name, reading: present(reading), ..} in words]"#,
        r#"[{"name": name, "reading": reading} | case {kind: "measurement", name, reading: present(reading), ..} in words]"#,
    ] {
        assert_eq!(
            run_query(source, measurements(), &mut ample()).unwrap(),
            expected
        );
    }
}

#[test]
fn case_generators_keep_candidate_scope_order_and_skip_unmatched_work() {
    let source = r#"[(label, n, copy) for case [label, present(n)] in [["ash", 2], ["absent", null], ["short"], ["negative", -1], ["oak", 1]] if n > 0 for copy in 1..n]"#;
    assert_eq!(
        run_query(source, V::Absent, &mut ample()).unwrap(),
        V::Array(vec![
            V::Tuple(vec![V::String("ash".into()), V::Integer(2), V::Integer(1)]),
            V::Tuple(vec![V::String("ash".into()), V::Integer(2), V::Integer(2)]),
            V::Tuple(vec![V::String("oak".into()), V::Integer(1), V::Integer(1)]),
        ])
    );
    assert_eq!(
        run_query(
            "[[a, b] | case present(a), present(b) in [1, null, 2] where a < b]",
            V::Absent,
            &mut ample()
        )
        .unwrap(),
        V::Array(vec![V::Array(vec![V::Integer(1), V::Integer(2)])])
    );
    assert_eq!(
        run_query(
            "[1 div 0 for case 0 in [1, 2] if 1 div 0 == 0 for n in 1..(1 div 0)]",
            V::Absent,
            &mut ample()
        )
        .unwrap(),
        V::Array(vec![])
    );
    assert_eq!(
        run_query("[n for case present(n) in []]", V::Absent, &mut ample()).unwrap(),
        V::Array(vec![])
    );
    assert_eq!(
        run_query("['case' for 'case' in [3]]", V::Absent, &mut ample()).unwrap(),
        V::Array(vec![V::Integer(3)])
    );
}

#[test]
fn case_generators_preserve_errors_preflight_and_rejected_candidate_budgets() {
    let error =
        run_query("[n for [n] in [[1], []] if false]", V::Absent, &mut ample()).unwrap_err();
    assert!(error.message.contains("generator pattern"), "{error}");
    let error = run_query(
        "if true { let [n] = []; n } else { 0 }",
        V::Absent,
        &mut ample(),
    )
    .unwrap_err();
    assert!(error.message.contains("pattern"), "{error}");
    let error = run_query(
        "[1 / n for case present(n) in [null, 0]]",
        V::Absent,
        &mut ample(),
    )
    .unwrap_err();
    assert!(error.message.contains("zero"), "{error}");
    let error = run_query("[n | case n = 1]", V::Absent, &mut ample()).unwrap_err();
    assert!(error.message.contains("case"), "{error}");
    let error = run_query("[true + 1 for case 0 in []]", V::Absent, &mut ample()).unwrap_err();
    assert!(error.message.contains("numeric"), "{error}");
    let error = run_query("[0 for case 0 in [] if 1]", V::Absent, &mut ample()).unwrap_err();
    assert!(error.message.contains("Boolean"), "{error}");

    let mut host = WordInput {
        value: V::Absent,
        unknown: false,
    };
    let expression = zirql::read_value_expression(
        &mut zirql::ArgumentReader::new("[0 for case 0 in 1..5000]", 0).unwrap(),
        &mut host,
        &mut ample(),
        64,
    )
    .unwrap();
    let error = zirql::evaluate_query_value(
        &expression,
        &BTreeMap::new(),
        &mut host,
        &mut budget(1000, 100_000, 64),
    )
    .unwrap_err();
    assert_eq!(error.category, zirql::QueryErrorCategory::WorkLimit);
    zirql::dispose_native_value(expression);
}
