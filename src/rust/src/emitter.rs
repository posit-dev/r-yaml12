//! YAML serialization with support for wrapping long strings.
//!
//! Derived from the `saphyr` crate's emitter (`src/emitter.rs`, MIT OR
//! Apache-2.0, <https://github.com/saphyr-rs/saphyr>), extended with:
//!
//! - `string_wrap_width`: long single-line strings and paragraph-shaped
//!   multiline strings are emitted as folded block scalars wrapped at word
//!   boundaries. Folding round-trips the original spaces and paragraph
//!   breaks exactly.
//! - Mapping keys never use block styles. Keys longer than YAML's simple-key
//!   limit use explicit mapping syntax.

use core::fmt::{self, Write as _};
use std::borrow::Cow;

use saphyr::{EmitError, Mapping, Scalar, Tag, Yaml};

const INDENT: &str = "  ";

struct ColumnTrackingWriter<'a> {
    writer: &'a mut dyn fmt::Write,
    column: usize,
}

#[derive(Clone, Copy)]
enum Chomping {
    Strip,
    Clip,
}

struct FoldedBlock<'a> {
    lines: Vec<&'a str>,
    chomping: Chomping,
}

impl fmt::Write for ColumnTrackingWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.writer.write_str(s)?;
        if let Some((_, suffix)) = s.rsplit_once('\n') {
            self.column = suffix.chars().count();
        } else {
            self.column += s.chars().count();
        }
        Ok(())
    }
}

/// The YAML serializer.
pub struct YamlEmitter<'a> {
    writer: ColumnTrackingWriter<'a>,
    level: usize,
    string_wrap_width: Option<usize>,
    emitting_key: bool,
}

/// A convenience alias for emitter functions that may fail without returning a value.
pub type EmitResult = Result<(), EmitError>;

fn special_floating_point(value: f64) -> Option<&'static str> {
    if value.is_nan() {
        Some(".NaN")
    } else if value == f64::INFINITY {
        Some(".Inf")
    } else if value == f64::NEG_INFINITY {
        Some("-.Inf")
    } else {
        None
    }
}

fn escape_sequence(byte: u8) -> Option<&'static str> {
    Some(match byte {
        b'"' => "\\\"",
        b'\\' => "\\\\",
        b'\x00' => "\\u0000",
        b'\x01' => "\\u0001",
        b'\x02' => "\\u0002",
        b'\x03' => "\\u0003",
        b'\x04' => "\\u0004",
        b'\x05' => "\\u0005",
        b'\x06' => "\\u0006",
        b'\x07' => "\\u0007",
        b'\x08' => "\\b",
        b'\t' => "\\t",
        b'\n' => "\\n",
        b'\x0b' => "\\u000b",
        b'\x0c' => "\\f",
        b'\r' => "\\r",
        b'\x0e' => "\\u000e",
        b'\x0f' => "\\u000f",
        b'\x10' => "\\u0010",
        b'\x11' => "\\u0011",
        b'\x12' => "\\u0012",
        b'\x13' => "\\u0013",
        b'\x14' => "\\u0014",
        b'\x15' => "\\u0015",
        b'\x16' => "\\u0016",
        b'\x17' => "\\u0017",
        b'\x18' => "\\u0018",
        b'\x19' => "\\u0019",
        b'\x1a' => "\\u001a",
        b'\x1b' => "\\u001b",
        b'\x1c' => "\\u001c",
        b'\x1d' => "\\u001d",
        b'\x1e' => "\\u001e",
        b'\x1f' => "\\u001f",
        b'\x7f' => "\\u007f",
        _ => return None,
    })
}

// from serialize::json
fn escape_str(wr: &mut dyn fmt::Write, v: &str) -> Result<(), fmt::Error> {
    wr.write_str("\"")?;

    let mut start = 0;

    for (i, byte) in v.bytes().enumerate() {
        let Some(escaped) = escape_sequence(byte) else {
            continue;
        };

        if start < i {
            wr.write_str(&v[start..i])?;
        }

        wr.write_str(escaped)?;

        start = i + 1;
    }

    if start != v.len() {
        wr.write_str(&v[start..])?;
    }

    wr.write_str("\"")?;
    Ok(())
}

