use std::collections::BTreeMap;

use zirql::{
    ArrayType, Bindings, Budget, ConstructedValue as V, Limits, Located, Location, ProgramLimits,
    SetElementType, ValueBinaryOperator as B, ValueConstructor, ValueExpression as E,
    ValueExpressionKind as K, ValueGenerator as G, ValueType, ValueUnaryOperator as U,
    constructed_value_type, copy_constructed_value, evaluate_value, free_value_names,
    validate_value_expression,
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
