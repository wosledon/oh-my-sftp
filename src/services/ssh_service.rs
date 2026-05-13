use crate::core::connection::{AuthMethod, Connection};
use anyhow::{Context, Result};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

/// SSH 服务：管理 SSH 连接生命周期
pub struct SshService;

#[allow(dead_code)]
impl SshService {
    /// 建立 SSH 连接
    pub fn connect(conn: &Connection, timeout_secs: u64) -> Result<(Session, TcpStream)> {
        let addr = format!("{}:{}", conn.host, conn.port);
        let timeout = Duration::from_secs(timeout_secs);

        let tcp =
            TcpStream::connect_timeout(&addr.parse().context("Invalid socket address")?, timeout)
                .with_context(|| format!("Failed to connect to {}", addr))?;

        tcp.set_read_timeout(Some(timeout))?;
        tcp.set_write_timeout(Some(timeout))?;

        let mut session = Session::new().context("Failed to create SSH session")?;

        session.set_tcp_stream(tcp.try_clone()?);
        session.handshake()?;

        // 认证
        Self::authenticate(&session, conn)?;

        Ok((session, tcp))
    }

    /// SSH 认证
    fn authenticate(session: &Session, conn: &Connection) -> Result<()> {
        match &conn.auth_method {
            AuthMethod::Password(encoded_pwd) => {
                // 简单 base64 解码（生产环境应使用 OS keyring）
                let password = Self::decode_password(encoded_pwd)?;
                session
                    .userauth_password(&conn.username, &password)
                    .with_context(|| format!("Password auth failed for {}", conn.username))?;
            }
            AuthMethod::KeyFile(key_path) => {
                session
                    .userauth_pubkey_file(&conn.username, None, key_path, None)
                    .with_context(|| {
                        format!(
                            "Key auth failed for {} with key {:?}",
                            conn.username, key_path
                        )
                    })?;
            }
            AuthMethod::Agent => {
                // 尝试 agent 认证
                let mut agent = session.agent().context("SSH agent not available")?;
                agent.connect()?;
                agent.list_identities()?;

                let identities = agent.identities()?;
                let mut authenticated = false;

                for identity in identities {
                    if agent.userauth(&conn.username, &identity).is_ok() {
                        authenticated = true;
                        break;
                    }
                }

                if !authenticated {
                    anyhow::bail!("Agent auth failed: no valid identity found");
                }
            }
        }

        // 验证认证成功
        if !session.authenticated() {
            anyhow::bail!("Authentication failed");
        }

        Ok(())
    }

    /// 在远程服务器上执行命令并返回 stdout
    pub fn execute_command(session: &Session, cmd: &str) -> Result<String> {
        let mut channel = session.channel_session()?;
        channel.exec(cmd)?;

        let mut output = String::new();
        channel.read_to_string(&mut output)?;

        channel.wait_close()?;
        Ok(output.trim().to_string())
    }

    /// 检查连接是否存活
    pub fn is_alive(session: &Session) -> bool {
        // 尝试执行一个轻量命令
        Self::execute_command(session, "echo ok").is_ok()
    }

    /// 探测远程 OS 类型
    pub fn detect_os(session: &Session) -> Result<String> {
        let uname = Self::execute_command(session, "uname -s")?;
        Ok(uname)
    }

    /// 创建 SFTP 通道
    pub fn create_sftp(session: &Session) -> Result<ssh2::Sftp> {
        session.sftp().context("Failed to create SFTP channel")
    }

    /// Base64 解码密码
    fn decode_password(encoded: &str) -> Result<String> {
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded.as_bytes())
            .context("Failed to decode base64 password")?;
        String::from_utf8(decoded).context("Invalid UTF-8 in decoded password")
    }
}