/// Extra columns added by double-quoting and escaping a string.
fn escaped_str_overhead(v: &str) -> usize {
    2 + v
        .bytes()
        .filter_map(escape_sequence)
        .map(|escaped| escaped.len() - 1)
        .sum::<usize>()
}

impl<'a> YamlEmitter<'a> {
    /// Create a new emitter serializing into `writer`.
    pub fn new(writer: &'a mut dyn fmt::Write) -> Self {
        YamlEmitter {
            writer: ColumnTrackingWriter { writer, column: 0 },
            level: 0,
            string_wrap_width: None,
            emitting_key: false,
        }
    }

    /// Wrap long single-line strings and paragraph-shaped multiline strings
    /// as folded block scalars, breaking at word boundaries so that emitted
    /// lines stay within `width` columns (including indentation) whenever
    /// possible. Wrapping never changes the parsed value.
    pub fn string_wrap_width(&mut self, width: Option<usize>) {
        self.string_wrap_width = width;
    }

    /// Dump Yaml to an output stream.
    /// # Errors
    /// Returns `EmitError` when an error occurs.
    pub fn dump(&mut self, doc: &Yaml) -> EmitResult {
        // write DocumentStart
        writeln!(self.writer, "---")?;
        self.level = 0;
        self.emit_node(doc)
    }

    /// Dump multiple YAML documents to an output stream as a stream.
    ///
    /// Each document is preceded by a document start marker (`---`) and
    /// followed by a newline.
    pub fn dump_docs(&mut self, docs: &[Yaml]) -> EmitResult {
        for doc in docs {
            self.dump(doc)?;
            self.writer.write_str("\n")?;
        }
        Ok(())
    }

    fn write_indent(&mut self) -> EmitResult {
        for _ in 1..self.level {
            self.writer.write_str(INDENT)?;
        }
        Ok(())
    }

    fn emit_node(&mut self, node: &Yaml) -> EmitResult {
        match *node {
            Yaml::Sequence(ref v) => self.emit_sequence(v),
            Yaml::Mapping(ref h) => self.emit_mapping(h),
            Yaml::Value(Scalar::String(ref v)) => {
                if let Some(block) = self.folded_block(v) {
                    self.emit_folded_block(&block)?;
                } else if self.should_emit_string_as_block(v) {
                    self.emit_literal_block(v)?;
                } else if need_quotes(v) {
                    escape_str(&mut self.writer, v)?;
                } else {
                    write!(self.writer, "{v}")?;
                }
                Ok(())
            }
            Yaml::Value(Scalar::Boolean(v)) => {
                if v {
                    self.writer.write_str("true")?;
                } else {
                    self.writer.write_str("false")?;
                }
                Ok(())
            }
            Yaml::Value(Scalar::Integer(v)) => Ok(write!(self.writer, "{v}")?),
            Yaml::Value(Scalar::FloatingPoint(ref v)) => {
                if let Some(rendered) = special_floating_point(v.0) {
                    self.writer.write_str(rendered)?;
                } else {
                    write!(
                        self.writer,
                        "{v}{}",
                        if v.fract() == 0.0 { ".0" } else { "" }
                    )?;
                }
                Ok(())
            }
            Yaml::Value(Scalar::Null) => Ok(write!(self.writer, "~")?),
            Yaml::Tagged(ref tag, ref node) => {
                write!(self.writer, "{} ", tag.as_ref())?;
                // We need to insert a newline after the tag when followed by a
                // non-empty sequence or mapping. `emit_sequence` and
                // `emit_mapping` do not add that extra newline at the beginning.
                if node.is_non_empty_collection() {
                    self.level += 1;
                    writeln!(self.writer)?;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_node(node.as_ref())
            }
            Yaml::Representation(_, _, _) | Yaml::Alias(_) | Yaml::BadValue => {
                unreachable!("parser-only YAML node reached the private emitter")
            }
        }
    }

    fn emit_literal_block(&mut self, v: &str) -> EmitResult {
        self.writer.write_str("|")?;
        if needs_explicit_block_indent(v) {
            let indent_indicator = INDENT.len();
            write!(self.writer, "{indent_indicator}")?;
        }
        if !v.ends_with('\n') {
            self.writer.write_str("-")?;
        }

        // lines() will omit the last line if it is empty.
        self.emit_block_lines(v.lines())
    }

    fn emit_block_lines<'b>(&mut self, lines: impl IntoIterator<Item = &'b str>) -> EmitResult {
        let indent = self.block_indent();
        for line in lines {
            writeln!(self.writer)?;
            if !line.is_empty() {
                for _ in 0..indent {
                    self.writer.write_str(" ")?;
                }
            }
            // Block scalar content is literal text; no escaping.
            self.writer.write_str(line)?;
        }
        Ok(())
    }

