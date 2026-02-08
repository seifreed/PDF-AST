use crate::parser::lexer::*;
use crate::types::*;
use log::warn;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{map, value},
    multi::many0,
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

pub fn parse_value(input: &[u8]) -> IResult<&[u8], PdfValue> {
    parse_value_with_depth(input, 0)
}

const MAX_NESTING_DEPTH: usize = 256;

fn parse_value_with_depth(input: &[u8], depth: usize) -> IResult<&[u8], PdfValue> {
    if depth > MAX_NESTING_DEPTH {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    preceded(
        skip_whitespace_and_comments,
        alt((
            parse_null,
            parse_boolean,
            parse_reference,
            parse_real_or_integer,
            parse_string,
            parse_name_value,
            |i| parse_array_with_depth(i, depth + 1),
            |i| parse_dictionary_with_depth(i, depth + 1),
        )),
    )(input)
}

fn parse_null(input: &[u8]) -> IResult<&[u8], PdfValue> {
    value(PdfValue::Null, tag(b"null"))(input)
}

fn parse_boolean(input: &[u8]) -> IResult<&[u8], PdfValue> {
    alt((
        value(PdfValue::Boolean(true), tag(b"true")),
        value(PdfValue::Boolean(false), tag(b"false")),
    ))(input)
}

fn parse_real_or_integer(input: &[u8]) -> IResult<&[u8], PdfValue> {
    alt((map(real, PdfValue::Real), map(integer, PdfValue::Integer)))(input)
}

fn parse_string(input: &[u8]) -> IResult<&[u8], PdfValue> {
    alt((
        map(hex_string, |bytes| {
            PdfValue::String(PdfString::new_hex(bytes))
        }),
        map(literal_string, |bytes| {
            PdfValue::String(PdfString::new_literal(bytes))
        }),
    ))(input)
}

fn parse_name_value(input: &[u8]) -> IResult<&[u8], PdfValue> {
    map(name, |n| PdfValue::Name(PdfName::new(n)))(input)
}

fn parse_array_with_depth(input: &[u8], depth: usize) -> IResult<&[u8], PdfValue> {
    map(
        delimited(
            terminated(char('['), skip_whitespace_and_comments),
            many0(|i| parse_value_with_depth(i, depth + 1)),
            preceded(skip_whitespace_and_comments, char(']')),
        ),
        |values| PdfValue::Array(PdfArray::from(values)),
    )(input)
}

fn parse_dictionary_with_depth(input: &[u8], depth: usize) -> IResult<&[u8], PdfValue> {
    map(
        delimited(
            terminated(tag(b"<<"), skip_whitespace_and_comments),
            many0(preceded(
                skip_whitespace_and_comments,
                separated_pair(name, skip_whitespace_and_comments, |i| {
                    parse_value_with_depth(i, depth + 1)
                }),
            )),
            preceded(skip_whitespace_and_comments, tag(b">>")),
        ),
        |pairs| {
            let mut dict = PdfDictionary::new();
            for (key, value) in pairs {
                dict.insert(key, value);
            }
            PdfValue::Dictionary(dict)
        },
    )(input)
}

fn parse_reference(input: &[u8]) -> IResult<&[u8], PdfValue> {
    map(
        tuple((
            integer,
            preceded(skip_whitespace, integer),
            preceded(skip_whitespace, char('R')),
        )),
        |(obj_num, gen_num, _)| {
            PdfValue::Reference(PdfReference::new(obj_num as u32, gen_num as u16))
        },
    )(input)
}

pub fn parse_indirect_object(input: &[u8]) -> IResult<&[u8], (ObjectId, PdfValue)> {
    let (input, obj_num) = integer(input)?;
    let (input, _) = skip_whitespace(input)?;
    let (input, gen_num) = integer(input)?;
    let (input, _) = skip_whitespace(input)?;
    let (input, _) = tag(b"obj")(input)?;
    let (input, _) = skip_whitespace_and_comments(input)?;
    let (input, value) = match parse_value(input) {
        Ok(result) => result,
        Err(err) => {
            warn!("Failed to parse object value, attempting recovery to endobj");
            if let Some(rest) = recover_to_endobj(input) {
                return Ok((
                    rest,
                    (
                        ObjectId::new(obj_num as u32, gen_num as u16),
                        PdfValue::Null,
                    ),
                ));
            }
            return Err(err);
        }
    };
    let (input, _) = skip_whitespace_and_comments(input)?;

    let (input, value) =
        if let Ok((input2, _)) = tag::<_, _, nom::error::Error<_>>(b"stream")(input) {
            let (input3, stream_value) = parse_stream_data(input2, value)?;
            (input3, stream_value)
        } else {
            (input, value)
        };

    let (input, _) = skip_whitespace_and_comments(input)?;
    let (input, _) = tag(b"endobj")(input)?;

    Ok((
        input,
        (ObjectId::new(obj_num as u32, gen_num as u16), value),
    ))
}

fn recover_to_endobj(input: &[u8]) -> Option<&[u8]> {
    let marker = b"endobj";
    input
        .windows(marker.len())
        .position(|w| w == marker)
        .map(|pos| &input[pos + marker.len()..])
}

fn parse_stream_data(input: &[u8], dict_value: PdfValue) -> IResult<&[u8], PdfValue> {
    parse_stream_data_with_resolver(input, dict_value, None)
}

fn parse_stream_data_with_resolver<'a>(
    input: &'a [u8],
    dict_value: PdfValue,
    _resolver: Option<
        &'a crate::parser::reference_resolver::ReferenceResolver<
            std::io::BufReader<std::io::Cursor<Vec<u8>>>,
        >,
    >,
) -> IResult<&'a [u8], PdfValue> {
    if let PdfValue::Dictionary(dict) = dict_value {
        let (input, _) = alt((tag(b"\r\n"), tag(b"\n")))(input)?;

        // Try to resolve Length - could be direct integer or indirect reference
        let length = match dict.get("Length") {
            Some(PdfValue::Integer(len)) => *len as usize,
            Some(PdfValue::Reference(pdf_ref)) => {
                // For now, we'll store the reference and handle it later in the resolver
                // This is a limitation of the current parser architecture
                warn!(
                    "Indirect Length reference {} {} R found - will need resolution",
                    pdf_ref.object_id().number,
                    pdf_ref.object_id().generation
                );
                // Try to read until "endstream" as fallback
                return parse_stream_with_endstream_detection(input, dict);
            }
            _ => {
                warn!("No Length found for stream, trying endstream detection");
                return parse_stream_with_endstream_detection(input, dict);
            }
        };

        let (input, data) = nom::bytes::complete::take(length)(input)?;
        let (input, _) = skip_whitespace(input)?;
        let (input, _) = tag(b"endstream")(input)?;

        let stream = PdfStream::new(dict, data.to_vec());
        Ok((input, PdfValue::Stream(stream)))
    } else {
        Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )))
    }
}

