//! Source-range protection, adapted from bilingual-view. No translation eligibility or splitting.
use anyhow::{Result, anyhow};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    path::Path,
    sync::LazyLock,
};
use url::Url;

type Span = Range<usize>;

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
}

fn escaped(source: &str, at: usize) -> bool {
    source.as_bytes()[..at]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

fn overlaps(a: &Span, b: &Span) -> bool {
    a.start < b.end && b.start < a.end
}

fn label_end(source: &str, span: &Span) -> Option<usize> {
    let raw = &source[span.clone()];
    let first = raw.find('[')? + span.start;
    let bytes = source.as_bytes();
    let mut depth = 1;
    let mut i = first + 1;
    while i < span.end {
        match bytes[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'`' => {
                let width = bytes[i..span.end]
                    .iter()
                    .take_while(|&&b| b == b'`')
                    .count();
                let delimiter = "`".repeat(width);
                if let Some(end) = source[i + width..span.end].find(&delimiter) {
                    i += width + end + width;
                    continue;
                }
                i += width;
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

// Changing the display label of a shortcut/collapsed reference must not change its destination.
fn explicit_references(source: &str) -> String {
    let mut changes = Vec::new();
    for (event, span) in Parser::new_ext(source, options()).into_offset_iter() {
        if let Event::Start(Tag::Link { link_type, .. } | Tag::Image { link_type, .. }) = event
            && matches!(link_type, LinkType::Shortcut | LinkType::Collapsed)
            && let Some(close) = label_end(source, &span)
        {
            let open = source[span.clone()].find('[').unwrap() + span.start;
            let label = &source[open + 1..close];
            let range = if link_type == LinkType::Collapsed && source[close + 1..].starts_with("[]")
            {
                close + 1..close + 3
            } else {
                close + 1..close + 1
            };
            changes.push((range, format!("[{label}]")));
        }
    }
    replace_ranges(source, changes)
}

fn replace_ranges(source: &str, mut changes: Vec<(Span, String)>) -> String {
    changes.sort_by(|a, b| b.0.start.cmp(&a.0.start).then(b.0.end.cmp(&a.0.end)));
    let mut output = source.to_owned();
    for (span, value) in changes {
        output.replace_range(span, &value);
    }
    output
}

static LEXICAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
    r#"https?://[^\s，；。！？、：<>"'）]+"#,
    r#"|@(?:"[^"\r\n]+"|'[^'\r\n]+'|[^\s,，;；。！？、：<>"'“”「」『』`]+)"#,
    r#"|(?:\.{0,2}/|~/|[A-Za-z]:[\\/]|[\p{L}\p{M}\p{N}_.-]+/)[^\s,，;；。！？、：<>"'“”「」『』`]+"#,
    r#"|(?:[\p{L}_][\p{L}\p{M}\p{N}_.-]*\.[A-Za-z0-9_-]+|\.[A-Za-z][A-Za-z0-9_-]*)"#,
    r#"|\\[[:punct:]]"#,
)).unwrap()
});

static QUOTED_REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""(?:\\.|[^"\\\r\n])*"|'(?:\\.|[^'\\\r\n])*'"#).unwrap());

fn quoted_path(text: &str) -> bool {
    static FILENAME_END: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\.[A-Za-z0-9_-]+$").unwrap());
    text.starts_with(['/', '~'])
        || text.starts_with("./")
        || text.starts_with("../")
        || text.starts_with("https://")
        || text.starts_with("http://")
        || (text.as_bytes().get(1) == Some(&b':')
            && text.as_bytes().get(2).is_some_and(|b| b"/\\".contains(b)))
        || FILENAME_END.is_match(text)
}

fn reference_end(source: &str, range: Span) -> usize {
    let mut stack = Vec::new();
    for (offset, c) in source[range.clone()].char_indices() {
        let at = range.start + offset;
        if escaped(source, at) {
            continue;
        }
        if c == '|' {
            return at;
        }
        match c {
            '(' => stack.push(')'),
            '[' => stack.push(']'),
            '{' => stack.push('}'),
            ')' | ']' | '}' if stack.pop() != Some(c) => return at,
            _ => {}
        }
    }
    range.end
}

fn math_spans(source: &str, opaque: &[Span], tables: &[Span]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < source.len() {
        let tail = &source[cursor..];
        let start = cursor;
        if opaque.iter().any(|span| span.contains(&start)) || escaped(source, start) {
            cursor += tail.chars().next().unwrap().len_utf8();
            continue;
        }
        let pair = if tail.starts_with("\\(") {
            Some(("\\(", "\\)"))
        } else if tail.starts_with("\\[") {
            Some(("\\[", "\\]"))
        } else if tail.starts_with("$$") {
            Some(("$$", "$$"))
        } else if tail.starts_with('$')
            && !tail[1..].starts_with(char::is_whitespace)
            && !source[..start].ends_with('$')
        {
            Some(("$", "$"))
        } else {
            None
        };
        if let Some((open, close)) = pair {
            let mut search = start + open.len();
            while let Some(relative) = source[search..].find(close) {
                let end = search + relative;
                if escaped(source, end) {
                    search = end + close.len();
                    continue;
                }
                let span = start..end + close.len();
                let single_bad = open == "$"
                    && (source[start..end].contains('\n')
                        || source[..end].ends_with(char::is_whitespace)
                        || source[end + 1..].starts_with(|c: char| c.is_ascii_digit() || c == '$'));
                let crosses_cell = tables.iter().any(|t| t.contains(&start))
                    && source[span.clone()]
                        .char_indices()
                        .any(|(i, c)| c == '|' && !escaped(source, start + i));
                if !single_bad && !crosses_cell && !opaque.iter().any(|s| overlaps(s, &span)) {
                    cursor = span.end;
                    spans.push(span);
                }
                break;
            }
        }
        if cursor == start {
            cursor += tail.chars().next().unwrap().len_utf8();
        }
    }
    spans
}

pub struct ProtectedText {
    pub text: String,
    pub prefix: String,
    pub values: Vec<String>,
    pub warnings: Vec<String>,
}

impl ProtectedText {
    pub fn new(original: &str) -> Self {
        let source = explicit_references(original);
        let parser = Parser::new_ext(&source, options());
        let mut spans: Vec<Span> = parser
            .reference_definitions()
            .iter()
            .map(|(_, d)| d.span.clone())
            .collect();
        let mut text_spans = Vec::new();
        let mut tables = Vec::new();
        let mut warnings = BTreeSet::new();
        for (event, span) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::CodeBlock(_)) | Event::Code(_) => spans.push(span),
                Event::Start(Tag::MetadataBlock(_)) => {
                    warnings.insert("frontmatter 已按原文保留".to_string());
                    spans.push(span);
                }
                Event::Start(Tag::HtmlBlock) | Event::Html(_) | Event::InlineHtml(_) => {
                    warnings.insert("原始 HTML 已按原文保留".to_string());
                    spans.push(span);
                }
                Event::Start(Tag::Link { link_type, .. } | Tag::Image { link_type, .. }) => {
                    match link_type {
                        LinkType::Autolink | LinkType::Email => spans.push(span),
                        LinkType::Inline => {
                            if let Some(close) = label_end(&source, &span)
                                && close + 2 < span.end - 1
                            {
                                spans.push(close + 2..span.end - 1);
                            }
                        }
                        LinkType::Reference => {
                            if let Some(close) = label_end(&source, &span)
                                && close + 2 < span.end - 1
                            {
                                spans.push(close + 2..span.end - 1);
                            }
                        }
                        _ => {}
                    }
                }
                Event::FootnoteReference(_) => spans.push(span),
                Event::Start(Tag::FootnoteDefinition(_)) => {
                    if let Some(close) = label_end(&source, &span) {
                        spans.push(span.start..close + 1);
                    }
                }
                Event::Start(Tag::Table(_)) => tables.push(span),
                Event::Text(_) => text_spans.push(span),
                _ => {}
            }
        }
        spans.extend(math_spans(&source, &spans, &tables));
        let mut quote_cursor = 0;
        while let Some(quoted) = QUOTED_REFERENCE.find_at(&source, quote_cursor) {
            let span = quoted.start()..quoted.end();
            if source[..span.start]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric)
            {
                // A rejected opener's closing quote may be the start of a real path.
                quote_cursor = span.start + 1;
                continue;
            }
            quote_cursor = span.end;
            if !spans.iter().any(|other| overlaps(other, &span))
                && quoted_path(&quoted.as_str()[1..quoted.as_str().len() - 1])
            {
                spans.push(span);
            }
        }
        for text in text_spans {
            for capture in LEXICAL.find_iter(&source[text.clone()]) {
                let start = text.start + capture.start();
                if spans.iter().any(|span| span.contains(&start)) {
                    continue;
                }
                let before = source[..start].chars().next_back();
                if !capture.as_str().starts_with('\\')
                    && before.is_some_and(|c| c.is_alphanumeric() || "_./\\-".contains(c))
                {
                    continue;
                }
                let mut end = reference_end(&source, start..text.start + capture.end());
                if ["e.g", "i.e"].contains(&capture.as_str().to_ascii_lowercase().as_str())
                    && source[end..].starts_with('.')
                {
                    continue;
                }
                while end > start && source[..end].ends_with(['.', ',', ';', ':', '!', '?']) {
                    end -= 1;
                }
                if end > start {
                    spans.push(start..end);
                }
            }
        }
        spans.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
        let mut selected: Vec<Span> = Vec::new();
        for span in spans {
            if span.is_empty() {
                continue;
            }
            if let Some(previous) = selected.last_mut()
                && span.start < previous.end
            {
                previous.end = previous.end.max(span.end);
                continue;
            }
            selected.push(span);
        }
        let mut prefix = "MD_LITERAL_".to_owned();
        while source.contains(&prefix) {
            prefix.insert(0, '_');
        }
        let mut text = String::new();
        let mut values = Vec::new();
        let mut cursor = 0;
        for span in selected {
            text.push_str(&source[cursor..span.start]);
            text.push_str(&format!("{prefix}{}_END", values.len()));
            values.push(source[span.clone()].to_owned());
            cursor = span.end;
        }
        text.push_str(&source[cursor..]);
        Self {
            text,
            values,
            prefix,
            warnings: warnings.into_iter().collect(),
        }
    }

    pub fn body(&self) -> &str {
        self.text.trim()
    }

    pub fn restore(&self, translated: &str) -> Result<String> {
        let translated = translated.trim();
        if self.text.trim().is_empty() {
            anyhow::ensure!(translated.is_empty(), "模型为原本空白的文件新增了正文");
            return Ok(self.text.clone());
        }
        let pattern = Regex::new(&format!(r"{}(\d+)_END", regex::escape(&self.prefix))).unwrap();
        let expected = locations(self.body(), &pattern);
        let actual = locations(translated, &pattern);
        let mut problems = Vec::new();
        for (token, before) in &expected {
            match actual.get(token) {
                None => problems.push(format!("missing {token}")),
                Some(after) if after.len() != 1 => problems.push(format!("duplicated {token}")),
                Some(after) if before != after => problems.push(format!(
                    "moved {token} from {:?} to {:?}",
                    before[0], after[0]
                )),
                _ => {}
            }
        }
        for token in actual.keys() {
            if !expected.contains_key(token) {
                problems.push(format!("unknown {token}"));
            }
        }
        if !problems.is_empty() {
            return Err(anyhow!(
                "{}",
                problems.into_iter().take(8).collect::<Vec<_>>().join("; ")
            ));
        }
        let restored = pattern.replace_all(translated, |captures: &regex::Captures<'_>| {
            self.values[captures[1].parse::<usize>().unwrap()].clone()
        });
        let leading = self.text.len() - self.text.trim_start().len();
        let trailing = self.text.trim_end().len();
        Ok(format!(
            "{}{}{}",
            &self.text[..leading],
            restored,
            &self.text[trailing..]
        ))
    }
}