    fn emit_folded_block(&mut self, block: &FoldedBlock<'_>) -> EmitResult {
        match block.chomping {
            Chomping::Strip => self.writer.write_str(">-")?,
            Chomping::Clip => self.writer.write_str(">")?,
        }
        self.emit_block_lines(block.lines.iter().copied())
    }

    fn emit_sequence(&mut self, v: &[Yaml]) -> EmitResult {
        if v.is_empty() {
            write!(self.writer, "[]")?;
        } else {
            self.level += 1;
            for (cnt, x) in v.iter().enumerate() {
                if cnt > 0 {
                    writeln!(self.writer)?;
                    self.write_indent()?;
                }
                write!(self.writer, "-")?;
                self.emit_val(true, x)?;
            }
            self.level -= 1;
        }
        Ok(())
    }

    fn emit_mapping(&mut self, h: &Mapping) -> EmitResult {
        if h.is_empty() {
            self.writer.write_str("{}")?;
        } else {
            self.level += 1;
            for (cnt, (k, v)) in h.iter().enumerate() {
                let explicit_key = requires_explicit_key(k);
                if cnt > 0 {
                    writeln!(self.writer)?;
                    self.write_indent()?;
                }
                if explicit_key {
                    write!(self.writer, "?")?;
                    self.emitting_key = true;
                    let key_result = self.emit_val(true, k);
                    self.emitting_key = false;
                    key_result?;
                    writeln!(self.writer)?;
                    self.write_indent()?;
                    write!(self.writer, ":")?;
                    self.emit_val(true, v)?;
                } else {
                    // Implicit keys must stay on one line: block styles
                    // (literal/folded) are not allowed there.
                    self.emitting_key = true;
                    let key_result = self.emit_node(k);
                    self.emitting_key = false;
                    key_result?;
                    write!(self.writer, ":")?;
                    self.emit_val(false, v)?;
                }
            }
            self.level -= 1;
        }
        Ok(())
    }

    /// Emit a yaml as a hash or array value: i.e., which should appear
    /// following a ":" or "-", either after a space, or on a new line.
    /// If `inline` is true, a collection may follow after a space.
    fn emit_val(&mut self, inline: bool, val: &Yaml) -> EmitResult {
        match *val {
            Yaml::Sequence(ref v) => {
                if inline || v.is_empty() {
                    write!(self.writer, " ")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_sequence(v)
            }
            Yaml::Mapping(ref h) => {
                if inline || h.is_empty() {
                    write!(self.writer, " ")?;
                } else {
                    writeln!(self.writer)?;
                    self.level += 1;
                    self.write_indent()?;
                    self.level -= 1;
                }
                self.emit_mapping(h)
            }
            _ => {
                write!(self.writer, " ")?;
                self.emit_node(val)
            }
        }
    }

    /// Check whether the string should be emitted as a literal block.
    #[must_use]
    fn should_emit_string_as_block(&self, s: &str) -> bool {
        !self.emitting_key && s.contains('\n') && is_safe_literal_block_scalar(s)
    }

    /// Return a lossless folded-block representation when the string should
    /// be wrapped, or `None` to use another scalar style.
    #[must_use]
    fn folded_block<'s>(&self, s: &'s str) -> Option<FoldedBlock<'s>> {
        let width = self.string_wrap_width?;
        // Keys must stay on one line.
        if self.emitting_key {
            return None;
        }

