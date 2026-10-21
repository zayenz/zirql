use serde_json::{Value, json};
use zirql::{
    Budget, ConstructedValue as V, DeclaredType, JsonLayout, JsonRepresentation, KeyAxis,
    KeyedEntry, Location, NominalMember, NominalMemberId, NominalTypeId, ProgramLimits,
    QueryErrorCategory, SetElementType, ValueKey, ValueKeyType, render_json, render_json_value,
    to_json_value,
};

fn limits() -> ProgramLimits {
    ProgramLimits {
        work: 100_000,
        results: 100_000,
        nesting: 64,
        stream_items: 100,
    }
}

fn member(name: &str, order: usize) -> NominalMember {
    NominalMember {
        type_id: NominalTypeId(7),
        member_id: NominalMemberId(order as u64),
        declaration_order: order,
        type_name: "Color".into(),
        member_name: name.into(),
    }
}

fn fixture() -> V {
    V::Record(vec![
        (
            "z".into(),
            V::Tuple(vec![
                V::Absent,
                V::Boolean(true),
                V::Integer(-4),
                V::Float(1.25),
                V::Declared {
                    value: Box::new(V::String("quote\"\n\\\tλ".into())),
                    declaration: DeclaredType::String,
                },
            ]),
        ),
        (
            "a".into(),
            V::Set {
                element_type: Some(SetElementType::Integer),
                elements: vec![V::Integer(9), V::Integer(1)].into(),
            },
        ),
        (
            "dictionary".into(),
            V::Dictionary(
                vec![
                    (
                        V::Tuple(vec![V::Dictionary(
                            vec![(V::Integer(3), V::String("nested".into()))].into(),
                        )]),
                        V::Array(vec![V::Integer(2), V::Member(member("Blue", 1))]),
                    ),
                    (V::Integer(8), V::Boolean(false)),
                ]
                .into(),
            ),
        ),
        ("member".into(), V::Member(member("Blue", 1))),
        (
            "keyed".into(),
            V::KeyedArray {
                axes: vec![
                    KeyAxis {
                        key_type: ValueKeyType::Integer,
                        keys: vec![
                            ValueKey::Integer(5),
                            ValueKey::Integer(2),
                            ValueKey::Integer(9),
                        ],
                    },
                    KeyAxis {
                        key_type: ValueKeyType::Member(NominalTypeId(7)),
                        keys: vec![
                            ValueKey::Member(member("Blue", 1)),
                            ValueKey::Member(member("Red", 0)),
                        ],
                    },
                ],
                entries: vec![
                    KeyedEntry {
                        key: vec![ValueKey::Integer(5), ValueKey::Member(member("Blue", 1))],
                        value: V::Absent,
                    },
                    KeyedEntry {
                        key: vec![ValueKey::Integer(2), ValueKey::Member(member("Red", 0))],
                        value: V::Record(vec![("cell".into(), V::Integer(20))]),
                    },
                ],
            },
        ),
    ])
}

fn convert(value: &V, representation: JsonRepresentation) -> Value {
    to_json_value(
        value,
        representation,
        &mut Budget::for_program(limits()),
        Location::Query(2..8),
    )
    .unwrap()
}

#[test]
fn ordinary_and_tagged_encodings_preserve_structure_order_and_sparse_shape() {
    let value = fixture();
    let ordinary = convert(&value, JsonRepresentation::Ordinary);
    assert_eq!(
        ordinary
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["z", "a", "dictionary", "member", "keyed"]
    );
    assert_eq!(
        ordinary["z"],
        json!([null, true, -4, 1.25, "quote\"\n\\\tλ"])
    );
    assert_eq!(ordinary["a"], json!([9, 1]));
    assert_eq!(
        ordinary["dictionary"]["entries"][0]["key"],
        json!([
            {"kind":"dictionary","entries":[{"key":3,"value":"nested"}]}
        ])
    );
    assert_eq!(
        ordinary["dictionary"]["entries"][1],
        json!({"key":8,"value":false})
    );
    let blue = json!({"kind":"member","enum":"Color","name":"Blue"});
    assert_eq!(ordinary["member"], blue);
    assert_eq!(
        ordinary["dictionary"]["entries"][0]["value"],
        json!([2, blue])
    );
    assert_eq!(ordinary["keyed"]["indexing"], "explicit");
    assert_eq!(
        ordinary["keyed"]["axes"][0],
        json!([
            {"kind":"integer","value":5}, {"kind":"integer","value":2}, {"kind":"integer","value":9}
        ])
    );
    assert_eq!(
        ordinary["keyed"]["axes"][1],
        json!([
            blue, {"kind":"member","enum":"Color","name":"Red"}
        ])
    );
    let cells = ordinary["keyed"]["entries"].as_array().unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(
        cells[0],
        json!({"key":[{"kind":"integer","value":5},blue],"value":{"kind":"absent"}})
    );
    assert_eq!(cells[1]["key"][0]["value"], 2);
    assert_eq!(
        cells[1]["value"],
        json!({"cell":{"kind":"integer","value":20}})
    );

    let tagged = convert(&value, JsonRepresentation::Tagged);
    assert_eq!(
        tagged["z"],
        json!({"kind":"tuple","elements":[
            {"kind":"absent"}, {"kind":"boolean","value":true},
            {"kind":"integer","value":-4}, {"kind":"float","value":1.25},
            {"kind":"string","value":"quote\"\n\\\tλ"}
        ]})
    );
    assert_eq!(
        tagged["a"],
        json!({"kind":"set","elements":[
            {"kind":"integer","value":9}, {"kind":"integer","value":1}
        ]})
    );
    assert_eq!(tagged["dictionary"]["entries"][0]["key"]["kind"], "tuple");
    assert_eq!(
        tagged["dictionary"]["entries"][0]["key"]["elements"][0]["entries"][0],
        json!({"key":{"kind":"integer","value":3},"value":{"kind":"string","value":"nested"}})
    );
    assert_eq!(tagged["keyed"], ordinary["keyed"]);
    let positional = &tagged["dictionary"]["entries"][0]["value"];
    assert_eq!(positional["indexing"], "positional");
    assert_eq!(
        positional["axes"],
        json!([[{"kind":"integer","value":1},{"kind":"integer","value":2}]])
    );
    assert_eq!(
        positional["entries"][1],
        json!({"key":[{"kind":"integer","value":2}],"value":blue})
    );

    for (representation, expected) in [
        (JsonRepresentation::Ordinary, ordinary),
        (JsonRepresentation::Tagged, tagged),
    ] {
        let compact = render_json(
            &value,
            representation,
            JsonLayout::Compact,
            &mut Budget::for_program(limits()),
            Location::Source(4..9),
        )
        .unwrap();
        let pretty = render_json(
            &value,
            representation,
            JsonLayout::Pretty,
            &mut Budget::for_program(limits()),
            Location::Source(4..9),
        )
        .unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&compact).unwrap(), expected);
        assert_eq!(serde_json::from_slice::<Value>(&pretty).unwrap(), expected);
        assert!(compact.starts_with(br#"{"z":"#));
        assert!(pretty.starts_with(b"{\n  \"z\":"));
        assert!(!compact.ends_with(b"\n"));
    }
}