fn parse_stream_with_endstream_detection(
    input: &[u8],
    dict: PdfDictionary,
) -> IResult<&[u8], PdfValue> {
    // Find "endstream" marker
    let endstream_marker = b"endstream";
    let mut pos = 0;

    while pos + endstream_marker.len() <= input.len() {
        if &input[pos..pos + endstream_marker.len()] == endstream_marker {
            // Found endstream, check if it's properly delimited
            let before_endstream =
                if pos > 0 && (input[pos - 1] == b'\r' || input[pos - 1] == b'\n') {
                    pos - 1
                } else {
                    pos
                };

            let data = if before_endstream > 0
                && input[before_endstream - 1] == b'\r'
                && input[before_endstream] == b'\n'
            {
                input[0..before_endstream - 1].to_vec()
            } else {
                input[0..before_endstream].to_vec()
            };

            let remaining = &input[pos + endstream_marker.len()..];
            let stream = PdfStream::new(dict, data);
            return Ok((remaining, PdfValue::Stream(stream)));
        }
        pos += 1;
    }

    // If we get here, no endstream found - return error
    // Aggressive recovery: treat the rest of the buffer as stream data.
    let stream = PdfStream::new(dict, input.to_vec());
    Ok((&[][..], PdfValue::Stream(stream)))
}
