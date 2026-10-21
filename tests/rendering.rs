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

fn markdown(value: &V) -> String {
    String::from_utf8(
        zirql::render_markdown(
            value,
            &mut Budget::for_program(limits()),
            Location::Query(2..8),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn markdown_record_rows_use_first_seen_columns_and_blank_missing_cells() {
    let rows = V::Array(vec![
        V::Record(vec![
            ("z|".into(), V::String("<tag>&\r\nnext".into())),
            ("a".into(), V::Absent),
        ]),
        V::Record(vec![
            ("a".into(), V::Integer(2)),
            ("nested".into(), V::Array(vec![V::Integer(3)])),
        ]),
    ]);
    assert_eq!(
        markdown(&rows),
        "\n| z\\| | a | nested |\n| --- | --- | --- |\n| &lt;tag&gt;&amp;<br>next |  |  |\n|  | 2 | \\[3\\] |\n\n"
    );
    assert_eq!(
        markdown(&V::Array(vec![V::Record(vec![]), V::Record(vec![])])),
        "\n| Value |\n| --- |\n| {} |\n| {} |\n\n"
    );
}

#[test]
fn markdown_maps_keep_keyed_rows_and_shared_json_for_structural_cells() {
    let rows = V::Dictionary(
        vec![
            (
                V::Tuple(vec![V::Integer(2), V::String("key".into())]),
                V::Record(vec![("count".into(), V::Integer(4))]),
            ),
            (
                V::Integer(9),
                V::Record(vec![("note".into(), V::String("`*_[]~\\".into()))]),
            ),
        ]
        .into(),
    );
    assert_eq!(
        markdown(&rows),
        "\n| Key | count | note |\n| --- | --- | --- |\n| \\[2,\"key\"\\] | 4 |  |\n| 9 |  | \\`\\*\\_\\[\\]\\~\\\\ |\n\n"
    );
    let mut fields = match fixture() {
        V::Record(fields) => fields,
        _ => unreachable!(),
    };
    let keyed = fields.pop().unwrap();
    let member = fields.pop().unwrap();
    let rendered = markdown(&V::Record(vec![member, keyed]));
    assert!(rendered.starts_with("\n| Key | Value |\n| --- | --- |\n| member | {\"kind\":\"member\",\"enum\":\"Color\",\"name\":\"Blue\"} |\n"));
    assert!(rendered.contains("\"indexing\":\"explicit\""));
    assert!(rendered.contains("{\"kind\":\"integer\",\"value\":9}"));
    assert_eq!(
        markdown(&V::Record(vec![("first".into(), V::Record(vec![]))])),
        "\n| Key |\n| --- |\n| first |\n\n"
    );
}

#[test]
fn markdown_scalars_ordered_values_and_empty_results_have_explicit_layouts() {
    assert_eq!(markdown(&V::Integer(3)), "\n3\n\n");
    assert_eq!(
        markdown(&V::String(
            "    # heading\r\n- item\n+ item\n1. item\n2) item\n---\n===\n> quote\n```code".into()
        )),
        "\n&#32;&#32;&#32;&#32;\\# heading<br>\\- item<br>\\+ item<br>1\\. item<br>2\\) item<br>\\-\\-\\-<br>\\=\\=\\=<br>&gt; quote<br>\\`\\`\\`code\n\n"
    );
    assert_eq!(
        markdown(&V::Set {
            element_type: Some(SetElementType::Integer),
            elements: vec![V::Integer(9), V::Integer(1)].into(),
        }),
        "\n| Value |\n| --- |\n| 9 |\n| 1 |\n\n"
    );
    assert_eq!(markdown(&V::Array(vec![])), "\n_No entries._\n\n");
    assert_eq!(markdown(&V::Record(vec![])), "\n_No entries._\n\n");
}

#[test]
fn markdown_checks_sparse_rectangle_and_escaped_bytes_before_growth() {
    let location = Location::Source(12..18);
    let sparse = V::Array(
        (0..12)
            .map(|index| V::Record(vec![(format!("column{index}"), V::Integer(index))]))
            .collect(),
    );
    let error = zirql::render_markdown(
        &sparse,
        &mut Budget::for_program(limits()),
        location.clone(),
    )
    .unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::ItemLimit);
    assert_eq!(error.location, location);

    let output_limits = ProgramLimits {
        work: 400,
        ..limits()
    };
    zirql::render_markdown(
        &V::Array(vec![V::String("x".repeat(100))]),
        &mut Budget::for_program(output_limits),
        location.clone(),
    )
    .unwrap();
    let error = zirql::render_markdown(
        &V::Array(vec![V::String("&".repeat(100))]),
        &mut Budget::for_program(output_limits),
        location.clone(),
    )
    .unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::WorkLimit);
    assert_eq!(error.location, location);
    let error = zirql::render_markdown(
        &V::Array(vec![V::String("x".repeat(100))]),
        &mut Budget::for_program(ProgramLimits {
            results: 50,
            ..limits()
        }),
        location.clone(),
    )
    .unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::ResultLimit);
    assert_eq!(error.location, location);
    let error = zirql::render_markdown(
        &V::Array(vec![V::Integer(1), V::Float(f64::INFINITY)]),
        &mut Budget::for_program(limits()),
        location.clone(),
    )
    .unwrap_err();
    assert!(error.message.contains("nonfinite"));
    assert_eq!(error.location, location);
}

