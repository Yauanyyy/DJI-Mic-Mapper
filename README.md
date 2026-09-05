# DJI Mic Mapper

一个轻量的 Windows 11 后台程序，将 DJI Mic Mini 接收器产生的按钮事件映射为可配置的键盘单键或快捷键。

## 默认设备匹配

- VID：`0x2CA3`
- PID：`0x4011`
- 顶层集合 Usage Page：`0x000C`（Consumer）
- 顶层集合 Usage：`0x0001`（Consumer Control）
- 按钮 Usage：`0x00E9`（Volume Increment）
- Report ID：`6`

这些默认值来自本机实际连接设备的 Windows HID API 枚举结果。Usage 和 Report ID 可在 `config.toml` 中覆盖，以适配其他固件版本。

## 使用

1. 将 `dji-mic-mapper.exe` 与 `config.toml` 放在同一目录。
2. 启动程序并接受 UAC 管理员权限提示。
3. 程序驻留系统托盘。右键图标可以查看状态、重新加载配置或退出。
4. 修改配置后选择 **Reload config**，不需要重启程序。

配置示例：

```toml
target = "Ctrl+Shift+F13"
suppress_volume_up = true
log_level = "info"
correlation_window_ms = 100
usage_page = 0x000C
usage = 0x0001
button_usage = 0x00E9
report_id = 6
```

支持的日志级别：`off`、`error`、`warn`、`info`、`debug`、`trace`。日志位于程序旁边的 `logs` 目录；单个文件最大 1 MiB，最多保留 5 个历史文件。

目标键支持：

- `F1`–`F24`
- `A`–`Z`、`0`–`9`
- `Ctrl`、`Alt`、`Shift`、`Win` 修饰键
- 方向键、Home、End、PageUp、PageDown、Insert、Delete 等常用键
- 数字键盘和常用媒体键

## 诊断模式

```powershell
.\dji-mic-mapper.exe --diagnose
```

诊断模式不会发送映射按键，也不会安装 Volume Up 钩子。匹配设备的原始 HID report 会写入 trace 日志。

## Volume Up 屏蔽限制

Raw Input 是监听接口，不能按设备阻止 Windows 消费 HID report。程序采用用户态折中方案：暂时截获全局 `Volume Up`，再与 DJI Raw Input 的按下区间进行时间关联。

- 判断为 DJI 的事件会被丢弃。
- 无法确认来源的事件会延迟约 `correlation_window_ms` 后重放，优先保证普通键盘和耳机的音量键可用。
- 极端时序下，DJI 的原始 Volume Up 仍可能漏过。要做到设备级绝对可靠屏蔽，需要安装 HID 过滤驱动。

## 构建

```powershell
cargo test
cargo build --release
powershell -ExecutionPolicy Bypass -File .\scripts\package.ps1
```

生成文件位于 `target\release\dji-mic-mapper.exe`。
便携发布包位于 `dist`，包含管理员清单版程序、配置文件和说明文档。
