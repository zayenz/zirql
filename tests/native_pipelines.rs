use std::{ops::Range, rc::Rc};

use zirql::{
    ArgumentReader, Budget, EvaluationError, Host, Limits, Located, Location, NativeInput,
    NativeResult, NativeValue, Pipeline, PredicateKind, QueryError, Stage, Truth, evaluate,
    parse_pipeline,
};

fn limits() -> Limits {
    Limits {
        work: 100,
        items: 8,
        nesting: 64,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Selection<'a> {
    text: &'a str,
    range: Range<usize>,
}

#[derive(Debug)]
struct SourceView<'a> {
    source: &'a str,
}

enum SourceLeaf {
    Prefix(Located<String>),
    Yes,
    No,
    Unknown,
    Fail,
}

fn source_leaf(reader: &mut ArgumentReader<'_>) -> Result<SourceLeaf, QueryError> {
    let name = reader.word()?;
    Ok(match name.value {
        "prefix" => {
            reader.expect('(')?;
            let argument = reader.quoted_string()?;
            reader.expect(')')?;
            SourceLeaf::Prefix(argument)
        }
        "yes" => SourceLeaf::Yes,
        "no" => SourceLeaf::No,
        "unknown" => SourceLeaf::Unknown,
        "fail" => SourceLeaf::Fail,
        _ => {
            return Err(QueryError::query(
                name.range,
                "unsupported source predicate",
            ));
        }
    })
}

struct SourceHost;

impl<'a> Host<Selection<'a>, Rc<SourceView<'a>>, SourceLeaf> for SourceHost {
    type Error = QueryError;
    type Key = ();

    fn evaluate_leaf(
        &self,
        leaf: &SourceLeaf,
        item: &Selection<'a>,
        _context: &mut Rc<SourceView<'a>>,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<QueryError>> {
        Ok(match leaf {
            SourceLeaf::Prefix(prefix) if item.text.starts_with(&prefix.value) => Truth::True,
            SourceLeaf::Prefix(_) | SourceLeaf::No => Truth::False,
            SourceLeaf::Yes => Truth::True,
            SourceLeaf::Unknown => Truth::Unknown,
            SourceLeaf::Fail => {
                return Err(EvaluationError::Host(QueryError::source(
                    item.range.clone(),
                    "source leaf failed",
                )));
            }
        })
    }
}

fn source_input<'a>(
    context: Rc<SourceView<'a>>,
    budget: &mut Budget,
) -> Result<NativeInput<Selection<'a>, Rc<SourceView<'a>>>, QueryError> {
    let source = context.source;
    let selections = source.split_whitespace().map(|text| {
        let start = text.as_ptr() as usize - source.as_ptr() as usize;
        Selection {
            text,
            range: start..start + text.len(),
        }
    });
    NativeInput::collect(
        context,
        selections,
        budget,
        Location::Source(0..source.len()),
    )
}

fn source_run<'a>(
    query: &str,
    source: &'a str,
    budget: &mut Budget,
) -> Result<NativeResult<Selection<'a>, Rc<SourceView<'a>>>, EvaluationError<QueryError>> {
    let pipeline = parse_pipeline(query, source_leaf, 64)?;
    let input = source_input(Rc::new(SourceView { source }), budget)?;
    evaluate(&pipeline, &SourceHost, input, budget)
}

#[test]
fn borrowed_source_keeps_byte_ranges_and_context_through_stream_and_count() {
    let source = String::from("βeta alpha apple");
    let context = Rc::new(SourceView { source: &source });
    let mut budget = Budget::new(limits());
    let pipeline = parse_pipeline("filter(prefix(\"a\")) | head(1)", source_leaf, 64).unwrap();
    let input = source_input(Rc::clone(&context), &mut budget).unwrap();
    let result = evaluate(&pipeline, &SourceHost, input, &mut budget).unwrap();
    assert_eq!(
        result.value,
        NativeValue::Stream(vec![Selection {
            text: &source[6..11],
            range: 6..11,
        }])
    );
    assert!(Rc::ptr_eq(&result.context, &context));
    let NativeValue::Stream(items) = result.value else {
        unreachable!()
    };
    assert_eq!(items[0].text.as_ptr(), source[6..11].as_ptr());

    let pipeline =
        parse_pipeline("filter(prefix(\"a\")) | head(1) | count", source_leaf, 64).unwrap();
    let input = source_input(Rc::clone(&context), &mut budget).unwrap();
    let result = evaluate(&pipeline, &SourceHost, input, &mut budget).unwrap();
    assert_eq!(result.value, NativeValue::Count(1));
    assert!(Rc::ptr_eq(&result.context, &context));
}

