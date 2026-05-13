use super::connection::SshConfigEntry;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// 解析 ~/.ssh/config 文件，提取所有 Host 条目
pub fn parse_ssh_config() -> Result<Vec<SshConfigEntry>> {
    let ssh_dir = dirs::home_dir()
        .context("Cannot find home directory")?
        .join(".ssh");

    let config_path = ssh_dir.join("config");

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read SSH config: {:?}", config_path))?;

    parse_ssh_config_content(&content, &ssh_dir)
}

/// 解析 SSH config 文本内容
fn parse_ssh_config_content(content: &str, ssh_dir: &PathBuf) -> Result<Vec<SshConfigEntry>> {
    let mut entries: Vec<SshConfigEntry> = Vec::new();
    let mut current: Option<SshConfigEntry> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        // 跳过注释和空行
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // 分割关键字和值
        let parts: Vec<&str> = line.splitn(2, |c: char| c == ' ' || c == '\t').collect();
        if parts.len() < 2 {
            continue;
        }

        let keyword = parts[0].to_lowercase();
        let value = parts[1].trim();

        // 去除行尾注释（简单处理：引号内的 # 保留）
        let value = if value.starts_with('"') {
            // 引号内的值保持原样
            value.trim_matches('"')
        } else {
            // 去除可能的行尾注释
            match value.find(" #") {
                Some(pos) => &value[..pos],
                None => value,
            }
        };

        match keyword.as_str() {
            "host" => {
                // 保存上一个条目
                if let Some(entry) = current.take() {
                    entries.push(entry);
                }
                // 开始新条目，跳过通配符 Host
                if !value.contains('*') {
                    current = Some(SshConfigEntry {
                        host_alias: value.to_string(),
                        ..Default::default()
                    });
                }
            }
            "hostname" => {
                if let Some(ref mut entry) = current {
                    entry.hostname = Some(value.to_string());
                }
            }
            "port" => {
                if let Some(ref mut entry) = current {
                    entry.port = value.parse().ok();
                }
            }
            "user" => {
                if let Some(ref mut entry) = current {
                    entry.user = Some(value.to_string());
                }
            }
            "identityfile" => {
                if let Some(ref mut entry) = current {
                    // 处理 ~ 路径展开
                    let path = expand_tilde(value, ssh_dir);
                    entry.identity_file = Some(path);
                }
            }
            "proxyjump" => {
                if let Some(ref mut entry) = current {
                    entry.proxy_jump = Some(value.to_string());
                }
            }
            "forwardagent" => {
                if let Some(ref mut entry) = current {
                    entry.forward_agent = value.eq_ignore_ascii_case("yes");
                }
            }
            "include" => {
                // 处理 Include 指令：展开路径并递归解析
                let include_pattern = expand_tilde(value, ssh_dir);
                if let Ok(included) = parse_included_files(&include_pattern) {
                    entries.extend(included);
                }
            }
            _ => {
                // 忽略不支持的关键字
            }
        }
    }

    // 保存最后一个条目
    if let Some(entry) = current.take() {
        entries.push(entry);
    }

    Ok(entries)
}

/// 展开路径中的 ~ 符号
fn expand_tilde(path: &str, ssh_dir: &PathBuf) -> PathBuf {
    if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_default();
        if path == "~" {
            home
        } else if path.starts_with("~/") {
            home.join(&path[2..])
        } else {
            // ~user 格式，简化处理
            PathBuf::from(path)
        }
    } else if std::path::Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        // 相对路径，相对于 .ssh 目录
        ssh_dir.join(path)
    }
}

/// 解析 Include 指令引入的文件
fn parse_included_files(pattern: &PathBuf) -> Result<Vec<SshConfigEntry>> {
    let mut entries = Vec::new();

    let pattern_str = pattern.to_string_lossy().to_string();
    let paths = glob::glob(&pattern_str)?;

    for path in paths.flatten() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            // Include 文件中的 Host 条目，使用文件所在目录解析相对路径
            let parent = path.parent().unwrap_or(std::path::Path::new("."));
            let sub_entries = parse_ssh_config_content(&content, &parent.to_path_buf())?;
            entries.extend(sub_entries);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_host() {
        let content = r#"
Host myserver
    HostName 192.168.1.100
    User admin
    Port 2222
    IdentityFile ~/.ssh/id_rsa
"#;
        let entries = parse_ssh_config_content(content, &PathBuf::from("/tmp/.ssh")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "myserver");
        assert_eq!(entries[0].hostname, Some("192.168.1.100".to_string()));
        assert_eq!(entries[0].user, Some("admin".to_string()));
        assert_eq!(entries[0].port, Some(2222));
        assert!(entries[0].identity_file.is_some());
    }

    #[test]
    fn test_skip_wildcard_host() {
        let content = r#"
Host *
    ForwardAgent yes

Host production
    HostName prod.example.com
    User deploy
"#;
        let entries = parse_ssh_config_content(content, &PathBuf::from("/tmp/.ssh")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].host_alias, "production");
    }

    #[test]
    fn test_multiple_hosts() {
        let content = r#"
Host server1
    HostName 10.0.0.1
    User root

Host server2
    HostName 10.0.0.2
    User admin
    Port 2222
"#;
        let entries = parse_ssh_config_content(content, &PathBuf::from("/tmp/.ssh")).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
