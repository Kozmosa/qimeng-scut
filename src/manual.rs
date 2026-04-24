use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::content::resolve_title;

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
                .map(|path| build_entry(&docs_path, path))
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
        .map(|path| build_entry(section_root, &path))
        .collect()
}

fn build_entry(display_root: &Path, path: &Path) -> Result<Entry, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("读取 Markdown 文件失败 `{}`：{error}", path.display()))?;
    let relative_path = path
        .strip_prefix(display_root)
        .map_err(|_| format!("无法计算相对路径：`{}`。", path.display()))?
        .to_path_buf();
    let title = resolve_title(&source, path);

    Ok(Entry {
        title,
        relative_path,
        source_path: path.to_path_buf(),
    })
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

    use super::ManualRepo;

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
    fn validation_rejects_missing_docs_directory() {
        let temp = tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("content")).expect("create content");

        let error = super::validate_repo_root(temp.path()).expect_err("validation should fail");
        assert!(error.contains("docs"));
    }
}