        let (body, chomping) = if let Some(body) = s.strip_suffix('\n') {
            if body.ends_with('\n') {
                return None;
            }
            (body, Chomping::Clip)
        } else {
            (s, Chomping::Strip)
        };

        if s.contains('\n') {
            return folded_paragraphs(body, width.saturating_sub(self.block_indent()))
                .map(|lines| FoldedBlock { lines, chomping });
        }

        if !is_foldable_string(body) {
            return None;
        }
        let inline_width = width.saturating_sub(self.writer.column);
        let inline_overhead = if need_quotes(body) {
            escaped_str_overhead(body)
        } else {
            0
        };
        let rendered_width = body.chars().count().saturating_add(inline_overhead);
        if rendered_width <= inline_width || !has_fold_point(body) {
            return None;
        }

        let block_width = width.saturating_sub(self.block_indent());
        let lines = folded_lines(body, block_width).unwrap_or_else(|| vec![body]);
        Some(FoldedBlock { lines, chomping })
    }

    fn block_indent(&self) -> usize {
        self.level.max(1) * INDENT.len()
    }
}

/// Copied from saphyr's crate-private `char_traits::is_valid_literal_block_scalar`.
fn is_valid_literal_block_scalar(string: &str) -> bool {
    string.chars().all(|character: char| {
        matches!(character, '\t' | '\n' | '\x20'..='\x7e' | '\u{0085}' | '\u{00a0}'..='\u{d7fff}')
    })
}

fn first_content_byte(string: &str) -> Option<u8> {
    string.bytes().find(|&byte| byte != b'\n')
}

/// Check that block indentation and clip chomping preserve the scalar exactly.
/// Multiple trailing newlines require keep chomping, which this emitter does
/// not produce. An all-empty multiline value may be consumed as separation
/// before the following node, so it is quoted.
fn is_safe_literal_block_scalar(string: &str) -> bool {
    is_valid_literal_block_scalar(string)
        && !string.ends_with("\n\n")
        && first_content_byte(string).is_some()
}

/// An explicit indentation indicator prevents leading spaces or a tab on the
/// first nonempty line from being mistaken for the block's indentation.
fn needs_explicit_block_indent(string: &str) -> bool {
    matches!(first_content_byte(string), Some(b' ' | b'\t'))
}

fn has_edge_whitespace(string: &str) -> bool {
    let mut chars = string.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    let last = chars.next_back().unwrap_or(first);
    first.is_whitespace() || last.is_whitespace()
}

/// Check that a string can be emitted as a folded block scalar without
/// changing its parsed value: block-scalar-safe characters only, no line
/// breaks, and no leading/trailing white space (which folding would drop or
/// which would confuse indentation detection).
fn is_foldable_string(s: &str) -> bool {
    !s.is_empty()
        && !has_edge_whitespace(s)
        && !s.contains('\n')
        && is_valid_literal_block_scalar(s)
}

fn has_fold_point(s: &str) -> bool {
    s.as_bytes().windows(3).any(|window| {
        window[1] == b' '
            && !matches!(window[0], b' ' | b'\t')
            && !matches!(window[2], b' ' | b'\t')
    })
}

/// Wrap unindented, single-line paragraphs separated by exactly two newlines.
/// Folded YAML needs two empty physical lines to preserve each paragraph break.
fn folded_paragraphs(s: &str, width: usize) -> Option<Vec<&str>> {
    let mut lines = Vec::new();
    let mut wrapped = false;

    for (index, paragraph) in s.split("\n\n").enumerate() {
        if !is_foldable_string(paragraph) {
            return None;
        }
        if index > 0 {
            lines.extend(["", ""]);
        }
        if let Some(paragraph_lines) = folded_lines(paragraph, width) {
            lines.extend(paragraph_lines);
            wrapped = true;
        } else {
            lines.push(paragraph);
        }
    }

    wrapped.then_some(lines)
}

