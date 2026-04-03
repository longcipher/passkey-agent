这是一份为你量身定制的 **`passkey-agent`** 项目设计文档。该方案采用全 Rust 栈编写，深度结合了现代密码学（FIDO2 `hmac-secret`、Argon2id、XChaCha20-Poly1305）、安全的 C/S 架构设计，以及内存级的安全防护。

这是一份达到 **“企业级零信任”与“开源极客美学”** 标准的设计蓝图。

---

# 🚀 项目蓝图：`passkey-agent`

**定位**：一个下一代、无密码优先（Passwordless-first）的本地凭据代理与 SOPS 运行环境。
**核心理念**：生物识别解锁、私钥零落地、内存自动销毁、防侧信道攻击。

## 🏗 一、 架构设计 (Client-Server 架构)

整个项目作为一个 Rust Workspace，编译产出两个二进制文件（或由一个可执行文件通过子命令派生）。

### 1. 核心组件交互图

```text[ 用户 / Terminal ]
       |  (CLI 命令)
       V
+-------------------+      (Unix Domain Socket)     +-------------------+
|                   |  <--- 1. 状态查询/挑战 --->   |                   |
|   passkey-cli     |                               | passkey-daemon    |
|   (前端交互进程)    |  ---- 2. 发送解密密钥 ---->   |   (后台守护进程)    |
|                   |                               |                   |
+--------+----------+  <--- 3. 下发明文 age key --  +---------+---------+
         |                                                    |
         | (4. 注入 ENV 环境变量)                               | (维护内存状态)
         V                                                    V
+-------------------+                               +-------------------+
|  execvp("sops")   |                               |  Memory / State   |
|  顶替 CLI 进程运行  |                               |  [ 15 mins TTL ]  |
+-------------------+                               |  [ Zeroize 销毁 ]  |
                                                    +-------------------+
```

### 2. 守护进程 `passkey-daemon` (Agent)

* **通信**：监听 `~/.cache/passkey-agent/agent.sock`。
* **状态机**：维护 `Locked`（锁定）和 `Unlocked`（解锁）两种状态。
* **内存保护**：明文 `age` 密钥使用 Rust 的 `secrecy` 库包裹，防止被 swap 到硬盘；定时器到期后调用 `zeroize` 将内存强制清零（写 0 覆盖）。
* **静默运行**：无 UI，完全受控于 CLI 的 IPC（进程间通信）指令。

### 3. 客户端进程 `passkey-cli` (CLI)

* **职责**：处理用户的命令行交互，唤起操作系统的 Passkey 弹窗（Touch ID / Windows Hello），收集密码，发起 RPC 请求。
* **进程替换**：拿到 `age` 密钥后，将其设为环境变量 `SOPS_AGE_KEY`，并调用底层 `execvp` 执行 `sops`。这会使得 `sops` 直接覆盖当前 CLI 的内存空间，CLI 进程生命周期结束，**不留任何内存残影**。

---

## 🔐 二、 密码学与工作流设计

为了支持 **Passkey（主）+ Password（备）** 双轨解锁，同时保证 `age` 私钥不落地，我们需要引入 **DEK（数据加密密钥）** 和 **KEK（密钥加密密钥）** 机制。

### 0. 密钥层次结构 (Key Hierarchy)

1. **DEK (Data Encryption Key)**：一个随机生成的 32 字节对称密钥。用它来加密真正的 `age` 私钥。
2. **KEK_1 (Passkey KEK)**：通过 FIDO2 `hmac-secret` 扩展生成的 32 字节对称密钥。
3. **KEK_2 (Password KEK)**：通过 `Argon2id` 从用户主密码派生出的 32 字节对称密钥。
*(我们将 DEK 分别用 KEK_1 和 KEK_2 加密，保存两份 ciphertext。任何一种方式解锁 KEK，就能解开 DEK，进而解开 `age` 私钥)*。

