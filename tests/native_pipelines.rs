use std::{ops::Range, rc::Rc};

use zirql::{
    ArgumentReader, Budget, Host, Limits, Located, Location, NativeInput, NativeResult,
    NativeValue, Pipeline, PredicateKind, QueryError, Stage, Truth, evaluate, parse_pipeline,
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
    fn evaluate_leaf(
        &self,
        leaf: &SourceLeaf,
        item: &Selection<'a>,
        _context: &Rc<SourceView<'a>>,
        _budget: &mut Budget,
        _range: &Range<usize>,
    ) -> Result<Truth, QueryError> {
        Ok(match leaf {
            SourceLeaf::Prefix(prefix) if item.text.starts_with(&prefix.value) => Truth::True,
            SourceLeaf::Prefix(_) | SourceLeaf::No => Truth::False,
            SourceLeaf::Yes => Truth::True,
            SourceLeaf::Unknown => Truth::Unknown,
            SourceLeaf::Fail => {
                return Err(QueryError::source(item.range.clone(), "source leaf failed"));
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
) -> Result<NativeResult<Selection<'a>, Rc<SourceView<'a>>>, QueryError> {
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
    fn evaluate_leaf(
        &self,
        leaf: &&'static str,
        item: &OperationId,
        context: &Rc<Document>,
        _budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, QueryError> {
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
    assert_eq!(error, QueryError::source(0..5, "source leaf failed"));
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
    assert_eq!(error, QueryError::query(0..5, "work limit exceeded"));

    let error = parse_pipeline("filter(not yes)", source_leaf, 2)
        .err()
        .unwrap();
    assert_eq!(error, QueryError::query(11..11, "nesting limit exceeded"));
    let mut budget = Budget::new(Limits {
        nesting: 1,
        ..limits()
    });
    let error = source_run("filter(yes)", "alpha", &mut budget).unwrap_err();
    assert_eq!(error, QueryError::query(7..10, "nesting limit exceeded"));
    assert_eq!(
        source_run("count", "alpha", &mut budget).unwrap().value,
        NativeValue::Count(1)
    );
}

struct NestedHost {
    pipeline: Pipeline<()>,
}

impl Host<usize, (), ()> for NestedHost {
    fn evaluate_leaf(
        &self,
        _leaf: &(),
        item: &usize,
        _context: &(),
        budget: &mut Budget,
        range: &Range<usize>,
    ) -> Result<Truth, QueryError> {
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
    assert_eq!(error, QueryError::query(0..5, "nesting limit exceeded"));

    let mut budget = Budget::new(Limits {
        work: 5,
        ..limits()
    });
    let input = NativeInput::collect((), [9], &mut budget, Location::Source(0..1)).unwrap();
    let error = evaluate(&pipeline, &host, input, &mut budget).unwrap_err();
    assert_eq!(error, QueryError::query(0..5, "work limit exceeded"));
}
