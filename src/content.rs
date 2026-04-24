use std::{ffi::OsStr, path::Path};

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::text::Text;
use serde::Deserialize;
use textwrap::{wrap, Options as WrapOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentContent {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        text: String,
    },
    Paragraph(String),
    List(Vec<ListItem>),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Quote(String),
    Rule,
    Placeholder(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub depth: usize,
    pub ordered: bool,
    pub index: Option<u64>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRenderCache {
    pub width: usize,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RichContentRenderCache {
    pub width: usize,
    pub text: Text<'static>,
}

#[derive(Debug, Deserialize)]
struct Frontmatter {
    title: Option<String>,
}

#[derive(Debug)]
enum InlineContainer {
    Paragraph(String),
    Heading(String),
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    Quote(String),
    Item {
        depth: usize,
        ordered: bool,
        index: Option<u64>,
        text: String,
    },
    Link {
        destination: String,
        text: String,
    },
    Image {
        destination: String,
        alt: String,
    },
}

#[derive(Debug)]
struct ListContext {
    ordered: bool,
    next_index: u64,
    items: Vec<ListItem>,
}

impl DocumentContent {
    pub fn parse(markdown: &str) -> Self {
        let (_, body) = split_frontmatter(markdown);
        let mut parser = MarkdownParser::new();
        parser.parse(body);
        Self {
            blocks: parser.finish(),
        }
    }

    pub fn render_lines(&self, width: usize) -> Vec<String> {
        let width = width.max(12);
        let mut lines: Vec<String> = Vec::new();

        for (index, block) in self.blocks.iter().enumerate() {
            if index > 0 && !matches!(lines.last(), Some(line) if line.is_empty()) {
                lines.push(String::new());
            }

            match block {
                Block::Heading { level, text } => {
                    let prefix = format!("{} ", "#".repeat((*level).clamp(1, 3) as usize));
                    push_wrapped_lines(&mut lines, text, width, &prefix, "");
                }
                Block::Paragraph(text) => {
                    push_wrapped_lines(&mut lines, text, width, "", "");
                }
                Block::List(items) => {
                    for item in items {
                        let indent = "  ".repeat(item.depth.saturating_sub(1));
                        let marker = if item.ordered {
                            format!("{}. ", item.index.unwrap_or(1))
                        } else {
                            "- ".to_string()
                        };
                        let initial = format!("{indent}{marker}");
                        let subsequent = format!("{indent}{}", " ".repeat(marker.chars().count()));
                        push_wrapped_lines(&mut lines, &item.text, width, &initial, &subsequent);
                    }
                }
                Block::CodeBlock { language, code } => {
                    if let Some(language) = language.as_deref().filter(|value| !value.is_empty()) {
                        lines.push(format!("[code: {language}]"));
                    } else {
                        lines.push("[code]".to_string());
                    }

                    for code_line in code.lines() {
                        if code_line.is_empty() {
                            lines.push(String::new());
                            continue;
                        }
                        push_wrapped_lines(&mut lines, code_line, width, "    ", "    ");
                    }
                }
                Block::Quote(text) => {
                    push_wrapped_lines(&mut lines, text, width, "> ", "> ");
                }
                Block::Rule => lines.push("─".repeat(width.max(3))),
                Block::Placeholder(text) => {
                    push_wrapped_lines(&mut lines, &format!("[{text}]"), width, "", "");
                }
            }
        }

        if lines.is_empty() {
            lines.push("No content".to_string());
        }

        lines
    }
}

impl ContentRenderCache {
    pub fn new(content: &DocumentContent, width: usize) -> Self {
        Self {
            width,
            lines: content.render_lines(width),
        }
    }
}

impl RichContentRenderCache {
    pub fn new(source: &str, width: usize) -> Self {
        let parsed = tui_markdown::from_str(source);
        let lines: Vec<ratatui::text::Line<'static>> = parsed
            .lines
            .into_iter()
            .map(|line| {
                let spans: Vec<ratatui::text::Span<'static>> = line
                    .spans
                    .into_iter()
                    .map(|span| {
                        ratatui::text::Span::styled(
                            span.content.to_string(),
                            span.style,
                        )
                    })
                    .collect();
                ratatui::text::Line::from(spans).style(line.style)
            })
            .collect();
        let text = Text::from(lines);
        Self { width, text }
    }
}

pub fn resolve_title(markdown: &str, path: &Path) -> String {
    extract_frontmatter_title(markdown)
        .or_else(|| extract_first_h1(markdown))
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_else(|| OsStr::new("untitled"))
                .to_string_lossy()
                .to_string()
        })
}

pub(crate) fn split_frontmatter(markdown: &str) -> (Option<&str>, &str) {
    let mut lines = markdown.split_inclusive('\n');
    let Some(first_line) = lines.next() else {
        return (None, markdown);
    };
    if first_line.trim_end_matches(['\r', '\n']) != "---" {
        return (None, markdown);
    }

    let mut offset = first_line.len();
    for line in lines {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let frontmatter = &markdown[first_line.len()..offset];
            offset += line.len();
            return (Some(frontmatter), &markdown[offset..]);
        }
        offset += line.len();
    }

    (None, markdown)
}