### 1. 初始化流程 (`pk init`)

1. **生成 `age` 密钥**：CLI 调用 `age` 库生成一对公私钥。
2. **生成 DEK**：CLI 使用 `OsRng` 随机生成 32 字节的 DEK。
3. **设置 Passkey (主路由)**：
   * CLI 随机生成 32 字节 `Salt`。
   * 唤起硬件生成 Passkey 凭据，传入 `hmac-secret` 参数及 `Salt`。
   * 获取硬件返回的 32 字节 `KEK_1`。
   * 使用 `XChaCha20-Poly1305` 和 `KEK_1` 加密 DEK，得到 `Encrypted_DEK_Passkey`。
4. **设置 Fallback (备用路由)**：
   * CLI 提示用户输入高强度主密码。
   * 生成随机 `Password_Salt`，使用 `Argon2id` 派生出 32 字节 `KEK_2`。
   * 使用 `XChaCha20-Poly1305` 和 `KEK_2` 加密 DEK，得到 `Encrypted_DEK_Password`。
5. **持久化**：使用 DEK 加密 `age` 私钥得到 `Encrypted_Age_Key`。将所有 Metadata（Salts, Credential_ID, 密文）写入 `~/.config/passkey-agent/config.json`。

### 2. 日常使用流程 (`pk run -- sops edit secrets.yaml`)

1. **检查 Agent**：CLI 通过 Socket 问 Daemon：“你有私钥吗？”
2. **若 Agent 未解锁**：
   * CLI 读取配置文件，优先尝试 Passkey 解锁。
   * 向系统发起 FIDO2 断言请求（带上 `Salt`）。
   * 指纹验证通过，拿到 `KEK_1`。
   * 解密 `Encrypted_DEK_Passkey` 得到 DEK。
   * 解密 `Encrypted_Age_Key` 得到 `age` 明文私钥。
   * 通过 IPC Socket 将明文私钥安全送入 Daemon 的内存。
3. **运行 SOPS**：Daemon 启动 15 分钟倒计时。CLI 从 Daemon 获取密钥，通过 `execvp("sops", ...)` 启动 SOPS。

---

## 💻 三、 Rust 工程实现指南

### 1. 推荐的 crate 生态依赖 (Tech Stack)

* **FIDO2 / Passkey 交互**：`authenticator` (Mozilla 维护，支持跨平台及 hmac-secret) 或 `libfido2-sys` (C 库的 Rust 绑定，控制力最强)。
* **密码学原语**：`chacha20poly1305`, `argon2`, `rand_core` (RustCrypto 官方实现)。
* **内存安全**：`secrecy` (包装敏感数据防打印、防 dump)、`zeroize` (Trait，用于安全擦除内存)。
* **IPC 与 守护进程**：`tokio` (使用 `UnixStream` 做本地通信), `interprocess` (轻量级 IPC), `daemonize` (进程脱离终端变为后台服务)。
* **CLI 交互与美化**：`clap` (参数解析), `inquire` (精美的终端提示符，处理密码输入), `indicatif` (进度条)。

### 2. 优雅的项目目录结构

```text
passkey-agent/
├── Cargo.toml                  # Workspace 配置
├── crates/
│   ├── pk-core/                # 共享库 (无状态)
│   │   ├── src/
│   │   │   ├── crypto.rs       # XChaCha20, Argon2id 实现
│   │   │   ├── fido.rs         # 封装 Passkey (hmac-secret) 逻辑
│   │   │   ├── ipc.rs          # 定义 C/S 通信协议 (Protobuf/Bincode)
│   │   │   └── config.rs       # 配置文件序列化
│   │   └── Cargo.toml
│   ├── pk-daemon/              # 守护进程
│   │   ├── src/
│   │   │   ├── main.rs         # 守护进程入口
│   │   │   ├── server.rs       # Unix Socket 监听
│   │   │   └── state.rs        # 内存状态机与 15 分钟 Zeroize 倒计时
│   │   └── Cargo.toml
│   └── pk-cli/                 # 用户侧前端工具
│       ├── src/
│       │   ├── main.rs         # CLI 入口
│       │   ├── commands/       # init, status, lock, run
│       │   └── exec.rs         # unix execvp 的封装
│       └── Cargo.toml
```

