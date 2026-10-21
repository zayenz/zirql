use zirql::{
    Budget, Located, PatternKind, PredicateArgument, ProgramLimits, QueryErrorCategory,
    ValueExpression as E, ValueExpressionKind as K, ValueGenerator as G, for_each_value_child,
    map_value_payloads, walk_value, walk_value_mut,
};

fn native(id: usize) -> E<Box<usize>> {
    Located::new(K::NativeValue(Box::new(Box::new(id))), id..id + 1)
}
fn budget(work: usize) -> Budget {
    Budget::for_program(ProgramLimits {
        work,
        results: 1000,
        nesting: 64,
        stream_items: 100,
    })
}

#[test]
fn mapping_and_walks_preserve_written_structure_and_native_owners() {
    let value = Located::new(
        K::KeyedComprehension {
            declared_axis_sources: Some(vec![native(0)]),
            clauses: vec![Located::new(
                G::In {
                    case: true,
                    binders: vec![Located::new(PatternKind::Wildcard, 20..21)],
                    source: Located::new(
                        K::Map {
                            array: Box::new(native(1)),
                            argument: PredicateArgument {
                                argument: None,
                                body: Box::new(native(2)),
                            },
                        },
                        1..3,
                    ),
                    filter: Some(native(3)),
                },
                20..30,
            )],
            key: Box::new(native(4)),
            head: Box::new(Located::new(
                K::With {
                    record: Box::new(native(5)),
                    fields: vec![(
                        Located::new("window".into(), 40..46),
                        Located::new(
                            K::Windows {
                                items: Box::new(native(6)),
                                size: Box::new(native(7)),
                            },
                            6..8,
                        ),
                    )],
                },
                5..8,
            )),
            empty_axis_sources: Some(vec![native(8)]),
        },
        0..50,
    );
    let mut owners = Vec::new();
    let value = map_value_payloads(value, |owner| {
        owners.push(std::ptr::from_ref(owner.as_ref()));
        owner
    });
    let mut visited = Vec::new();
    walk_value(&value, &mut budget(1000), 64, |node, _, _| {
        if let K::NativeValue(owner) = &node.value {
            assert_eq!(node.range, ***owner..***owner + 1);
            assert_eq!(
                owners[***owner],
                std::ptr::from_ref(owner.as_ref().as_ref())
            );
            visited.push(***owner);
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(visited, (0..9).collect::<Vec<_>>());
    let K::KeyedComprehension {
        clauses,
        empty_axis_sources,
        ..
    } = &value.value
    else {
        panic!()
    };
    assert!(
        matches!(&clauses[0].value, G::In { case: true, binders, .. } if binders[0].range == (20..21))
    );
    assert_eq!(clauses[0].range, 20..30);
    assert_eq!(empty_axis_sources.as_ref().unwrap()[0].range, 8..9);
    let mut value = value;
    walk_value_mut(&mut value, &mut budget(1000), 64, |node, _, _| {
        if let K::NativeValue(owner) = &mut node.value {
            ***owner += 10;
        }
        Ok(())
    })
    .unwrap();
    let mut direct_ranges = Vec::new();
    for_each_value_child(&value, |node| direct_ranges.push(node.range.clone()));
    assert_eq!(direct_ranges, vec![0..1, 1..3, 3..4, 4..5, 5..8, 8..9]);
}

#[test]
fn traversal_refusals_keep_the_expression_location() {
    let mut value: E<()> = Located::new(K::Array(vec![Located::new(K::CurrentInput, 7..8)]), 3..9);
    let error = walk_value(&value, &mut budget(1), 64, |_, _, _| Ok(())).unwrap_err();
    assert_eq!(error.category, QueryErrorCategory::WorkLimit);
    assert_eq!(error.location, zirql::Location::Query(3..9));
    let error = walk_value_mut(&mut value, &mut budget(100), 1, |_, _, _| Ok(())).unwrap_err();
    assert_eq!(error.location, zirql::Location::Query(7..8));
}