#[test]
fn mixed_reports_keep_constructed_policy_native_projections_and_located_refusals() {
    use zirql::{EvaluationError, NativeCapture, RenderContext, RenderHost, RenderView};
    enum Report {
        Data(V),
        Native,
        Rows(Vec<NativeCapture<Report>>),
        Refused,
    }
    struct Host;
    impl RenderHost for Host {
        type Value = Report;
        type Error = &'static str;
        fn view<'a>(
            &mut self,
            value: &'a Report,
            _context: RenderContext,
            budget: &mut Budget,
            location: &Location,
        ) -> Result<RenderView<'a, Report>, Self::Error> {
            Ok(match value {
                Report::Data(value) => RenderView::Constructed(value, JsonRepresentation::Tagged),
                Report::Native => {
                    budget
                        .charge(13, location.clone())
                        .map_err(|_| "native projection limit")?;
                    budget
                        .charge_results(13, location.clone())
                        .map_err(|_| "native projection limit")?;
                    RenderView::Projected(json!({"name":"native"}))
                }
                Report::Rows(rows) => RenderView::Captures(rows),
                Report::Refused => return Err("computational value"),
            })
        }
    }
    let value = Report::Rows(vec![NativeCapture {
        fields: vec![
            ("z".into(), Report::Native),
            (
                "a".into(),
                Report::Data(V::Tuple(vec![V::Integer(2), V::Absent])),
            ),
        ],
    }]);
    let location = Location::Query(7..19);
    let json = zirql::to_report_json_value(
        &mut Host,
        &value,
        &mut Budget::for_program(limits()),
        location.clone(),
    )
    .unwrap();
    assert_eq!(
        json.to_string(),
        r#"[{"z":{"name":"native"},"a":{"kind":"tuple","elements":[{"kind":"integer","value":2},{"kind":"absent"}]}}]"#
    );
    let bytes = zirql::render_report_markdown(
        &mut Host,
        &value,
        &mut Budget::for_program(limits()),
        location.clone(),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "\n| z | a |\n| --- | --- |\n| {\"name\":\"native\"} | \\[2,null\\] |\n\n"
    );
    assert_eq!(
        zirql::render_report_markdown(
            &mut Host,
            &Report::Data(V::Array(vec![V::Integer(3)])),
            &mut Budget::for_program(limits()),
            location.clone()
        )
        .unwrap(),
        b"\n| Value |\n| --- |\n| 3 |\n\n",
    );
    let error = zirql::render_report_markdown(
        &mut Host,
        &value,
        &mut Budget::for_program(ProgramLimits {
            results: 2,
            ..limits()
        }),
        location.clone(),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.location == location && error.category == QueryErrorCategory::ResultLimit)
    );
    assert!(matches!(
        zirql::render_report_markdown(
            &mut Host,
            &Report::Refused,
            &mut Budget::for_program(limits()),
            location.clone()
        ),
        Err(EvaluationError::Host("computational value"))
    ));
    let error = zirql::to_report_json_value(
        &mut Host,
        &value,
        &mut Budget::for_program(ProgramLimits {
            results: 2,
            ..limits()
        }),
        location.clone(),
    )
    .unwrap_err();
    assert!(
        matches!(error, EvaluationError::Core(error) if error.location == location && error.category == QueryErrorCategory::ResultLimit)
    );
    let error = zirql::to_report_json_value(
        &mut Host,
        &Report::Rows(vec![NativeCapture {
            fields: vec![("queue".into(), Report::Refused)],
        }]),
        &mut Budget::for_program(limits()),
        location,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        EvaluationError::Host("computational value")
    ));
}