/// Split `s` into lines of at most `width` characters for a folded block
/// scalar, or `None` when `s` fits or cannot be split.
///
/// Breaks are only taken at a single space flanked by non-white-space on both
/// sides: folding joins lines with exactly one space, and a line that starts
/// or ends with white space would be treated as "more indented" and not
/// folded. A word longer than `width` yields a longer line rather than being
/// split.
fn folded_lines(s: &str, width: usize) -> Option<Vec<&str>> {
    let total_chars = s.chars().count();
    if total_chars <= width {
        return None;
    }

    // (byte offset, char offset) of each break candidate.
    let mut candidates: Vec<(usize, usize)> = Vec::new();
    let mut chars = 0;
    let mut prev = '\0';
    let mut iter = s.char_indices().peekable();
    while let Some((idx, ch)) = iter.next() {
        if ch == ' '
            && !matches!(prev, '\0' | ' ' | '\t')
            && matches!(iter.peek(), Some(&(_, next)) if next != ' ' && next != '\t')
        {
            candidates.push((idx, chars));
        }
        chars += 1;
        prev = ch;
    }
    if candidates.is_empty() {
        return None;
    }

    let mut lines = Vec::new();
    let (mut start_byte, mut start_chars) = (0, 0);
    let mut next = 0;
    while total_chars - start_chars > width && next < candidates.len() {
        // Break at the furthest candidate that fits, or at the first one past
        // an over-long word.
        let mut chosen = None;
        while next < candidates.len() && candidates[next].1 - start_chars <= width {
            chosen = Some(candidates[next]);
            next += 1;
        }
        let (break_byte, break_chars) = match chosen {
            Some(candidate) => candidate,
            None => {
                let candidate = candidates[next];
                next += 1;
                candidate
            }
        };
        lines.push(&s[start_byte..break_byte]);
        // Skip the space; folding will restore it.
        (start_byte, start_chars) = (break_byte + 1, break_chars + 1);
    }
    lines.push(&s[start_byte..]);
    Some(lines)
}

/// Check whether a character may appear in a single-line plain scalar.
///
/// This is YAML 1.2 `c-printable` minus line breaks, tabs, and the byte order
/// mark. Tabs are technically permitted inside plain scalars but interact
/// badly with indentation handling in many parsers, so they are quoted.
fn is_plain_safe_char(c: char) -> bool {
    c != '\u{feff}'
        && matches!(
            c,
            '\x20'..='\x7e' | '\u{85}' | '\u{a0}'..='\u{d7ff}' | '\u{e000}'..='\u{10ffff}'
        )
}

/// Check for YAML 1.2 core-schema integer syntax without parsing the value.
fn is_core_schema_integer(string: &str) -> bool {
    let decimal = string
        .strip_prefix('+')
        .or_else(|| string.strip_prefix('-'))
        .unwrap_or(string);
    if !decimal.is_empty() && decimal.bytes().all(|byte| byte.is_ascii_digit()) {
        return true;
    }

    if let Some(octal) = string.strip_prefix("0o") {
        return !octal.is_empty() && octal.bytes().all(|byte| matches!(byte, b'0'..=b'7'));
    }

    if let Some(hexadecimal) = string.strip_prefix("0x") {
        return !hexadecimal.is_empty() && hexadecimal.bytes().all(|byte| byte.is_ascii_hexdigit());
    }

    false
}