#[derive(Debug, PartialEq, Eq)]
struct OperationId {
    owner: usize,
    index: usize,
}

#[derive(Debug)]
struct Document {
    owner: usize,
    operations: Vec<&'static str>,
}

impl Document {
    fn resolve(&self, id: &OperationId) -> Option<&'static str> {
        (id.owner == self.owner)
            .then(|| self.operations.get(id.index).copied())
            .flatten()
    }
}

struct DocumentHost;

impl Host<OperationId, Rc<Document>, &'static str> for DocumentHost {
    type Error = QueryError;
    type Key = ();

    fn evaluate_leaf(
        &self,
        leaf: &&'static str,
        item: &OperationId,
        context: &mut Rc<Document>,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<QueryError>> {
        let operation = context
            .resolve(item)
            .ok_or_else(|| QueryError::query(range.clone(), "invalid operation handle"))?;
        Ok(if operation == *leaf {
            Truth::True
        } else {
            Truth::False
        })
    }
}

#[test]
fn document_handles_keep_identity_order_and_resolution_with_typed_lowering() {
    let document = Rc::new(Document {
        owner: 7,
        operations: vec!["add", "mul", "add"],
    });
    let stages = vec![
        Located::new(
            Stage::Filter(Located::new(PredicateKind::Leaf("add"), 7..10)),
            0..11,
        ),
        Located::new(Stage::Head(2), 14..21),
    ];
    let input = |budget: &mut Budget| {
        NativeInput::collect(
            Rc::clone(&document),
            [2, 0, 2, 1].map(|index| OperationId { owner: 7, index }),
            budget,
            Location::Source(0..3),
        )
        .unwrap()
    };
    let mut budget = Budget::new(limits());
    let pipeline = Pipeline::new(stages.clone(), 0..21, 64).unwrap();
    let result = evaluate(&pipeline, &DocumentHost, input(&mut budget), &mut budget).unwrap();
    assert!(Rc::ptr_eq(&result.context, &document));
    let NativeValue::Stream(items) = result.value else {
        panic!("expected native operation handles")
    };
    assert_eq!(
        items,
        [
            OperationId { owner: 7, index: 2 },
            OperationId { owner: 7, index: 0 }
        ]
    );
    assert_eq!(result.context.resolve(&items[0]), Some("add"));

    let mut stages = stages;
    stages.push(Located::new(Stage::Count, 24..29));
    let pipeline = Pipeline::new(stages, 0..29, 64).unwrap();
    let result = evaluate(&pipeline, &DocumentHost, input(&mut budget), &mut budget).unwrap();
    assert_eq!(result.value, NativeValue::Count(2));
    assert!(Rc::ptr_eq(&result.context, &document));
}

#[test]
fn unknown_negation_and_short_circuiting_preserve_host_outcomes() {
    let mut budget = Budget::new(limits());
    assert_eq!(
        source_run("filter(not unknown) | count", "alpha", &mut budget)
            .unwrap()
            .value,
        NativeValue::Count(0)
    );
    assert_eq!(
        source_run(
            "filter(no and fail or yes or fail) | count",
            "alpha",
            &mut budget
        )
        .unwrap()
        .value,
        NativeValue::Count(1)
    );
    assert_eq!(
        source_run("filter(not (unknown and no)) | count", "alpha", &mut budget)
            .unwrap()
            .value,
        NativeValue::Count(1)
    );
    let error = source_run("filter(unknown or fail)", "alpha", &mut budget).unwrap_err();
    assert_eq!(
        error,
        EvaluationError::Host(QueryError::source(0..5, "source leaf failed"))
    );
}

#[test]
fn malformed_arguments_and_scalar_transitions_keep_query_locations() {
    let query = "filter(prefix(\"λ\") and prefix(42))";
    let error = parse_pipeline(query, source_leaf, 64).err().unwrap();
    let argument_start = query.find("42").unwrap();
    assert_eq!(
        error.location,
        Location::Query(argument_start..argument_start + 1)
    );

    let query = "count | filter(fail)";
    let error = parse_pipeline(query, source_leaf, 64).err().unwrap();
    assert_eq!(
        error,
        QueryError::query(8..20, "a stage cannot follow count")
    );

    let error = Pipeline::<SourceLeaf>::new(
        vec![
            Located::new(Stage::Tally, 0..5),
            Located::new(Stage::Head(1), 8..15),
        ],
        0..15,
        64,
    )
    .err()
    .unwrap();
    assert_eq!(
        error,
        QueryError::query(8..15, "a stage cannot follow tally")
    );
}

