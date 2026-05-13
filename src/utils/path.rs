use std::path::{Path, PathBuf};

/// 展开 home 目录中的 ~ 符号
#[allow(dead_code)]
pub fn expand_home(path: &str) -> PathBuf {
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home;
            }
            if path.starts_with("~/") {
                return home.join(&path[2..]);
            }
        }
    }
    PathBuf::from(path)
}

/// 缩短路径显示（显示最后 N 级 + 前缀 ...）
#[allow(dead_code)]
pub fn shorten_path(path: &Path, max_levels: usize) -> String {
    let path_str = path.to_string_lossy();
    let components: Vec<&str> = path_str
        .split(std::path::MAIN_SEPARATOR)
        .filter(|s| !s.is_empty())
        .collect();

    if components.len() <= max_levels {
        return path.to_string_lossy().to_string();
    }

    let truncated: Vec<&str> = components
        .iter()
        .skip(components.len() - max_levels)
        .copied()
        .collect();

    format!("...{}", truncated.join(&std::path::MAIN_SEPARATOR.to_string()))
}

/// 获取文件名，如果路径是目录则在末尾加 /
#[allow(dead_code)]
pub fn display_name(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    if path.is_dir() {
        format!("{}/", name)
    } else {
        name
    }
}