/// Check if the string must be double-quoted rather than emitted as a plain
/// scalar.
///
/// This follows the YAML 1.2 plain-scalar grammar for block context (this
/// emitter never emits plain scalars inside flow collections), plus the core
/// schema for tag resolution — so `yes`, `on`, and friends are strings and do
/// not need quoting. Core integer syntax is detected independently because
/// the parser's resolver is bounded to `i64`; its resolver handles nulls,
/// booleans, and floats.
fn need_quotes(string: &str) -> bool {
    // The empty plain scalar resolves to null.
    let Some(first) = string.chars().next() else {
        return true;
    };

    // Would be resolved as a non-string by the core schema.
    if is_core_schema_integer(string)
        || !matches!(
            Scalar::parse_from_cow(Cow::Borrowed(string)),
            Scalar::String(_)
        )
    {
        return true;
    }

    // Plain scalars cannot start or end with white space, and every character
    // must be printable and single-line.
    if has_edge_whitespace(string) || !string.chars().all(is_plain_safe_char) {
        return true;
    }

    // First-character indicator rules (`c-indicator`): `-`, `?`, and `:` are
    // allowed when followed by a non-space character; the rest never are.
    match first {
        ',' | '[' | ']' | '{' | '}' | '#' | '&' | '*' | '!' | '|' | '>' | '\'' | '"' | '%'
        | '@' | '`' => return true,
        '-' | '?' | ':' if string.len() == 1 || string[1..].starts_with(' ') => return true,
        _ => {}
    }

    // Could be mistaken for a document marker when emitted at the start of a
    // line (root scalars and top-level keys sit in column 0).
    if string.starts_with("---") || string.starts_with("...") {
        return true;
    }

    // `: ` (or a trailing `:`) would end an implicit key, and ` #` would
    // start a comment.
    let bytes = string.as_bytes();
    bytes.iter().enumerate().any(|(i, &b)| match b {
        b':' => i + 1 == bytes.len() || bytes[i + 1] == b' ',
        b'#' => i > 0 && bytes[i - 1] == b' ',
        _ => false,
    })
}

// YAML limits an implicit simple key's emitted representation to 1024 Unicode
// characters. Longer keys require explicit `?` / `:` mapping syntax.
const MAX_SIMPLE_KEY_LENGTH: usize = 1024;

fn rendered_string_length(string: &str) -> usize {
    string.chars().count()
        + if need_quotes(string) {
            escaped_str_overhead(string)
        } else {
            0
        }
}

fn rendered_tag_length(tag: &Tag) -> usize {
    if tag.handle == "!" {
        1 + tag.suffix.chars().count()
    } else {
        tag.handle.chars().count() + 1 + tag.suffix.chars().count()
    }
}

/// Return the length of an implicit key's emitted representation, or `None`
/// when the key requires explicit mapping syntax regardless of length.
fn implicit_key_length(key: &Yaml<'_>) -> Option<usize> {
    match key {
        Yaml::Value(Scalar::String(string)) => Some(rendered_string_length(string)),
        Yaml::Value(Scalar::Boolean(value)) => Some(if *value { 4 } else { 5 }),
        Yaml::Value(Scalar::Integer(value)) => Some(value.to_string().len()),
        Yaml::Value(Scalar::FloatingPoint(value)) => {
            Some(special_floating_point(value.0).map_or_else(
                || value.to_string().len() + if value.fract() == 0.0 { 2 } else { 0 },
                str::len,
            ))
        }
        Yaml::Value(Scalar::Null) => Some(1),
        Yaml::Tagged(tag, node) => implicit_key_length(node).map(|length| {
            rendered_tag_length(tag)
                .saturating_add(1)
                .saturating_add(length)
        }),
        Yaml::Mapping(_) | Yaml::Sequence(_) => None,
        Yaml::Representation(_, _, _) | Yaml::Alias(_) | Yaml::BadValue => {
            unreachable!("parser-only YAML node reached the private emitter")
        }
    }
}

fn requires_explicit_key(key: &Yaml<'_>) -> bool {
    match implicit_key_length(key) {
        Some(length) => length > MAX_SIMPLE_KEY_LENGTH,
        None => true,
    }
}

#[cfg(test)]
mod test {
    use super::{folded_lines, YamlEmitter};
    use saphyr::{LoadableYamlNode, Scalar, Yaml};

    fn emit_wrapped(doc: &Yaml) -> String {
        let mut output = String::new();
        let mut emitter = YamlEmitter::new(&mut output);
        emitter.string_wrap_width(Some(80));
        emitter.dump(doc).unwrap();
        output
    }

