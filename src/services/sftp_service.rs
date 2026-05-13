use crate::core::connection::{TransferStatus, TransferTask};
use anyhow::{Context, Result};
use ssh2::{Session, Sftp};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

/// SFTP 文件传输服务
pub struct SftpService {
    sftp: Sftp,
    transfer_queue: Arc<Mutex<Vec<TransferTask>>>,
}

impl SftpService {
    /// 创建 SFTP 服务实例
    pub fn new(session: &Session) -> Result<Self> {
        let sftp = session.sftp().context("Failed to open SFTP channel")?;
        Ok(Self {
            sftp,
            transfer_queue: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// 列出远程目录内容
    pub fn list_dir(&self, path: &str) -> Result<Vec<SftpEntry>> {
        let entries = self.sftp.readdir(Path::new(path))?;
        Ok(entries
            .into_iter()
            .map(|(p, stat)| SftpEntry {
                name: p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: p.to_string_lossy().to_string(),
                is_dir: stat.is_dir(),
                size: stat.size.unwrap_or(0),
                mtime: stat.mtime.unwrap_or(0),
            })
            .collect())
    }

    /// 上传文件
    pub fn upload_file(&self, local_path: &Path, remote_path: &Path, task_id: &str) -> Result<()> {
        let file_size = fs::metadata(local_path)
            .with_context(|| format!("Cannot read local file: {:?}", local_path))?
            .len();

        self.update_task_status(task_id, TransferStatus::InProgress, 0, file_size);

        let mut local_file = fs::File::open(local_path)?;
        let mut remote_file = self
            .sftp
            .create(remote_path)
            .with_context(|| format!("Failed to create remote file: {:?}", remote_path))?;

        let mut buffer = [0u8; 64 * 1024]; // 64KB buffer
        let mut transferred: u64 = 0;

        loop {
            let bytes_read = local_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            remote_file.write_all(&buffer[..bytes_read])?;
            transferred += bytes_read as u64;
            self.update_task_status(task_id, TransferStatus::InProgress, transferred, file_size);
        }

        remote_file.flush()?;
        self.update_task_status(task_id, TransferStatus::Completed, file_size, file_size);

        Ok(())
    }

    /// 下载文件
    pub fn download_file(
        &self,
        remote_path: &Path,
        local_path: &Path,
        task_id: &str,
    ) -> Result<()> {
        let file_size = self
            .sftp
            .stat(remote_path)
            .with_context(|| format!("Cannot stat remote file: {:?}", remote_path))?
            .size
            .unwrap_or(0);

        self.update_task_status(task_id, TransferStatus::InProgress, 0, file_size);

        // 确保本地目录存在
        if let Some(parent) = local_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut remote_file = self.sftp.open(remote_path)?;
        let mut local_file = fs::File::create(local_path)?;

        let mut buffer = [0u8; 64 * 1024];
        let mut transferred: u64 = 0;

        loop {
            let bytes_read = remote_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            local_file.write_all(&buffer[..bytes_read])?;
            transferred += bytes_read as u64;
            self.update_task_status(task_id, TransferStatus::InProgress, transferred, file_size);
        }

        local_file.flush()?;
        self.update_task_status(task_id, TransferStatus::Completed, file_size, file_size);

        Ok(())
    }

    /// 删除远程文件
    pub fn remove_file(&self, path: &Path) -> Result<()> {
        self.sftp
            .unlink(path)
            .with_context(|| format!("Failed to remove remote file: {:?}", path))
    }

    /// 创建远程目录
    pub fn create_dir(&self, path: &Path) -> Result<()> {
        self.sftp
            .mkdir(path, 0o755)
            .with_context(|| format!("Failed to create remote dir: {:?}", path))
    }

    /// 删除远程目录
    pub fn remove_dir(&self, path: &Path) -> Result<()> {
        self.sftp
            .rmdir(path)
            .with_context(|| format!("Failed to remove remote dir: {:?}", path))
    }

    /// 读取远程文件内容
    pub fn read_file(&self, path: &Path) -> Result<String> {
        let mut file = self.sftp.open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }

    /// 写入远程文件内容
    pub fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        let mut file = self
            .sftp
            .create(path)
            .with_context(|| format!("Failed to create remote file: {:?}", path))?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    /// 添加传输任务到队列
    pub fn add_task(&self, task: TransferTask) {
        if let Ok(mut queue) = self.transfer_queue.lock() {
            queue.push(task);
        }
    }

    /// 获取传输队列
    pub fn get_queue(&self) -> Vec<TransferTask> {
        self.transfer_queue
            .lock()
            .map(|q| q.clone())
            .unwrap_or_default()
    }

    /// 获取 SFTP 引用（用于创建子服务）
    pub fn sftp_ref(&self) -> &Sftp {
        &self.sftp
    }

    fn update_task_status(
        &self,
        task_id: &str,
        status: TransferStatus,
        transferred: u64,
        total: u64,
    ) {
        if let Ok(mut queue) = self.transfer_queue.lock() {
            if let Some(task) = queue.iter_mut().find(|t| t.id == task_id) {
                task.status = status;
                task.transferred_bytes = transferred;
                task.total_bytes = total;
            }
        }
    }
}

/// SFTP 目录条目
#[derive(Debug, Clone)]
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub mtime: u64,
}

impl SftpEntry {
    pub fn format_size(&self) -> String {
        if self.is_dir {
            return "<DIR>".to_string();
        }
        bytesize::ByteSize::b(self.size).to_string()
    }
}