fn extract_frontmatter_title(markdown: &str) -> Option<String> {
    let (frontmatter, _) = split_frontmatter(markdown);
    let frontmatter = frontmatter?;
    let parsed = serde_yaml::from_str::<Frontmatter>(frontmatter).ok()?;
    parsed.title.map(|title| title.trim().to_string())
}

fn extract_first_h1(markdown: &str) -> Option<String> {
    let (_, body) = split_frontmatter(markdown);
    let parser = Parser::new_ext(body, parse_options());
    let mut in_h1 = false;
    let mut title = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) if level == HeadingLevel::H1 => {
                in_h1 = true;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) if in_h1 => {
                let normalized = normalize_inline_text(&title);
                if !normalized.is_empty() {
                    return Some(normalized);
                }
                in_h1 = false;
            }
            Event::Text(text) | Event::Code(text) if in_h1 => title.push_str(&text),
            Event::SoftBreak | Event::HardBreak if in_h1 => title.push(' '),
            _ => {}
        }
    }

    None
}

fn parse_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    options
}

fn normalize_inline_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn push_wrapped_lines(
    lines: &mut Vec<String>,
    text: &str,
    width: usize,
    initial_indent: &str,
    subsequent_indent: &str,
) {
    let effective_width = width.max(initial_indent.chars().count() + 2);
    let options = WrapOptions::new(effective_width)
        .initial_indent(initial_indent)
        .subsequent_indent(subsequent_indent)
        .break_words(false);
    let wrapped = wrap(text, options);

    if wrapped.is_empty() {
        lines.push(initial_indent.trim_end().to_string());
        return;
    }

    for line in wrapped {
        lines.push(line.into_owned());
    }
}

#[derive(Debug, Default)]
struct MarkdownParser {
    blocks: Vec<Block>,
    containers: Vec<InlineContainer>,
    list_stack: Vec<ListContext>,
    suppress_table_depth: usize,
}

impl MarkdownParser {
    fn new() -> Self {
        Self::default()
    }

    fn parse(&mut self, markdown: &str) {
        let parser = Parser::new_ext(markdown, parse_options());
        for event in parser {
            self.handle_event(event);
        }
    }

    fn finish(self) -> Vec<Block> {
        self.blocks
    }

