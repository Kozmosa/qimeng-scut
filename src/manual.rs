use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use walkdir::WalkDir;

pub const TOP_LEVEL_SECTION_ID: &str = "__top_level__";
pub const TOP_LEVEL_SECTION_TITLE: &str = "首页 / 顶层";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualRepo {
    pub root_path: PathBuf,
    pub docs_path: PathBuf,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub id: String,
    pub title: String,
    pub root_path: PathBuf,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub title: String,
    pub relative_path: PathBuf,
    pub source_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Frontmatter {
    title: Option<String>,
}

impl ManualRepo {
    pub fn load(root_path: impl AsRef<Path>) -> Result<Self, String> {
        let root_path = root_path.as_ref().to_path_buf();
        let docs_path = validate_repo_root(&root_path)?;
        let mut sections = Vec::new();
        let mut top_level_files = Vec::new();
        let mut directory_entries = Vec::new();

        let read_dir = fs::read_dir(&docs_path)
            .map_err(|error| format!("无法读取 docs 目录 `{}`：{error}", docs_path.display()))?;

        for entry_result in read_dir {
            let entry = entry_result.map_err(|error| {
                format!(
                    "读取 docs 目录条目时失败 `{}`：{error}",
                    docs_path.display()
                )
            })?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') {
                continue;
            }

            let file_type = entry
                .file_type()
                .map_err(|error| format!("读取路径类型失败 `{}`：{error}", path.display()))?;

            if file_type.is_file() && is_markdown(&path) {
                top_level_files.push(path);
            } else if file_type.is_dir() {
                directory_entries.push((file_name, path));
            }
        }

        top_level_files.sort();
        if !top_level_files.is_empty() {
            let entries = top_level_files
                .iter()
                .map(|path| build_entry(&docs_path, &docs_path, path))
                .collect::<Result<Vec<_>, _>>()?;
            sections.push(Section {
                id: TOP_LEVEL_SECTION_ID.to_string(),
                title: TOP_LEVEL_SECTION_TITLE.to_string(),
                root_path: docs_path.clone(),
                entries,
            });
        }

        directory_entries.sort_by(|left, right| left.0.cmp(&right.0));
        for (title, path) in directory_entries {
            let entries = collect_entries(&path)?;
            if entries.is_empty() {
                continue;
            }

            sections.push(Section {
                id: title.clone(),
                title,
                root_path: path,
                entries,
            });
        }

        Ok(Self {
            root_path,
            docs_path,
            sections,
        })
    }
}

pub fn validate_repo_root(path: impl AsRef<Path>) -> Result<PathBuf, String> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|_| format!("路径不存在：`{}`。", path.display()))?;

    if !metadata.is_dir() {
        return Err(format!("路径不是目录：`{}`。", path.display()));
    }

    let docs_path = path.join("docs");
    let docs_metadata = fs::metadata(&docs_path)
        .map_err(|_| format!("路径缺少 docs/ 子目录：`{}`。", path.display()))?;
    if !docs_metadata.is_dir() {
        return Err(format!("docs 路径不是目录：`{}`。", docs_path.display()));
    }

    Ok(docs_path)
}

fn collect_entries(section_root: &Path) -> Result<Vec<Entry>, String> {
    let mut paths = WalkDir::new(section_root)
        .into_iter()
        .filter_entry(|entry| !is_hidden(entry.path(), entry.file_name()))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_markdown(entry.path()))
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();

    paths.sort();
    paths
        .into_iter()
        .map(|path| build_entry(section_root, section_root, &path))
        .collect()
}

fn build_entry(display_root: &Path, title_root: &Path, path: &Path) -> Result<Entry, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("读取 Markdown 文件失败 `{}`：{error}", path.display()))?;
    let relative_path = path
        .strip_prefix(display_root)
        .map_err(|_| format!("无法计算相对路径：`{}`。", path.display()))?
        .to_path_buf();
    let title = extract_title(&source, path);

    Ok(Entry {
        title,
        relative_path,
        source_path: title_root.join(
            path.strip_prefix(title_root)
                .map_err(|_| format!("无法计算标题相对路径：`{}`。", path.display()))?,
        ),
    })
}

fn extract_title(markdown: &str, path: &Path) -> String {
    extract_frontmatter_title(markdown)
        .or_else(|| extract_first_h1(markdown))
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_else(|| OsStr::new("untitled"))
                .to_string_lossy()
                .to_string()
        })
}

fn extract_frontmatter_title(markdown: &str) -> Option<String> {
    let (frontmatter, _) = split_frontmatter(markdown);
    let frontmatter = frontmatter?;
    let parsed = serde_yaml::from_str::<Frontmatter>(frontmatter).ok()?;
    parsed.title.map(|title| title.trim().to_string())
}

fn extract_first_h1(markdown: &str) -> Option<String> {
    let (_, body) = split_frontmatter(markdown);
    let parser = Parser::new_ext(body, Options::ENABLE_TABLES);
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

fn normalize_inline_text(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(OsStr::to_str),
        Some("md" | "markdown")
    )
}

fn is_hidden(path: &Path, name: &OsStr) -> bool {
    if path.components().count() <= 1 {
        return false;
    }

    name.to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::tempdir;

    use super::{extract_first_h1, extract_frontmatter_title, split_frontmatter, ManualRepo};

    fn fixture_root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn load_repo_indexes_top_level_and_named_sections() {
        let repo = ManualRepo::load(fixture_root("manual_repo")).expect("load repo");

        let section_titles = repo
            .sections
            .iter()
            .map(|section| section.title.as_str())
            .collect::<Vec<_>>();
        assert_eq!(section_titles, vec!["首页 / 顶层", "health", "others"]);
        assert_eq!(
            repo.sections[0]
                .entries
                .iter()
                .map(|entry| entry.title.as_str())
                .collect::<Vec<_>>(),
            vec!["首页", "入门"]
        );
    }

    #[test]
    fn load_repo_ignores_hidden_and_empty_directories() {
        let repo = ManualRepo::load(fixture_root("manual_repo")).expect("load repo");

        assert!(repo
            .sections
            .iter()
            .all(|section| section.title != ".vuepress"));
        assert!(repo.sections.iter().all(|section| section.title != "empty"));
    }

    #[test]
    fn frontmatter_title_has_highest_priority() {
        let markdown = "---\ntitle: 来自 frontmatter\n---\n# 来自标题\n";

        assert_eq!(
            extract_frontmatter_title(markdown).as_deref(),
            Some("来自 frontmatter")
        );
    }

    #[test]
    fn first_heading_falls_back_when_frontmatter_missing() {
        let markdown = "一些导语\n\n# 第一标题\n\n正文";

        assert_eq!(extract_first_h1(markdown).as_deref(), Some("第一标题"));
    }

    #[test]
    fn split_frontmatter_returns_body_without_yaml() {
        let markdown = "---\ntitle: 首页\n---\n# 标题\n";
        let (frontmatter, body) = split_frontmatter(markdown);

        assert_eq!(frontmatter, Some("title: 首页\n"));
        assert_eq!(body, "# 标题\n");
    }

    #[test]
    fn validation_rejects_missing_docs_directory() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("content")).expect("create content");

        let error = super::validate_repo_root(temp.path()).expect_err("validation should fail");
        assert!(error.contains("docs"));
    }
}
