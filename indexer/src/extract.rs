mod code;
mod markdown;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

use anyhow::Result;

#[derive(Clone, Copy)]
pub enum Language {
    JavaScript,
    TypeScript,
    Tsx,
    Markdown,
}

impl Language {
    pub fn for_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "js" | "mjs" | "cjs" | "jsx" => Some(Self::JavaScript),
            "ts" | "mts" | "cts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "md" | "markdown" => Some(Self::Markdown),
            _ => None,
        }
    }

    pub fn extract(self, source: &str) -> Result<Vec<Annotation>> {
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        if !source.contains("@feature")
            && !source.contains("@file-feature")
            && !(matches!(self, Self::Markdown) && markdown::has_front_matter(source))
        {
            return Ok(Vec::new());
        }

        let mut extraction = Extraction::new(source);
        match self {
            Self::Markdown => markdown::extract(&mut extraction),
            language => code::extract(language, &mut extraction)?,
        }
        Ok(extraction.annotations.into_iter().collect())
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(Debug, serde::Deserialize))]
pub struct Annotation {
    pub feature: String,
    pub symbol: Option<String>,
    pub lines: Option<(u32, u32)>,
}

#[derive(Default)]
struct Tags<'a> {
    features: Vec<&'a str>,
    file_features: Vec<&'a str>,
    start: bool,
    end: bool,
    end_feature: Option<&'a str>,
}

impl<'a> Tags<'a> {
    fn parse(comment: &'a str) -> Self {
        let mut tags = Self::default();
        for line in comment.lines() {
            let line = line.trim().trim_start_matches('*').trim_start();
            let mut words = line.split_whitespace();
            let tag = words.next();
            let argument = words.next();
            let value = argument.and_then(identifier);
            if (argument.is_some() && value.is_none()) || words.next().is_some() {
                continue;
            }
            match tag {
                Some("@feature") => tags.features.extend(value),
                Some("@file-feature") => tags.file_features.extend(value),
                Some("@feature-start") => {
                    tags.start = true;
                    tags.features.extend(value);
                }
                Some("@feature-end") => {
                    tags.end = true;
                    tags.end_feature = value;
                }
                _ => {}
            }
        }
        tags
    }
}

fn identifier(value: &str) -> Option<&str> {
    let value = value
        .strip_prefix('{')
        .and_then(|inner| inner.strip_suffix('}'))
        .unwrap_or(value);
    (!value.is_empty() && !value.contains(['{', '}']) && !value.contains(char::is_whitespace))
        .then_some(value)
}

struct Block<'a> {
    start: usize,
    features: Vec<&'a str>,
}

struct Extraction<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    annotations: BTreeSet<Annotation>,
    blocks: Vec<Block<'a>>,
}

impl<'a> Extraction<'a> {
    fn new(source: &'a str) -> Self {
        let line_starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
            .collect();
        Self {
            source,
            line_starts,
            annotations: BTreeSet::new(),
            blocks: Vec::new(),
        }
    }

    fn add(&mut self, feature: &str, symbol: Option<&str>, range: Option<Range<usize>>) {
        self.annotations.insert(Annotation {
            feature: feature.to_owned(),
            symbol: symbol.map(str::to_owned),
            lines: range.map(|range| (self.line(range.start), self.line(range.end.saturating_sub(1)))),
        });
    }

    fn line(&self, offset: usize) -> u32 {
        self.line_starts.partition_point(|start| *start <= offset) as u32
    }

    fn block_marker(&mut self, range: Range<usize>, tags: Tags<'a>) {
        if tags.start && !tags.end {
            self.blocks.push(Block {
                start: range.end,
                features: tags.features,
            });
        } else if tags.end && !tags.start
            && self.blocks.last().is_some_and(|block| {
                tags.end_feature.is_none_or(|feature| block.features.contains(&feature))
            })
            && let Some(block) = self.blocks.pop()
        {
            let content = &self.source[block.start..range.start];
            let start = block.start + content.len() - content.trim_start().len();
            let end = block.start + content.trim_end().len();
            if start < end {
                for feature in block.features {
                    self.add(feature, None, Some(start..end));
                }
            }
        }
    }
}
