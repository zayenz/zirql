use std::fmt::{self, Write};

use super::{ConstructedValue, allocation, error, finite, sum};
use crate::{Budget, Location, QueryError};

pub(crate) struct Specification {
    alignment: Option<char>,
    zero: bool,
    width: usize,
    precision: Option<usize>,
    kind: Option<char>,
}

impl Specification {
    pub(crate) fn parse(text: &str, location: &Location) -> Result<Self, QueryError> {
        let invalid = || error(location, "invalid scalar format specification");
        let mut chars = text.chars().peekable();
        let alignment = chars.next_if(|ch| matches!(ch, '<' | '>' | '^'));
        let zero = chars.next_if_eq(&'0').is_some();
        if zero && alignment.is_some() {
            return Err(invalid());
        }
        let width = read_number(&mut chars, location)?.unwrap_or(0);
        let precision = if chars.next_if_eq(&'.').is_some() {
            Some(read_number(&mut chars, location)?.ok_or_else(invalid)?)
        } else {
            None
        };
        let kind = chars.next();
        if kind.is_some_and(|ch| !matches!(ch, 's' | 'd' | 'x' | 'X' | 'b' | 'f' | 'e' | 'E'))
            || chars.next().is_some()
            || precision.is_some() && !matches!(kind, Some('f' | 'e' | 'E'))
        {
            return Err(invalid());
        }
        Ok(Self {
            alignment,
            zero,
            width,
            precision,
            kind,
        })
    }
}

fn read_number(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    location: &Location,
) -> Result<Option<usize>, QueryError> {
    let mut value = None;
    while let Some(ch) = chars.next_if(char::is_ascii_digit) {
        value = Some(
            value
                .unwrap_or(0usize)
                .checked_mul(10)
                .and_then(|n| n.checked_add(ch as usize - '0' as usize))
                .ok_or_else(|| error(location, "invalid scalar format specification"))?,
        );
    }
    Ok(value)
}

pub(super) fn format(
    value: &ConstructedValue,
    specification: &str,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    budget.charge(specification.len(), location.clone())?;
    let spec = Specification::parse(specification, location)?;
    let mismatch = || {
        error(
            location,
            "format specification does not support this scalar type",
        )
    };
    match value.data() {
        ConstructedValue::String(text) if matches!(spec.kind, None | Some('s')) && !spec.zero => {
            budget.charge(text.len(), location.clone())?;
            padded_text(text, text.chars().count(), &spec, budget, location)
        }
        ConstructedValue::Boolean(value) if spec.kind.is_none() && !spec.zero => {
            let text = if *value { "true" } else { "false" };
            padded_text(text, text.len(), &spec, budget, location)
        }
        ConstructedValue::Integer(value) => {
            let magnitude = value.unsigned_abs();
            let sign = if *value < 0 { "-" } else { "" };
            match spec.kind {
                None | Some('d') => {
                    numeric_text(format_args!("{value}"), 20, &spec, budget, location)
                }
                Some('x') => numeric_text(
                    format_args!("{sign}{magnitude:x}"),
                    17,
                    &spec,
                    budget,
                    location,
                ),
                Some('X') => numeric_text(
                    format_args!("{sign}{magnitude:X}"),
                    17,
                    &spec,
                    budget,
                    location,
                ),
                Some('b') => numeric_text(
                    format_args!("{sign}{magnitude:b}"),
                    65,
                    &spec,
                    budget,
                    location,
                ),
                _ => Err(mismatch()),
            }
        }
        ConstructedValue::Float(value) => {
            finite(*value, location)?;
            let requested_precision = spec.precision.unwrap_or(6);
            let work_bound = match spec.kind {
                None => 24,
                Some('f') => sum(requested_precision, 311, location)?,
                Some('e' | 'E') => sum(requested_precision, 9, location)?,
                _ => return Err(mismatch()),
            };
            budget.charge(work_bound, location.clone())?;
            // Binary64 decimal expansions end within 1074 fractional digits.
            // Further precision adds zeroes without increasing Rust's precision counter.
            let precision = requested_precision.min(1074);
            match spec.kind {
                None => numeric_text(format_args!("{value:?}"), 0, &spec, budget, location),
                Some('f') => numeric_text(
                    format_args!("{value:.precision$}"),
                    0,
                    &spec,
                    budget,
                    location,
                ),
                Some('e') => numeric_text(
                    format_args!("{value:.precision$e}"),
                    0,
                    &spec,
                    budget,
                    location,
                ),
                Some('E') => numeric_text(
                    format_args!("{value:.precision$E}"),
                    0,
                    &spec,
                    budget,
                    location,
                ),
                _ => Err(mismatch()),
            }
        }
        ConstructedValue::String(_) | ConstructedValue::Boolean(_) => Err(mismatch()),
        _ => Err(error(
            location,
            "format needs a Boolean, integer, finite float or string",
        )),
    }
}