### 3. 核心代码片段示例：内存安全保障

在 Daemon 中，永远不要将私钥存放在 `String` 中，必须使用 `secrecy` 和 `zeroize`。

```rust
use secrecy::{SecretString, ExposeSecret};
use zeroize::Zeroize;

/// Agent 内存中的核心状态
pub struct AgentState {
    // SecretString 会在 Drop 时自动调用 Zeroize 清空内存
    // 并且会阻止意外的 println!("{:?}", state) 泄露私钥
    age_private_key: Option<SecretString>,
    unlock_time: Option<std::time::Instant>,
}

impl AgentState {
    pub fn unlock(&mut self, key: String) {
        self.age_private_key = Some(SecretString::new(key));
        self.unlock_time = Some(std::time::Instant::now());
    }

    pub fn lock(&mut self) {
        // 显式丢弃变量，Rust 的 Drop 机制会调用 secrecy 的内存擦除逻辑
        self.age_private_key = None; 
        self.unlock_time = None;
        println!("Agent memory securely zeroized.");
    }
}
```

### 4. 核心代码片段示例：安全的进程顶替 (CLI)

```rust
use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn execute_sops_and_exit(sops_args: &[String], age_key: &str) -> ! {
    let mut cmd = Command::new("sops");

    // 设置环境变量
    cmd.env("SOPS_AGE_KEY", age_key);
    cmd.args(sops_args);

    // 调用底层 execvp。
    // 这行代码执行后，当前的 Rust CLI 进程将被操作系统销毁，
    // sops 进程将继承当前的 PID 和环境变量。
    // 因此 CLI 内存中的明文 age_key 残留会被 OS 回收，不留痕迹。
    let err = cmd.exec(); 

    // 只有 sops 找不到时才会执行到这里
    panic!("Failed to execute sops: {}", err);
}
```

---

## 🎨 四、 优雅的终端命令设计 (UX)

为了让工具像商业软件一样好用，CLI 应该设计得极简且直观：

* **`pk init`**
    引导式初始化：自动生成 age 密钥，弹窗按指纹绑定 Passkey，提示设置备用密码。
* **`pk status`**
    输出当前 Daemon 状态（例如：`🔓 Unlocked (Expires in 12m 30s)`）。
* **`pk lock`**
    向 Daemon 发送指令，立刻强制擦除内存中的私钥。
* **`pk run <command>`**
    代理命令执行。
  * 示例：`pk run sops edit secrets.yaml`
  * 简写：可以直接将别名加入 `.zshrc`: `alias sops="pk run sops"`。以后用户直接敲 `sops edit ...`，如果没解锁就会自动弹指纹识别，体验丝滑至极。

## 🌟 五、 为什么这个设计是企业级/大师级的？

1. **容灾设计完美**：引入 DEK/KEK 架构后，即使用户的 YubiKey 丢了或者重装了系统（Passkey 丢失），依然可以通过高强度的主密码解锁，不会造成公司加密配置库的彻底损毁。
2. **抗侧信道（Side-channel）**：选择 `XChaCha20-Poly1305` 而非 AES-GCM，在不需要特殊硬件指令集的终端设备上，天生免疫缓存计时攻击。
3. **不信任操作系统的 Swap**：通过 `zeroize` 确保进程内存被立刻覆写为零，避免私钥残留在操作系统的虚拟内存交换文件（Pagefile/Swap）中被提取。
4. **无缝集成**：通过 `execvp` 注入环境变量，完全不需要修改 `sops` 的源码，做到了完美的解耦。
