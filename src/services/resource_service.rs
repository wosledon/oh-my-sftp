use crate::core::connection::SystemResources;
use crate::services::ssh_service::SshService;
use anyhow::Result;
use ssh2::Session;

/// 远程资源采集服务
pub struct ResourceService;

impl ResourceService {
    /// 采集系统资源信息
    pub fn collect(session: &Session) -> Result<SystemResources> {
        let os_type = SshService::detect_os(session)?;
        let is_linux = os_type.contains("Linux");

        let cpu_usage = Self::collect_cpu(session, is_linux)?;
        let (mem_used, mem_total) = Self::collect_memory(session, is_linux)?;
        let (disk_used, disk_total) = Self::collect_disk(session, is_linux)?;
        let load_avg = Self::collect_loadavg(session, is_linux)?;
        let uptime = Self::collect_uptime(session, is_linux)?;

        Ok(SystemResources {
            cpu_usage,
            memory_used_mb: mem_used,
            memory_total_mb: mem_total,
            disk_used_gb: disk_used,
            disk_total_gb: disk_total,
            load_average: load_avg,
            uptime,
        })
    }

    /// 采集 CPU 使用率
    fn collect_cpu(session: &Session, is_linux: bool) -> Result<f64> {
        if is_linux {
            // 读取 /proc/stat 计算 CPU 使用率
            let stat = SshService::execute_command(
                session,
                "cat /proc/stat | grep '^cpu ' | head -1",
            )?;

            let parts: Vec<u64> = stat
                .split_whitespace()
                .skip(1)
                .filter_map(|s| s.parse().ok())
                .collect();

            if parts.len() >= 4 {
                let idle = parts[3];
                let total: u64 = parts.iter().sum();
                // 单次采样（理想情况需要两次采样计算差值）
                let usage = 100.0 * (1.0 - idle as f64 / total as f64);
                return Ok((usage * 10.0).round() / 10.0);
            }
        }

        // macOS fallback
        let output = SshService::execute_command(
            session,
            "top -l 1 | grep 'CPU usage' | awk '{print $3}' | tr -d '%'",
        )?;
        Ok(output.parse().unwrap_or(0.0))
    }

    /// 采集内存使用
    fn collect_memory(session: &Session, is_linux: bool) -> Result<(u64, u64)> {
        if is_linux {
            let output = SshService::execute_command(
                session,
                "free -m | grep '^Mem:' | awk '{print $3,$2}'",
            )?;
            let parts: Vec<u64> = output
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 2 {
                return Ok((parts[0], parts[1]));
            }
        }

        // macOS fallback
        let output = SshService::execute_command(
            session,
            "vm_stat | perl -ne '/page size of (\\d+)/ and $size=$1; /Pages free:\\s+(\\d+)/ and $free=$1; /Pages active:\\s+(\\d+)/ and $active=$1; /Pages inactive:\\s+(\\d+)/ and $inactive=$1; END { $used=$active+$inactive; printf \"%d %d\", ($used*$size)/1048576, (($used+$free)*$size)/1048576 }'",
        )?;
        let parts: Vec<u64> = output
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 2 {
            return Ok((parts[0], parts[1]));
        }

        Ok((0, 0))
    }

    /// 采集磁盘使用
    fn collect_disk(session: &Session, _is_linux: bool) -> Result<(f64, f64)> {
        let output = SshService::execute_command(session, "df -k / | tail -1 | awk '{print $3,$2}'")?;
        let parts: Vec<f64> = output
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() >= 2 {
            // 从 KB 转换为 GB
            let used_gb = (parts[0] / 1024.0 / 1024.0 * 10.0).round() / 10.0;
            let total_gb = (parts[1] / 1024.0 / 1024.0 * 10.0).round() / 10.0;
            return Ok((used_gb, total_gb));
        }

        Ok((0.0, 0.0))
    }

    /// 采集系统负载
    fn collect_loadavg(session: &Session, is_linux: bool) -> Result<[f64; 3]> {
        if is_linux {
            let output =
                SshService::execute_command(session, "cat /proc/loadavg | awk '{print $1,$2,$3}'")?;
            let parts: Vec<f64> = output
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 3 {
                return Ok([parts[0], parts[1], parts[2]]);
            }
        }

        // macOS fallback
        let output =
            SshService::execute_command(session, "sysctl -n vm.loadavg | awk '{print $2,$3,$4}'")?;
        let parts: Vec<f64> = output
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if parts.len() >= 3 {
            return Ok([parts[0], parts[1], parts[2]]);
        }

        Ok([0.0, 0.0, 0.0])
    }

    /// 采集系统运行时间
    fn collect_uptime(session: &Session, _is_linux: bool) -> Result<String> {
        let output = SshService::execute_command(session, "uptime -p")?;
        // 格式: "up 3 days, 5 hours, 23 minutes"
        Ok(output.trim_start_matches("up ").to_string())
    }
}