#[test]
fn work_items_and_nesting_refuse_at_the_active_location_without_resetting_budget() {
    let mut budget = Budget::new(Limits {
        items: 1,
        ..limits()
    });
    let error = source_input(
        Rc::new(SourceView {
            source: "alpha beta",
        }),
        &mut budget,
    )
    .unwrap_err();
    assert_eq!(error, QueryError::source(0..10, "item limit exceeded"));

    let mut budget = Budget::new(Limits {
        work: 3,
        ..limits()
    });
    assert_eq!(
        source_run("count", "alpha", &mut budget).unwrap().value,
        NativeValue::Count(1)
    );
    let error = source_run("count", "beta", &mut budget).unwrap_err();
    assert_eq!(
        error,
        EvaluationError::Core(QueryError::query(0..5, "work limit exceeded"))
    );

    let error = parse_pipeline("filter(not yes)", source_leaf, 2)
        .err()
        .unwrap();
    assert_eq!(error, QueryError::query(11..11, "nesting limit exceeded"));
    let mut budget = Budget::new(Limits {
        nesting: 1,
        ..limits()
    });
    let error = source_run("filter(yes)", "alpha", &mut budget).unwrap_err();
    assert_eq!(
        error,
        EvaluationError::Core(QueryError::query(7..10, "nesting limit exceeded"))
    );
    assert_eq!(
        source_run("count", "alpha", &mut budget).unwrap().value,
        NativeValue::Count(1)
    );
}

struct NestedHost {
    pipeline: Pipeline<()>,
}

impl Host<usize, (), ()> for NestedHost {
    type Error = QueryError;
    type Key = ();