    fn reparse(yaml: &str) -> Yaml<'static> {
        Yaml::load_from_str(yaml).unwrap().remove(0)
    }

    fn mapping_with_value(value: &str) -> Yaml<'static> {
        let mut mapping = saphyr::Mapping::new();
        mapping.insert(
            Yaml::Value(Scalar::String("key".into())),
            Yaml::Value(Scalar::String(value.to_string().into())),
        );
        Yaml::Mapping(mapping)
    }

    #[test]
    fn wraps_long_strings_as_folded_blocks() {
        let value = "word ".repeat(30).trim_end().to_string();
        let doc = mapping_with_value(&value);
        let output = emit_wrapped(&doc);
        assert!(output.contains("key: >-\n"), "{output}");
        for line in output.lines() {
            assert!(line.chars().count() <= 80, "line too long: {line:?}");
        }
        assert_eq!(reparse(&output), doc);
    }

    #[test]
    fn wrapping_respects_indentation() {
        let value = "word ".repeat(30).trim_end().to_string();
        let mut inner = saphyr::Mapping::new();
        inner.insert(
            Yaml::Value(Scalar::String("inner".into())),
            Yaml::Value(Scalar::String(value.into())),
        );
        let mut outer = saphyr::Mapping::new();
        outer.insert(
            Yaml::Value(Scalar::String("outer".into())),
            Yaml::Mapping(inner),
        );
        let doc = Yaml::Mapping(outer);
        let output = emit_wrapped(&doc);
        for line in output.lines() {
            assert!(line.chars().count() <= 80, "line too long: {line:?}");
        }
        assert_eq!(reparse(&output), doc);
    }

    #[test]
    fn short_and_unbreakable_strings_stay_inline() {
        let short = mapping_with_value("just a short string");
        assert!(emit_wrapped(&short).contains("key: just a short string"));

        let unbreakable = mapping_with_value(&"x".repeat(120));
        assert!(!emit_wrapped(&unbreakable).contains(">-"));
    }

    #[test]
    fn strings_with_unsafe_whitespace_are_not_folded() {
        for value in [
            format!(" {}", "word ".repeat(25).trim_end()),
            format!("{} ", "word ".repeat(25).trim_end()),
        ] {
            let doc = mapping_with_value(&value);
            let output = emit_wrapped(&doc);
            assert!(!output.contains(">-"), "{output}");
            assert_eq!(reparse(&output), doc);
        }

        // Double spaces and tab-adjacent spaces are not break points.
        let value = format!("{}  {}", "y".repeat(50), "z".repeat(50));
        let doc = mapping_with_value(&value);
        let output = emit_wrapped(&doc);
        assert!(!output.contains(">-"), "{output}");
        assert_eq!(reparse(&output), doc);
    }

    #[test]
    fn long_mapping_keys_are_not_wrapped() {
        let key = "word ".repeat(30).trim_end().to_string();
        let mut mapping = saphyr::Mapping::new();
        mapping.insert(
            Yaml::Value(Scalar::String(key.into())),
            Yaml::Value(Scalar::Integer(1)),
        );
        let doc = Yaml::Mapping(mapping);
        let output = emit_wrapped(&doc);
        assert!(!output.contains(">-"), "{output}");
        assert_eq!(reparse(&output), doc);
    }

    #[test]
    fn multiline_keys_are_quoted_not_literal_blocks() {
        let mut mapping = saphyr::Mapping::new();
        mapping.insert(
            Yaml::Value(Scalar::String("multi\nline".into())),
            Yaml::Value(Scalar::Integer(1)),
        );
        let doc = Yaml::Mapping(mapping);
        let output = emit_wrapped(&doc);
        assert!(output.contains("\"multi\\nline\": 1"), "{output}");
        assert_eq!(reparse(&output), doc);
    }

    #[test]
    fn folded_lines_break_only_at_safe_spaces() {
        let s = "aaaa bbbb cccc dddd";
        assert_eq!(folded_lines(s, 9), Some(vec!["aaaa bbbb", "cccc dddd"]));
        // A word longer than the width gets its own over-long line.
        let s = "aaaaaaaaaaaa bb";
        assert_eq!(folded_lines(s, 4), Some(vec!["aaaaaaaaaaaa", "bb"]));
        // No break points at all.
        assert_eq!(folded_lines(&"a".repeat(20), 4), None);
        // Already fits.
        assert_eq!(folded_lines("aa bb", 10), None);
    }
}
