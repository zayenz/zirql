use std::collections::BTreeMap;

use zirql::{
    ArrayType, Bindings, Budget, ConstructedValue as V, KeyAxis, KeyedArrayType, KeyedEntry,
    Limits, Located, Location, NominalMember, NominalMemberId, NominalTypeId, ProgramLimits,
    SetElementType, ValueBinaryOperator as B, ValueConstructor, ValueExpression as E,
    ValueExpressionKind as K, ValueGenerator as G, ValueKey, ValueKeyType, ValueReducer as R,
    ValueType, ValueUnaryOperator as U, constructed_value_type, copy_constructed_value,
    evaluate_value, free_value_names, validate_value_expression,
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
fn binder(value: &str) -> Located<Option<String>> {
    Located::new(Some(value.into()), 10..11)
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
fn evaluate(expression: &E) -> Result<V, zirql::QueryError> {
    evaluate_value(expression, &Bindings::default(), &mut ample())
}
fn choose(skipped: E) -> E {
    e(K::Conditional {
        branches: vec![(literal(V::Boolean(false)), skipped)],
        otherwise: Box::new(int(7)),
    })
}

#[test]
fn positional_report_preserves_scopes_order_and_budgeted_host_sequence() {
    let labels = V::Array(vec![V::String("beta".into()), V::String("alpha".into())]);
    let expression = e(K::Comprehension {
        constructor: ValueConstructor::Array,
        clauses: vec![
            clause(G::In {
                binders: vec![binder("p")],
                source: e(K::IndexSet(Box::new(Located::new(
                    K::Name("labels".into()),
                    20..26,
                )))),
                filter: None,
            }),
            clause(G::In {
                binders: vec![binder("i"), binder("j")],
                source: e(K::Array(vec![int(2), int(1)])),
                filter: Some(binary(B::NotEqual, name("i"), name("j"))),
            }),
            clause(G::In {
                binders: vec![binder("k")],
                source: e(K::Range {
                    lower: Box::new(int(1)),
                    upper: Box::new(name("p")),
                }),
                filter: Some(binary(B::Equal, name("k"), name("p"))),
            }),
            clause(G::Bind {
                binder: binder("row"),
                value: record(vec![
                    ("label", access(name("labels"), name("p"))),
                    ("triple", e(K::Tuple(vec![name("i"), name("j"), name("k")]))),
                ]),
                filter: Some(binary(B::GreaterEqual, name("p"), int(1))),
            }),
            clause(G::In {
                binders: vec![Located::new(None, 12..13)],
                source: e(K::Array(vec![literal(V::Boolean(true))])),
                filter: None,
            }),
        ],
        head: Box::new(record(vec![
            ("name", field(name("row"), "label")),
            (
                "pair",
                e(K::Tuple(vec![
                    tuple_access(field(name("row"), "triple"), 1),
                    tuple_access(field(name("row"), "triple"), 2),
                ])),
            ),
            ("position", tuple_access(field(name("row"), "triple"), 3)),
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
            binders: vec![Located::new(Some("existing".into()), 11..19)],
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
            binder: Located::new(Some("x".repeat(256)), 37..38),
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
    let unsupported = Located::new(K::Set(vec![literal(V::String("x".into()))]), 50..55);
    assert_eq!(
        evaluate(&choose(unsupported)).unwrap_err().location,
        Location::Query(50..55)
    );
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
            elements: vec![V::Integer(1), V::Integer(2)]
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
            elements: vec![]
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
fn integer_axis(keys: &[i64]) -> KeyAxis {
    KeyAxis {
        key_type: ValueKeyType::Integer,
        keys: keys.iter().copied().map(ValueKey::Integer).collect(),
    }
}
fn empty_keyed(sources: Option<Vec<E>>) -> E {
    e(K::KeyedComprehension {
        clauses: vec![
            clause(G::In {
                binders: vec![binder("i")],
                source: e(K::Range {
                    lower: Box::new(int(2)),
                    upper: Box::new(int(1)),
                }),
                filter: None,
            }),
            clause(G::In {
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
            elements: vec![V::Member(member(11, 1)), V::Member(member(22, 4))]
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
        clauses: vec![
            clause(G::In {
                binders: vec![binder("i")],
                source: e(K::Array(vec![int(9), int(3)])),
                filter: None,
            }),
            clause(G::In {
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
        entries
            .into_iter()
            .map(|entry| entry.value)
            .collect::<Vec<_>>(),
        vec![V::Integer(92), V::Integer(91), V::Integer(31)]
    );
    let one = e(K::KeyedArray {
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
            elements: vec![V::Integer(3), V::Integer(9)]
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
            elements: vec![V::Member(member(11, 1)), V::Member(member(22, 4))]
        }
    );

    let empty_bindings = Bindings::new([(
        Located::new("empty_members".into(), 0..13),
        V::Set {
            element_type: Some(SetElementType::Member(NominalTypeId(7))),
            elements: vec![],
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
        clauses: vec![clause(G::In {
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
fn nominal_keyed_duplicate_incompatible_empty_and_budget_refusals_are_located() {
    let duplicate = e(K::KeyedArray {
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
            elements: vec![],
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
                    message: "host product overflow".into(),
                })?;
            if count != entries.len() {
                return Err(zirql::QueryError {
                    location: location.clone(),
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
