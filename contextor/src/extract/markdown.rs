use std::ops::Range;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde::Deserialize;

use super::{Extraction, Tags, identifier};

#[derive(Deserialize)]
struct FrontMatter {
    feature: Option<Features>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Features {
    One(String),
    Many(Vec<String>),
}

struct Heading {
    level: HeadingLevel,
    range: Range<usize>,
    title: String,
}

pub(super) fn has_front_matter(source: &str) -> bool {
    source.lines().next().is_some_and(|line| line.trim_end() == "---")
}

fn split_front_matter(source: &str) -> (Option<&str>, usize) {
    if has_front_matter(source) && let Some((opener, rest)) = source.split_once('\n') {
        let start = opener.len() + 1;
        let mut offset = start;
        for line in rest.split_inclusive('\n') {
            if matches!(line.trim_end(), "---" | "...") {
                return (Some(&source[start..offset]), offset + line.len());
            }
            offset += line.len();
        }
    }
    (None, 0)
}

pub(super) fn extract(extraction: &mut Extraction<'_>) {
    let source = extraction.source;
    let (front_matter, body_start) = split_front_matter(source);
    if let Some(yaml) = front_matter
        && let Ok(FrontMatter { feature: Some(features) }) = serde_saphyr::from_str(yaml)
    {
        let features = match features {
            Features::One(value) => vec![value],
            Features::Many(values) => values,
        };
        for value in &features {
            if let Some(feature) = identifier(value) {
                extraction.add(feature, None, None);
            }
        }
    }

    let mut headings: Vec<Heading> = Vec::new();
    let mut comments = Vec::new();
    let mut in_heading = false;
    for (event, range) in Parser::new(&source[body_start..]).into_offset_iter() {
        let range = (body_start + range.start)..(body_start + range.end);
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                headings.push(Heading { level, range, title: String::new() });
            }
            Event::End(TagEnd::Heading(_)) => in_heading = false,
            Event::Text(text) | Event::Code(text) if in_heading => {
                headings.last_mut().expect("heading started").title.push_str(&text);
            }
            Event::Start(Tag::HtmlBlock) | Event::InlineHtml(_) => {
                // A multiline HTML block emits its contents as separate line events.
                // Use the full block range to locate the opening and closing delimiters.
                let html = source[range.clone()].trim_start();
                if let Some(body) = html.strip_prefix("<!--")
                    && let Some((body, _)) = body.split_once("-->")
                {
                    let start = range.end - html.len();
                    let end = start + "<!--".len() + body.len() + "-->".len();
                    comments.push((start..end, Tags::parse(body)));
                }
            }
            _ => {}
        }
    }

    let mut header_end = body_start;
    for (range, tags) in comments {
        if source.get(header_end..range.start).is_some_and(|gap| gap.trim().is_empty()) {
            header_end = range.end;
            for feature in &tags.file_features {
                extraction.add(feature, None, None);
            }
        }
        if tags.start || tags.end {
            extraction.block_marker(range, tags);
        } else if !tags.features.is_empty() {
            add_comment_features(extraction, &headings, range, tags);
        }
    }
}

fn add_comment_features(
    extraction: &mut Extraction<'_>,
    headings: &[Heading],
    comment: Range<usize>,
    tags: Tags<'_>,
) {
    let source = extraction.source;
    let remaining = &headings[headings.partition_point(|heading| heading.range.start < comment.end)..];
    let heading = remaining.first().filter(|heading| {
        let gap = &source[comment.end..heading.range.start];
        gap.trim().is_empty() && gap.bytes().filter(|byte| *byte == b'\n').count() <= 1
    });

    if let Some(heading) = heading {
        let end = remaining[1..]
            .iter()
            .find(|next| next.level <= heading.level)
            .map_or(source.len(), |next| next.range.start);
        let start = heading.range.start;
        let end = start + source[start..end].trim_end().len();
        for feature in tags.features {
            extraction.add(feature, Some(&heading.title), Some(start..end));
        }
    } else {
        for feature in tags.features {
            extraction.add(feature, None, Some(comment.clone()));
        }
    }
}