    fn handle_event(&mut self, event: Event<'_>) {
        if self.suppress_table_depth > 0 {
            match event {
                Event::Start(Tag::Table(_)) => self.suppress_table_depth += 1,
                Event::End(TagEnd::Table) => {
                    self.suppress_table_depth -= 1;
                    if self.suppress_table_depth == 0 {
                        self.blocks
                            .push(Block::Placeholder("Table placeholder".to_string()));
                    }
                }
                _ => {}
            }
            return;
        }

        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag_end) => self.end_tag(tag_end),
            Event::Text(text) => self.push_inline(&text),
            Event::Code(text) => self.push_inline(&text),
            Event::SoftBreak | Event::HardBreak => self.push_inline(" "),
            Event::Rule => self.blocks.push(Block::Rule),
            Event::Html(_) | Event::InlineHtml(_) => self.push_unsupported_inline(),
            Event::InlineMath(text) | Event::DisplayMath(text) => self.push_inline(&text),
            Event::TaskListMarker(checked) => {
                self.push_inline(if checked { "[x] " } else { "[ ] " });
            }
            Event::FootnoteReference(text) => self.push_inline(&format!("[^{text}]")),
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph if !self.is_inside_text_collector() => {
                self.containers
                    .push(InlineContainer::Paragraph(String::new()));
            }
            Tag::Heading { .. } if !self.is_inside_text_collector() => {
                self.containers
                    .push(InlineContainer::Heading(String::new()));
            }
            Tag::BlockQuote(_) => {
                self.containers.push(InlineContainer::Quote(String::new()));
            }
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Indented => None,
                    CodeBlockKind::Fenced(language) => {
                        let language = language.trim().to_string();
                        if language.is_empty() {
                            None
                        } else {
                            Some(language)
                        }
                    }
                };
                self.containers.push(InlineContainer::CodeBlock {
                    language,
                    code: String::new(),
                });
            }
            Tag::List(start) => {
                self.list_stack.push(ListContext {
                    ordered: start.is_some(),
                    next_index: start.unwrap_or(1),
                    items: Vec::new(),
                });
            }
            Tag::Item => {
                if let Some(list) = self.list_stack.last() {
                    self.containers.push(InlineContainer::Item {
                        depth: self.list_stack.len(),
                        ordered: list.ordered,
                        index: list.ordered.then_some(list.next_index),
                        text: String::new(),
                    });
                }
            }
            Tag::Link { dest_url, .. } => {
                self.containers.push(InlineContainer::Link {
                    destination: dest_url.to_string(),
                    text: String::new(),
                });
            }
            Tag::Image { dest_url, .. } => {
                self.containers.push(InlineContainer::Image {
                    destination: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            Tag::Table(_) => self.suppress_table_depth = 1,
            _ => {}
        }
    }

    fn end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Paragraph => {
                if matches!(self.containers.last(), Some(InlineContainer::Paragraph(_))) {
                    let Some(InlineContainer::Paragraph(text)) = self.pop_container() else {
                        return;
                    };
                    let text = normalize_inline_text(&text);
                    if !text.is_empty() {
                        self.blocks.push(Block::Paragraph(text));
                    }
                }
            }
            TagEnd::Heading(level) => {
                if matches!(self.containers.last(), Some(InlineContainer::Heading(_))) {
                    let Some(InlineContainer::Heading(text)) = self.pop_container() else {
                        return;
                    };
                    let text = normalize_inline_text(&text);
                    if !text.is_empty() {
                        self.blocks.push(Block::Heading {
                            level: heading_level(level),
                            text,
                        });
                    }
                }
            }
            TagEnd::BlockQuote(_) => {
                if matches!(self.containers.last(), Some(InlineContainer::Quote(_))) {
                    let Some(InlineContainer::Quote(text)) = self.pop_container() else {
                        return;
                    };
                    let text = normalize_inline_text(&text);
                    if !text.is_empty() {
                        self.blocks.push(Block::Quote(text));
                    }
                }
            }
            TagEnd::CodeBlock => {
                if let Some(InlineContainer::CodeBlock { language, code }) = self.pop_container() {
                    self.blocks.push(Block::CodeBlock { language, code });
                }
            }
            TagEnd::Item => {
                if let Some(InlineContainer::Item {
                    depth,
                    ordered,
                    index,
                    text,
                }) = self.pop_container()
                {
                    if let Some(list) = self.list_stack.last_mut() {
                        let text = normalize_inline_text(&text);
                        if !text.is_empty() {
                            list.items.push(ListItem {
                                depth,
                                ordered,
                                index,
                                text,
                            });
                        }
                        if ordered {
                            list.next_index += 1;
                        }
                    }
                }
            }
            TagEnd::List(_) => {
                if let Some(list) = self.list_stack.pop() {
                    if !list.items.is_empty() {
                        self.blocks.push(Block::List(list.items));
                    }
                }
            }
            TagEnd::Link => {
                if let Some(InlineContainer::Link { destination, text }) = self.pop_container() {
                    let text = normalize_inline_text(&text);
                    let rendered = if text.is_empty() {
                        destination
                    } else {
                        format!("{text} ({destination})")
                    };
                    self.push_inline(&rendered);
                }
            }
            TagEnd::Image => {
                if let Some(InlineContainer::Image { destination, alt }) = self.pop_container() {
                    let alt = normalize_inline_text(&alt);
                    let placeholder = if alt.is_empty() {
                        format!(
                            "[Image placeholder: {}]",
                            filename_from_destination(&destination)
                        )
                    } else {
                        format!("[Image: {alt}]")
                    };
                    self.push_inline(&placeholder);
                }
            }
            _ => {}
        }
    }

    fn push_inline(&mut self, text: &str) {
        for container in self.containers.iter_mut().rev() {
            match container {
                InlineContainer::Paragraph(buffer)
                | InlineContainer::Quote(buffer)
                | InlineContainer::Item { text: buffer, .. }
                | InlineContainer::Heading(buffer)
                | InlineContainer::Link { text: buffer, .. }
                | InlineContainer::Image { alt: buffer, .. } => {
                    buffer.push_str(text);
                    return;
                }
                InlineContainer::CodeBlock { code, .. } => {
                    code.push_str(text);
                    return;
                }
            }
        }
    }

    fn push_unsupported_inline(&mut self) {
        let placeholder = "[Unsupported content placeholder]";
        if self.is_inside_text_collector() {
            self.push_inline(placeholder);
        } else {
            self.blocks.push(Block::Placeholder(
                "Unsupported content placeholder".to_string(),
            ));
        }
    }

    fn pop_container(&mut self) -> Option<InlineContainer> {
        self.containers.pop()
    }

    fn is_inside_text_collector(&self) -> bool {
        self.containers.iter().rev().any(|container| {
            matches!(
                container,
                InlineContainer::Quote(_)
                    | InlineContainer::Item { .. }
                    | InlineContainer::Link { .. }
                    | InlineContainer::Image { .. }
                    | InlineContainer::CodeBlock { .. }
            )
        })
    }
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn filename_from_destination(destination: &str) -> String {
    destination
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(destination)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{resolve_title, Block, ContentRenderCache, DocumentContent, ListItem};

    #[test]
    fn resolves_title_from_frontmatter_then_heading_then_filename() {
        assert_eq!(
            resolve_title("---\ntitle: 首页\n---\n# 标题\n", Path::new("README.md")),
            "首页"
        );
        assert_eq!(
            resolve_title("# 第一标题\n内容\n", Path::new("guide.md")),
            "第一标题"
        );
        assert_eq!(resolve_title("没有标题\n", Path::new("guide.md")), "guide");
    }

    #[test]
    fn parses_markdown_into_text_blocks() {
        let document = DocumentContent::parse(
            "# 标题\n\n一段正文，带有[链接](https://example.com)。\n\n- 第一项\n- 第二项\n\n> 引用内容\n\n---\n",
        );

        assert_eq!(
            document.blocks,
            vec![
                Block::Heading {
                    level: 1,
                    text: "标题".to_string(),
                },
                Block::Paragraph("一段正文，带有链接 (https://example.com)。".to_string()),
                Block::List(vec![
                    ListItem {
                        depth: 1,
                        ordered: false,
                        index: None,
                        text: "第一项".to_string(),
                    },
                    ListItem {
                        depth: 1,
                        ordered: false,
                        index: None,
                        text: "第二项".to_string(),
                    },
                ]),
                Block::Quote("引用内容".to_string()),
                Block::Rule,
            ]
        );
    }

    #[test]
    fn renders_images_tables_and_components_as_placeholders() {
        let document = DocumentContent::parse(
            "![校园](image.png)\n\n<AppLanding />\n\n| A | B |\n| - | - |\n| 1 | 2 |\n",
        );

        assert_eq!(
            document.blocks,
            vec![
                Block::Paragraph("[Image: 校园]".to_string()),
                Block::Placeholder("Unsupported content placeholder".to_string()),
                Block::Placeholder("Table placeholder".to_string()),
            ]
        );
    }

    #[test]
    fn parses_code_blocks_and_builds_wrapped_render_cache() {
        let document = DocumentContent::parse(
            "```bash\ncargo run --bin qimeng-scut -- --manual-path /tmp/really/long/path\n```\n",
        );
        let cache = ContentRenderCache::new(&document, 24);

        assert_eq!(
            document.blocks,
            vec![Block::CodeBlock {
                language: Some("bash".to_string()),
                code: "cargo run --bin qimeng-scut -- --manual-path /tmp/really/long/path\n"
                    .to_string(),
            }],
        );
        assert!(cache.lines.iter().any(|line| line.contains("[code: bash]")));
        assert!(cache.lines.len() >= 3);
    }
}