#[test]
fn empty_shapes_and_host_envelopes_keep_their_representation() {
    let value = V::KeyedArray {
        axes: vec![KeyAxis {
            key_type: ValueKeyType::Integer,
            keys: vec![],
        }],
        entries: vec![],
    };
    let data = convert(&value, JsonRepresentation::Ordinary);
    assert_eq!(
        data,
        json!({"kind":"array","indexing":"explicit","axes":[[]],"entries":[]})
    );
    assert_eq!(
        convert(&V::Array(vec![]), JsonRepresentation::Tagged),
        json!({"kind":"array","indexing":"positional","axes":[[]],"entries":[]})
    );
    let envelope = json!({"complete":false,"data":data});
    let bytes = render_json_value(
        &envelope,
        JsonLayout::Pretty,
        &mut Budget::for_program(limits()),
        Location::Source(1..3),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), envelope);
    assert!(bytes.starts_with(b"{\n  \"complete\":"));
}

#[test]
fn conversion_and_escaped_output_fail_without_returning_partial_buffers() {
    let location = Location::Source(12..18);
    let output_limits = ProgramLimits {
        work: 400,
        ..limits()
    };
    render_json_value(
        &Value::String("x".repeat(100)),
        JsonLayout::Compact,
        &mut Budget::for_program(output_limits),
        location.clone(),
    )
    .unwrap();
    let error = render_json_value(
        &Value::String("\u{0001}".repeat(100)),
        JsonLayout::Compact,
        &mut Budget::for_program(output_limits),
        location.clone(),
    )
    .unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::WorkLimit);
    assert_eq!(error.location, location);

    let error = to_json_value(
        &V::Array(vec![V::Integer(1), V::Integer(2)]),
        JsonRepresentation::Tagged,
        &mut Budget::for_program(ProgramLimits {
            results: 8,
            ..limits()
        }),
        location.clone(),
    )
    .unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::ResultLimit);
    assert_eq!(error.location, location);
    let error = render_json_value(
        &json!("longer than the output allowance"),
        JsonLayout::Pretty,
        &mut Budget::for_program(ProgramLimits {
            results: 4,
            ..limits()
        }),
        location.clone(),
    )
    .unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::ResultLimit);
    assert_eq!(error.location, location);
}

#[test]
fn active_nesting_and_malformed_values_preserve_locations() {
    let location = Location::Source(20..29);
    let mut budget = Budget::for_program(ProgramLimits {
        nesting: 4,
        ..limits()
    });
    let error = budget
        .within_evaluation(3, location.clone(), |budget| {
            render_json_value(&json!([1]), JsonLayout::Compact, budget, location.clone())
        })
        .unwrap_err();
    assert!(error.message.contains("nesting"));
    assert_eq!(error.location, location);
    render_json_value(
        &json!(null),
        JsonLayout::Compact,
        &mut budget,
        location.clone(),
    )
    .unwrap();

    for (value, message) in [
        (V::Float(f64::INFINITY), "nonfinite"),
        (
            V::Record(vec![
                ("x".into(), V::Integer(1)),
                ("x".into(), V::Integer(2)),
            ]),
            "duplicate record",
        ),
        (
            V::KeyedArray {
                axes: vec![KeyAxis {
                    key_type: ValueKeyType::Integer,
                    keys: vec![ValueKey::Integer(1)],
                }],
                entries: vec![KeyedEntry {
                    key: vec![ValueKey::Integer(2)],
                    value: V::Absent,
                }],
            },
            "established axis",
        ),
    ] {
        let error = render_json(
            &value,
            JsonRepresentation::Ordinary,
            JsonLayout::Compact,
            &mut Budget::for_program(limits()),
            location.clone(),
        )
        .unwrap_err();
        assert!(error.message.contains(message), "{}", error.message);
        assert_eq!(error.category, QueryErrorCategory::Other);
        assert_eq!(error.location, location);
    }
}