    fn evaluate_leaf(
        &self,
        _leaf: &(),
        item: &usize,
        _context: &mut (),
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<QueryError>> {
        let input = NativeInput::collect((), [*item], budget, Location::Query(range.clone()))?;
        evaluate(&self.pipeline, self, input, budget)?;
        Ok(Truth::True)
    }
}

#[test]
fn nested_host_evaluation_shares_work_and_active_depth() {
    let host = NestedHost {
        pipeline: Pipeline::new(vec![Located::new(Stage::Count, 0..5)], 0..5, 64).unwrap(),
    };
    let pipeline = Pipeline::new(
        vec![Located::new(
            Stage::Filter(Located::new(PredicateKind::Leaf(()), 7..13)),
            0..14,
        )],
        0..14,
        64,
    )
    .unwrap();
    let mut budget = Budget::new(Limits {
        nesting: 2,
        ..limits()
    });
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    let error = evaluate(&pipeline, &host, input, &mut budget).unwrap_err();
    assert_eq!(
        error,
        EvaluationError::Core(QueryError::query(0..5, "nesting limit exceeded"))
    );

    let mut budget = Budget::new(Limits {
        work: 5,
        ..limits()
    });
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    let error = evaluate(&pipeline, &host, input, &mut budget).unwrap_err();
    assert_eq!(
        error,
        EvaluationError::Core(QueryError::query(0..5, "work limit exceeded"))
    );
}

#[derive(Debug, PartialEq, Eq)]
enum Row<'a> {
    Node(Selection<'a>),
    String(String),
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Key {
    Node(usize),
    String(String),
}

#[derive(Debug)]
struct Context<'a> {
    source: &'a str,
    model: Rc<str>,
    strings: bool,
    document: bool,
    prepared_name: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
enum NativeFailure {
    Kind(Range<usize>),
    Input(Range<usize>),
    Model { file: Rc<str>, range: Range<usize> },
}

enum NativeStage {
    Names,
    FailModel,
}

enum NativeLeaf {
    Prefix,
    FailInput,
}

struct NativeHost;

impl<'a> Host<Row<'a>, Context<'a>, NativeLeaf, NativeStage> for NativeHost {
    type Error = NativeFailure;
    type Key = Key;

    fn prepare_stage(
        &self,
        stage: &Located<Stage<NativeLeaf, NativeStage>>,
        context: &mut Context<'a>,
        _budget: &mut Budget,
    ) -> Result<(), EvaluationError<NativeFailure>> {
        let applicable = match stage.value {
            Stage::Filter(_) | Stage::Host(NativeStage::Names) => !context.strings,
            Stage::Tally => context.strings,
            _ => true,
        };
        if !applicable {
            return Err(EvaluationError::Host(NativeFailure::Kind(
                stage.range.clone(),
            )));
        }
        if matches!(stage.value, Stage::Filter(_)) {
            context.document = false;
        }
        Ok(())
    }

    fn prepare_filter_item(
        &self,
        item: &Row<'a>,
        context: &mut Context<'a>,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<(), EvaluationError<NativeFailure>> {
        let Row::Node(node) = item else {
            unreachable!()
        };
        // This host's input admission owns the row visit. Native preparation
        // supplies the name used by every leaf in this row's Boolean tree.
        context.prepared_name = Some(node.text);
        Ok(())
    }

    fn evaluate_leaf(
        &self,
        leaf: &NativeLeaf,
        item: &Row<'a>,
        context: &mut Context<'a>,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, EvaluationError<NativeFailure>> {
        let Row::Node(node) = item else {
            unreachable!()
        };
        match leaf {
            NativeLeaf::Prefix => Ok(if context.prepared_name.unwrap().starts_with('a') {
                Truth::True
            } else {
                Truth::False
            }),
            NativeLeaf::FailInput => Err(EvaluationError::Host(NativeFailure::Input(
                node.range.clone(),
            ))),
        }
    }

    fn evaluate_stage(
        &self,
        stage: &NativeStage,
        items: Vec<Row<'a>>,
        context: &mut Context<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Vec<Row<'a>>, EvaluationError<NativeFailure>> {
        if matches!(stage, NativeStage::FailModel) {
            return Err(EvaluationError::Host(NativeFailure::Model {
                file: Rc::clone(&context.model),
                range: 42..47,
            }));
        }
        budget.check_items(items.len(), Location::Query(range.clone()))?;
        let mut names = Vec::new();
        for item in items {
            let Row::Node(node) = item else {
                unreachable!()
            };
            budget.charge(node.text.len().max(1), Location::Query(range.clone()))?;
            names.push(Row::String(node.text.to_owned()));
        }
        context.strings = true;
        context.document = false;
        Ok(names)
    }

    fn identity_key(
        &self,
        item: &Row<'a>,
        _context: &mut Context<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Key, EvaluationError<NativeFailure>> {
        Ok(match item {
            Row::Node(node) => Key::Node(node.range.start),
            Row::String(text) => {
                budget.charge(text.len().saturating_sub(1), Location::Query(range.clone()))?;
                Key::String(text.clone())
            }
        })
    }

    fn tally_key(
        &self,
        item: Row<'a>,
        _context: &mut Context<'a>,
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<String, EvaluationError<NativeFailure>> {
        let Row::String(text) = item else {
            unreachable!()
        };
        budget.charge(text.len().saturating_sub(1), Location::Query(range.clone()))?;
        Ok(text)
    }
}

fn native_input<'a>(source: &'a str, budget: &mut Budget) -> NativeInput<Row<'a>, Context<'a>> {
    let context = Context {
        source,
        model: Rc::from("model.mzn"),
        strings: false,
        document: true,
        prepared_name: None,
    };
    let words: Vec<_> = source.split_whitespace().collect();
    NativeInput::collect(
        context,
        [0, 1, 0, 2, 1].map(|index| {
            let text = words[index];
            let start = text.as_ptr() as usize - source.as_ptr() as usize;
            Row::Node(Selection {
                text,
                range: start..start + text.len(),
            })
        }),
        budget,
        Location::Source(0..source.len()),
    )
    .unwrap()
}

#[test]
fn native_stages_unique_and_tally_retain_rows_and_mutable_context() {
    let source = String::from("beta alpha apple");
    let pipeline = Pipeline::new(vec![Located::new(Stage::Unique, 0..6)], 0..6, 64).unwrap();
    let mut budget = Budget::new(limits());
    let result = evaluate(
        &pipeline,
        &NativeHost,
        native_input(&source, &mut budget),
        &mut budget,
    )
    .unwrap();
    let NativeValue::Stream(rows) = result.value else {
        panic!("expected nodes")
    };
    assert_eq!(
        rows,
        [
            Row::Node(Selection {
                text: &source[0..4],
                range: 0..4
            }),
            Row::Node(Selection {
                text: &source[5..10],
                range: 5..10
            }),
            Row::Node(Selection {
                text: &source[11..16],
                range: 11..16
            }),
        ]
    );
    assert!(result.context.document);
    assert_eq!(result.context.source.as_ptr(), source.as_ptr());

    let pipeline = Pipeline::new(
        vec![
            Located::new(Stage::Unique, 0..6),
            Located::new(Stage::Head(2), 9..16),
            Located::new(Stage::Count, 19..24),
        ],
        0..24,
        64,
    )
    .unwrap();
    let mut budget = Budget::new(limits());
    let result = evaluate(
        &pipeline,
        &NativeHost,
        native_input(&source, &mut budget),
        &mut budget,
    )
    .unwrap();
    assert_eq!(result.value, NativeValue::Count(2));
    assert!(result.context.document);

    let pipeline = Pipeline::new(
        vec![
            Located::new(Stage::Host(NativeStage::Names), 0..5),
            Located::new(Stage::Tally, 8..13),
        ],
        0..13,
        64,
    )
    .unwrap();
    let mut budget = Budget::new(limits());
    let result = evaluate(
        &pipeline,
        &NativeHost,
        native_input(&source, &mut budget),
        &mut budget,
    )
    .unwrap();
    let NativeValue::Tally(counts) = result.value else {
        panic!("expected tally")
    };
    assert_eq!(
        counts.into_iter().collect::<Vec<_>>(),
        [("alpha".into(), 2), ("apple".into(), 1), ("beta".into(), 2),]
    );
    assert!(result.context.strings);
    assert!(!result.context.document);
    assert_eq!(result.context.source.as_ptr(), source.as_ptr());

    let pipeline = Pipeline::new(
        vec![
            Located::new(
                Stage::Filter(Located::new(PredicateKind::Leaf(NativeLeaf::Prefix), 7..13)),
                0..25,
            ),
            Located::new(Stage::Count, 28..33),
        ],
        0..33,
        64,
    )
    .unwrap();
    // Admission + stages + visited predicate nodes. Native row preparation
    // replaces the default charge because this host already admitted the rows.
    let mut budget = Budget::new(Limits {
        work: 12,
        ..limits()
    });
    let result = evaluate(
        &pipeline,
        &NativeHost,
        native_input(&source, &mut budget),
        &mut budget,
    )
    .unwrap();
    assert_eq!(result.value, NativeValue::Count(3));
    assert!(!result.context.document);
}

#[test]
fn empty_stream_applicability_and_native_source_errors_survive_evaluation() {
    let mut budget = Budget::new(limits());
    let context = Context {
        source: "",
        model: Rc::from("model.mzn"),
        strings: false,
        document: true,
        prepared_name: None,
    };
    let input = NativeInput::collect(context, [], &mut budget, Location::Source(0..0)).unwrap();
    let pipeline = Pipeline::new(vec![Located::new(Stage::Tally, 0..5)], 0..5, 64).unwrap();
    assert_eq!(
        evaluate(&pipeline, &NativeHost, input, &mut budget).unwrap_err(),
        EvaluationError::Host(NativeFailure::Kind(0..5))
    );

    let pipeline = Pipeline::new(
        vec![Located::new(
            Stage::Filter(Located::new(
                PredicateKind::Leaf(NativeLeaf::FailInput),
                7..11,
            )),
            0..12,
        )],
        0..12,
        64,
    )
    .unwrap();
    let error = evaluate(
        &pipeline,
        &NativeHost,
        native_input("beta alpha apple", &mut budget),
        &mut budget,
    )
    .unwrap_err();
    assert_eq!(error, EvaluationError::Host(NativeFailure::Input(0..4)));

    let model = Rc::from("included/model.mzn");
    let context = Context {
        source: "",
        model: Rc::clone(&model),
        strings: false,
        document: true,
        prepared_name: None,
    };
    let input = NativeInput::collect(context, [], &mut budget, Location::Source(0..0)).unwrap();
    let pipeline = Pipeline::new(
        vec![Located::new(Stage::Host(NativeStage::FailModel), 0..10)],
        0..10,
        64,
    )
    .unwrap();
    let EvaluationError::Host(NativeFailure::Model { file, range }) =
        evaluate(&pipeline, &NativeHost, input, &mut budget).unwrap_err()
    else {
        panic!("expected native model error")
    };
    assert!(Rc::ptr_eq(&file, &model));
    assert_eq!(range, 42..47);
}

#[test]
fn admitted_input_and_nested_scans_share_one_work_allowance() {
    let mut budget = Budget::new(Limits {
        work: 4,
        ..limits()
    });
    budget.charge(3, Location::Source(0..3)).unwrap();
    let input =
        NativeInput::from_admitted((), vec![1, 2, 3], &budget, Location::Source(0..3)).unwrap();
    let host = NestedHost {
        pipeline: Pipeline::new(vec![Located::new(Stage::Count, 0..5)], 0..5, 64).unwrap(),
    };
    let result = evaluate(&host.pipeline, &host, input, &mut budget).unwrap();
    assert_eq!(result.value, NativeValue::Count(3));

    let mut budget = Budget::new(Limits {
        work: 5,
        ..limits()
    });
    budget.charge(2, Location::Source(0..2)).unwrap();
    let input = NativeInput::from_admitted((), vec![9], &budget, Location::Source(0..1)).unwrap();
    let pipeline = Pipeline::new(
        vec![Located::new(
            Stage::Filter(Located::new(PredicateKind::Leaf(()), 7..13)),
            0..14,
        )],
        0..14,
        64,
    )
    .unwrap();
    // Nested input admission needs a fourth unit after the outer scan. It cannot
    // obtain a fresh allowance when the host opens its nested count.
    assert_eq!(
        evaluate(&pipeline, &host, input, &mut budget).unwrap_err(),
        EvaluationError::Core(QueryError::query(7..13, "work limit exceeded"))
    );
}

fn written_wrappers(count: usize) -> zirql::Predicate<SourceLeaf> {
    let mut predicate = Located::new(PredicateKind::Leaf(SourceLeaf::Yes), 100..103);
    for index in 0..count {
        let child = Box::new(predicate);
        predicate = Located::new(
            if index % 2 == 0 {
                PredicateKind::Group(child)
            } else {
                PredicateKind::Not(child)
            },
            index..index + 1,
        );
        // Zincite lowers flat precedence vectors around each written wrapper.
        predicate = Located::new(
            PredicateKind::Or(vec![
                Located::new(
                    PredicateKind::And(vec![
                        predicate,
                        Located::new(PredicateKind::Leaf(SourceLeaf::Yes), 100..103),
                    ]),
                    0..103,
                ),
                Located::new(PredicateKind::Leaf(SourceLeaf::No), 104..106),
            ]),
            0..106,
        );
    }
    predicate
}

#[test]
fn group_not_lowering_preserves_written_limits_and_nested_safety() {
    let mut budget = Budget::new(Limits {
        nesting: 0,
        ..limits()
    });
    let pipeline = Pipeline::with_group_not_nesting(
        vec![
            Located::new(
                Stage::Filter(Located::new(
                    PredicateKind::And(vec![
                        Located::new(PredicateKind::Leaf(SourceLeaf::Yes), 7..10),
                        Located::new(PredicateKind::Leaf(SourceLeaf::Yes), 15..18),
                    ]),
                    7..18,
                )),
                0..19,
            ),
            Located::new(Stage::Count, 22..27),
        ],
        0..27,
        0,
    )
    .unwrap();
    let input = source_input(Rc::new(SourceView { source: "alpha" }), &mut budget).unwrap();
    assert_eq!(
        evaluate(&pipeline, &SourceHost, input, &mut budget)
            .unwrap()
            .value,
        NativeValue::Count(1)
    );

    let pipeline = Pipeline::with_group_not_nesting(
        vec![Located::new(Stage::Filter(written_wrappers(64)), 0..106)],
        0..106,
        64,
    )
    .unwrap();
    let mut budget = Budget::new(Limits {
        work: 1000,
        ..limits()
    });
    let input = source_input(Rc::new(SourceView { source: "alpha" }), &mut budget).unwrap();
    assert!(
        matches!(evaluate(&pipeline, &SourceHost, input, &mut budget).unwrap().value, NativeValue::Stream(rows) if rows.len() == 1)
    );
    let error = Pipeline::<SourceLeaf>::with_group_not_nesting(
        vec![Located::new(Stage::Filter(written_wrappers(65)), 0..106)],
        0..106,
        64,
    )
    .err()
    .unwrap();
    assert_eq!(error, QueryError::query(0..1, "nesting limit exceeded"));

    let predicate = Located::new(
        PredicateKind::Group(Box::new(Located::new(PredicateKind::Leaf(()), 7..8))),
        6..9,
    );
    let host = NestedHost {
        pipeline: Pipeline::with_group_not_nesting(
            vec![Located::new(Stage::Filter(predicate), 0..10)],
            0..10,
            1,
        )
        .unwrap(),
    };
    let mut budget = Budget::new(Limits {
        nesting: 1,
        ..limits()
    });
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    assert_eq!(
        evaluate(&host.pipeline, &host, input, &mut budget).unwrap_err(),
        EvaluationError::Core(QueryError::query(6..9, "nesting limit exceeded"))
    );
    let count =
        Pipeline::with_group_not_nesting(vec![Located::new(Stage::Count, 0..5)], 0..5, 0).unwrap();
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    assert_eq!(
        evaluate(&count, &host, input, &mut budget).unwrap().value,
        NativeValue::Count(1)
    );

    let host = NestedHost {
        pipeline: Pipeline::with_group_not_nesting(
            vec![Located::new(
                Stage::Filter(Located::new(PredicateKind::Leaf(()), 7..8)),
                0..9,
            )],
            0..9,
            0,
        )
        .unwrap(),
    };
    let mut budget = Budget::new(Limits {
        nesting: 0,
        work: 1000,
        ..limits()
    });
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    assert_eq!(
        evaluate(&host.pipeline, &host, input, &mut budget).unwrap_err(),
        EvaluationError::Core(QueryError::query(
            0..9,
            "nested evaluation call limit exceeded"
        ))
    );
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    assert_eq!(
        evaluate(&count, &host, input, &mut budget).unwrap().value,
        NativeValue::Count(1)
    );
}

#[test]
fn weighted_stages_and_zero_collection_row_work_share_the_allowance() {
    struct WeightedHost;

    impl Host<&str, (), ()> for WeightedHost {
        type Error = QueryError;
        type Key = String;

        fn stage_work(&self, items: &[&str], _context: &()) -> usize {
            items.len().max(1)
        }

        fn collection_item_work(&self) -> usize {
            0
        }

        fn prepare_stage(
            &self,
            _stage: &Located<Stage<()>>,
            _context: &mut (),
            _budget: &mut Budget,
        ) -> Result<(), EvaluationError<QueryError>> {
            Ok(())
        }

        fn evaluate_leaf(
            &self,
            _leaf: &(),
            _item: &&str,
            _context: &mut (),
            _budget: &mut Budget,
            _range: &Range<usize>,
        ) -> Result<Truth, EvaluationError<QueryError>> {
            Ok(Truth::True)
        }

        fn identity_key(
            &self,
            item: &&str,
            _context: &mut (),
            _budget: &mut Budget,
            _range: &Range<usize>,
        ) -> Result<String, EvaluationError<QueryError>> {
            Ok((*item).into())
        }

        fn tally_key(
            &self,
            item: &str,
            _context: &mut (),
            _budget: &mut Budget,
            _range: &Range<usize>,
        ) -> Result<String, EvaluationError<QueryError>> {
            Ok(item.into())
        }
    }

    let pipeline = Pipeline::new(
        vec![
            Located::new(Stage::Unique, 0..6),
            Located::new(Stage::Tally, 9..14),
        ],
        0..14,
        64,
    )
    .unwrap();
    let input = |budget: &mut Budget| {
        NativeInput::collect(
            (),
            ["beta", "alpha", "beta", "apple"],
            budget,
            Location::Source(0..21),
        )
        .unwrap()
    };
    // Four admitted rows, four unique inputs and three tally inputs.
    let mut budget = Budget::new(Limits {
        work: 11,
        ..limits()
    });
    let result = evaluate(&pipeline, &WeightedHost, input(&mut budget), &mut budget).unwrap();
    assert_eq!(
        result.value,
        NativeValue::Tally([("alpha".into(), 1), ("apple".into(), 1), ("beta".into(), 1)].into())
    );
    assert_eq!(budget.remaining_work(), 0);

    let mut budget = Budget::new(Limits {
        work: 10,
        ..limits()
    });
    assert_eq!(
        evaluate(&pipeline, &WeightedHost, input(&mut budget), &mut budget).unwrap_err(),
        EvaluationError::Core(QueryError::query(9..14, "work limit exceeded"))
    );
}