fn numeric_text(
    arguments: fmt::Arguments<'_>,
    work_bound: usize,
    spec: &Specification,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    struct ByteCount(usize);
    impl Write for ByteCount {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            self.0 = self.0.checked_add(text.len()).ok_or(fmt::Error)?;
            Ok(())
        }
    }
    // Charge for precision before Rust's formatter performs proportional work.
    budget.charge(work_bound, location.clone())?;
    budget.charge(spec.width, location.clone())?;
    let mut count = ByteCount(0);
    count
        .write_fmt(arguments)
        .map_err(|_| error(location, "formatted text size overflow"))?;
    let extra_zeroes = if matches!(spec.kind, Some('f' | 'e' | 'E')) {
        spec.precision.unwrap_or(6).saturating_sub(1074)
    } else {
        0
    };
    let length = sum(count.0, extra_zeroes, location)?;
    let padding = spec.width.saturating_sub(length);
    let bytes = sum(length, padding, location)?;
    admit_output(bytes, budget, location)?;
    let mut text = String::with_capacity(bytes);
    text.write_fmt(arguments)
        .expect("writing to String cannot fail");
    if extra_zeroes > 0 {
        let position = text.find(['e', 'E']).unwrap_or(text.len());
        text = insert_fill(text, position, extra_zeroes, b'0');
    }
    let text = pad(text, padding, true, spec);
    Ok(ConstructedValue::String(text))
}

fn padded_text(
    text: &str,
    characters: usize,
    spec: &Specification,
    budget: &mut Budget,
    location: &Location,
) -> Result<ConstructedValue, QueryError> {
    let padding = spec.width.saturating_sub(characters);
    let bytes = sum(text.len(), padding, location)?;
    admit_output(bytes, budget, location)?;
    let mut result = String::with_capacity(bytes);
    result.push_str(text);
    let result = pad(result, padding, false, spec);
    Ok(ConstructedValue::String(result))
}

fn admit_output(bytes: usize, budget: &mut Budget, location: &Location) -> Result<(), QueryError> {
    allocation::<u8>(bytes, location)?;
    budget.charge(sum(bytes, bytes, location)?, location.clone())?;
    budget.charge_results(1, location.clone())
}

fn pad(text: String, padding: usize, numeric: bool, spec: &Specification) -> String {
    if padding == 0 {
        return text;
    }
    let (left, fill, start) = if spec.zero {
        (padding, b'0', usize::from(text.starts_with('-')))
    } else {
        let alignment = spec.alignment.unwrap_or(if numeric { '>' } else { '<' });
        let left = match alignment {
            '>' => padding,
            '^' => padding / 2,
            _ => 0,
        };
        (left, b' ', 0)
    };
    let mut text = insert_fill(text, start, left, fill);
    text.extend(std::iter::repeat_n(' ', padding - left));
    text
}

fn insert_fill(text: String, position: usize, count: usize, fill: u8) -> String {
    let mut bytes = text.into_bytes();
    let length = bytes.len();
    bytes.resize(length + count, fill);
    bytes.copy_within(position..length, position + count);
    bytes[position..position + count].fill(fill);
    String::from_utf8(bytes).expect("padding preserves complete UTF-8 text")
}