fn locations(text: &str, pattern: &Regex) -> BTreeMap<String, Vec<(usize, usize)>> {
    let mut found: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let (mut cursor, mut line, mut cell) = (0, 1, 1);
    for token in pattern.find_iter(text) {
        for (offset, c) in text[cursor..token.start()].char_indices() {
            if c == '\n' {
                line += 1;
                cell = 1;
            } else if c == '|' && !escaped(text, cursor + offset) {
                cell += 1;
            }
        }
        found
            .entry(token.as_str().to_owned())
            .or_default()
            .push((line, cell));
        cursor = token.end();
    }
    found
}

fn destination_span(source: &str, span: &Span, definition: bool) -> Option<Span> {
    let close = label_end(source, span)?;
    let mut i = close + 2; // ]( or ]:
    while i < span.end && source.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    if source.as_bytes().get(i) == Some(&b'<') {
        let start = i + 1;
        let end = source[start..span.end].find('>')? + start;
        return Some(start..end);
    }
    let start = i;
    let mut depth = 0;
    while i < span.end {
        match source.as_bytes()[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'(' => depth += 1,
            b')' if depth == 0 && !definition => break,
            b')' if depth > 0 => depth -= 1,
            c if c.is_ascii_whitespace() && depth == 0 => break,
            _ => {}
        }
        i += 1;
    }
    Some(start..i)
}

