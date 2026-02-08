# DNS Detector

DNS Detector 是一款高效、轻量级的本地 DNS 测速与准确性分析工具。它旨在帮助用户测量不同 DNS 服务器在各种解析方法（IPv4、IPv6、DoH）下的真实性能，特别是针对中国大陆环境下的 DNS 污染情况进行直观展示。

> [!NOTE]
> 本项目由开发人员自行维护。虽然欢迎反馈错误（Issues），但目前不打算添加大规模新功能。

## 功能特性

- **多协议支持**: 支持传统的 UDP (IPv4/IPv6) 以及现代的 DNS over HTTPS (DoH)。
- **并发测量**: 支持自定义并发数（默认 16），高效利用网络带宽。
- **智能重试**: 自动重试高延迟（>1000ms）或失败的请求（最多 3 次），取最优结果。
- **详细统计**: 控制台实时显示每个 DNS 的平均、最大、最小延迟及失败次数。
- **数据导出**: 测试结果自动导出为标准 CSV 格式，方便进行二次数据分析。
- **轻量高效**: 使用 Rust 编写，基于 Tokio 异步运行时，资源占用极低。

## 快速开始

### 安装

你可以从 [Releases](https://github.com/RainPPR/dns-detector/releases) 页面下载适合你操作系统的二进制文件。

如果你已经安装了 [Rust](https://rustup.rs/)，也可以直接克隆仓库并编译：

```bash
git clone https://github.com/RainPPR/dns-detector.git
cd dns-detector
cargo build --release
```

### 使用方法

程序需要两个 JSON 配置文件：`dns_servers.json` (包含 DNS 列表) 和 `site_servers.json` (包含待测域名列表)，示例文件在 `example` 文件夹中。

```bash
./dns-detector --dns-file dns_servers.json --sites-file site_servers.json --output results.csv
```

#### 参数说明

- `-d, --dns-file`: DNS 服务器配置文件路径 (默认: `dns_servers.json`)
- `-s, --sites-file`: 目标域名配置文件路径 (默认: `site_servers.json`)
- `-o, --output`: 结果输出 CSV 路径 (默认: `results.csv`)
- `-c, --concurrency`: 并发任务数 (默认: `16`)

## 配置文件格式

### DNS 服务器 (dns_servers.json)

```json
{
    "servers": [
        {
            "name": "AliDNS",
            "ipv4": ["223.5.5.5"],
            "ipv6": ["2400:3200::1"],
            "doh": ["https://dns.alidns.com/dns-query"]
        }
    ]
}
```

### 域名列表 (site_servers.json)

```json
{
    "servers": [
        {
            "name": "Google",
            "url": ["google.com", "google.com.hk"]
        }
    ]
}
```

## 测试结果说明

- **控制台**: 显示汇总信息，包括最大、最小、平均延迟。
- **CSV 文件**: 详细列出每个域名在每个 DNS/解析方法下的具体 IP 结果和延迟。
  - 延迟为 `-1` 表示解析失败。

## 许可证

本项目基于 [MIT License](LICENSE) 开源。

---

> [!TIP]
> 此文档由 **Antigravity (人工智能)** 驱动，基于模型指令生成，并由人工验收。