/// Resolve only a preview copy; cached Markdown stays independent of its storage location.
pub fn preview_markdown(markdown: &str, original: &Path) -> Result<String> {
    preview_resources(markdown, original, false)
}

pub fn remote_preview_markdown(markdown: &str, original: &Path) -> Result<String> {
    preview_resources(markdown, original, true)
}

fn preview_resources(markdown: &str, original: &Path, remote: bool) -> Result<String> {
    let base = Url::from_directory_path(original.parent().ok_or_else(|| anyhow!("原文缺少目录"))?)
        .map_err(|_| anyhow!("无法构造原文目录 URL"))?;
    let parser = Parser::new_ext(markdown, options());
    let mut resources = Vec::new();
    for (_, definition) in parser.reference_definitions().iter() {
        if let Some(span) = destination_span(markdown, &definition.span, true) {
            resources.push((span, definition.dest.to_string()));
        }
    }
    for (event, span) in parser.into_offset_iter() {
        if let Event::Start(
            Tag::Link {
                link_type: LinkType::Inline,
                dest_url,
                ..
            }
            | Tag::Image {
                link_type: LinkType::Inline,
                dest_url,
                ..
            },
        ) = event
            && let Some(destination) = destination_span(markdown, &span, false)
        {
            resources.push((destination, dest_url.to_string()));
        }
    }
    let mut changes = Vec::new();
    for (span, destination) in resources {
        if destination.is_empty()
            || destination.starts_with('#')
            || destination.starts_with("//")
            || Url::parse(&destination).is_ok()
        {
            continue;
        }
        let url = base.join(&destination)?;
        let absolute = if remote {
            url.as_str().strip_prefix("file://").unwrap_or(url.as_str())
        } else {
            url.as_str()
        }
        .replace('(', "%28")
        .replace(')', "%29");
        changes.push((span, absolute));
    }
    Ok(replace_ranges(markdown, changes))
}
