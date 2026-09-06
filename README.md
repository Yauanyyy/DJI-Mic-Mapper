# DJI Mic Mapper

[![CI](https://github.com/Yauanyyy/DJI-Mic-Mapper/actions/workflows/ci.yml/badge.svg)](https://github.com/Yauanyyy/DJI-Mic-Mapper/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

DJI Mic Mapper 是一个轻量的 Windows 后台工具，用于将 DJI Mic Mini 接收器的按键事件映射为可配置的键盘按键或快捷键。

程序运行在系统托盘中，不需要持续打开窗口。它通过 Windows Raw Input 识别 DJI 接收器，并可选地处理该设备同时产生的 `Volume Up` 输入。

## 功能

- 将 DJI Mic Mini 按键映射为单键或组合键。
- 支持 F1–F24、字母、数字、方向键、修饰键、标点键和常用媒体键。
- 支持 `off`、`best_effort` 和 `block_all` 三种 Volume Up 处理模式。
- 支持托盘状态查看和配置热重载。
- 提供只观察的诊断模式，用于记录 Raw Input 与 Volume Up 的事件顺序。
- 不依赖常驻运行时或大型框架。

## 下载与运行

打开 GitHub 的 [Releases](https://github.com/Yauanyyy/DJI-Mic-Mapper/releases) 页面，选择对应版本：

- `DJI-Mic-Mapper-<version>-windows-x64-setup.exe`：推荐。使用安装向导安装，并创建开始菜单快捷方式。
- `DJI-Mic-Mapper-<version>-windows-x64-portable.zip`：便携版。解压后直接运行，不写入系统安装目录。

当前版本使用管理员权限清单，首次启动和每次启动时可能显示 UAC 提示。这是为了保证全局键盘钩子和输入注入在不同权限级别的 Windows 程序中工作。

启动后程序会驻留系统托盘，并显示当前映射、配置文件路径和运行状态。右键托盘图标可以查看状态、重新加载配置、打开配置文件或退出程序；重载成功或失败都会显示结果。

## 配置

配置文件为程序目录下的 `config.toml`。修改后可以从托盘菜单选择 **Reload config**，无需重启程序。

最小配置示例：

```toml
target = "RightAlt"
volume_up_mode = "best_effort"
log_level = "info"
correlation_window_ms = 100
usage_page = 0x000C
usage = 0x0001
button_usage = 0x00E9
report_id = 6
```

配置项说明：

| 配置项 | 说明 | 默认值 |
| --- | --- | --- |
| `target` | 目标按键或快捷键 | `F13` |
| `volume_up_mode` | Volume Up 处理模式 | `best_effort` |
| `suppress_volume_up` | 旧版兼容配置；与 `volume_up_mode` 二选一 | 未设置 |
| `log_level` | `off`、`error`、`warn`、`info`、`debug` 或 `trace` | `info` |
| `correlation_window_ms` | `best_effort` 使用的关联窗口，范围为 20–500ms | `100` |
| `usage_page` | HID 顶层集合 Usage Page | `0x000C` |
| `usage` | HID 顶层集合 Usage | `0x0001` |
| `button_usage` | DJI 按键 Usage | `0x00E9` |
| `report_id` | HID Report ID；`0` 表示不预先限定 Report ID | `6` |

### 按键映射查询表

`target` 可以配置单个按键，也可以使用 `+` 连接修饰键组成快捷键：

```toml
target = "F13"
```

```toml
target = "Ctrl+Alt+]"
```

按键名称不区分大小写；名称中的空格、连字符和下划线会被忽略。因此 `Volume_Up`、`Volume-Up` 和 `volumeup` 等写法等价。下表列出所有支持的按键名称和别名。

| 类别 | 可用名称 | 示例或说明 |
| --- | --- | --- |
| 字母 | `A`–`Z` | `A`、`M`、`Z` |
| 数字 | `0`–`9` | 主键盘数字键 |
| 功能键 | `F1`–`F24` | `F13` |
| 修饰键 | `Ctrl` (`Control`)、`Alt`、`Shift`、`Win` (`Windows`、`Meta`)、`LeftWin` (`LWin`)、`RightWin` (`RWin`)、`LeftAlt` (`LAlt`)、`RightAlt` (`RAlt`、`AltGr`)、`LeftShift` (`LShift`)、`RightShift` (`RShift`) | 作为组合键前缀，例如 `Ctrl+Shift+F13`；也可以单独作为目标键 |
| 编辑与控制 | `Backspace` (`Back`)、`Tab`、`Enter` (`Return`)、`Esc` (`Escape`)、`Space` | `Ctrl+Alt+Delete` |
| 导航 | `PageUp` (`PgUp`)、`PageDown` (`PgDn`)、`Home`、`End`、`Insert` (`Ins`)、`Delete` (`Del`)、`Left`、`Up`、`Right`、`Down` | `Shift+Home` |
| 数字键盘 | `Numpad0`–`Numpad9`、`Multiply`、`Add`、`Subtract`、`Decimal`、`Divide`、`NumLock`、`ScrollLock` | `Numpad1`、`Add` |
| 媒体键 | `VolumeMute` (`Mute`)、`VolumeDown`、`VolumeUp`、`NextTrack`、`PrevTrack` (`PreviousTrack`)、`StopMedia` (`MediaStop`)、`PlayPause` (`MediaPlayPause`) | `PlayPause` |
| OEM 标点 | `OEM1` (`Semicolon`、`;`)、`OEM_PLUS` (`Equal`、`Equals`、`Plus`、`=`)、`OEMComma` (`Comma`、`,`)、`OEMMinus` (`Minus`、`Hyphen`、`-`)、`OEMPeriod` (`Period`、`Dot`、`.`)、`OEM2` (`Slash`、`ForwardSlash`、`/`)、`OEM3` (`Backtick`、`Grave`、`` ` ``)、`OEM4` (`LeftBracket`、`LBracket`、`[`)、`OEM5` (`Backslash`、`\\`)、`OEM6` (`RightBracket`、`RBracket`、`]`)、`OEM7` (`Apostrophe`、`Quote`、`'`) | 例如 `Ctrl+Alt+OEM6`；`Shift+OEM6` 表示 `}`，`Shift+OEM_PLUS` 表示 `+` |

`+` 是组合键分隔符，因此不能直接写成 `target = "+"`；映射加号请使用 `PLUS`、`Equal` 或 `OEM_PLUS`。配置文件中使用 TOML 字符串时，反斜杠和引号仍需遵循 TOML 转义规则。

## Volume Up 处理模式

### `off`

不拦截 Volume Up。DJI 按键产生的系统音量行为会保留。

### `best_effort`

暂时截获标准 Volume Up，并与 DJI Raw Input 事件进行时间关联：

- 判断为 DJI 产生的 Volume Up 会被丢弃。
- 无法确认来源的 Volume Up 会在关联窗口结束后重放。

这是用户态方案，不能保证对所有设备和极端时序都做到设备级别的绝对屏蔽。

### `block_all`

屏蔽系统识别到的所有 Volume Up，包括键盘、耳机、麦克风和其他设备产生的音量增加事件。该模式不区分来源，启用时会显示警告。

音量降低和静音不受影响。

## 诊断模式

使用以下命令启动只观察的诊断模式：

```powershell
.\dji-mic-mapper.exe --diagnose
```

诊断模式不会发送映射按键，也不会屏蔽或重放 Volume Up。它会记录 Raw Input 和 Volume Up Hook 事件，便于确认设备识别和两路事件的时序关系。

日志位于程序目录下的 `logs` 文件夹。需要更完整的日志时，也可以在普通模式的 `config.toml` 中设置：

```toml
log_level = "trace"
```

## 默认设备

当前默认匹配以下 DJI Mic Mini 接收器参数：

| 参数 | 默认值 |
| --- | --- |
| VID | `0x2CA3` |
| PID | `0x4011` |
| Usage Page | `0x000C` |
| Usage | `0x0001` |
| Button Usage | `0x00E9` |
| Report ID | `6` |

Usage 和 Report ID 可以通过配置文件覆盖。VID/PID 当前固定为默认设备，避免误匹配其他 HID 设备。

## 从源码构建

需要：

- Windows；当前主要在 Windows 11 上测试；
- Rust stable toolchain；
- 可用的 Windows C/C++ 构建环境。

执行：

```powershell
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

生成便携目录：

```powershell
.\scripts\package.ps1
```

生成完整发布产物（便携 ZIP + 安装程序）需要安装 [Inno Setup 6](https://jrsoftware.org/isinfo.php)，然后执行：

```powershell
.\scripts\release.ps1 -Version 0.1.1
```

发布产物会写入 `artifacts` 目录。`target`、`dist` 和 `artifacts` 都是生成目录，不应提交到 Git。

## GitHub Release

推送版本标签后，GitHub Actions 会自动：

1. 运行格式检查、测试、Clippy 和 Release 构建。
2. 构建便携 ZIP。
3. 构建 Inno Setup 安装程序。
4. 创建 GitHub Release 并上传两个文件。

示例：

```powershell
git tag v0.1.1
git push origin v0.1.1
```

普通提交和 Pull Request 会触发 CI，但不会创建 Release。

## 限制

- 程序目前面向 Windows，其他平台不受支持。
- Raw Input 是监听接口，不能按设备阻止 Windows 消费 HID 报告。
- `best_effort` 是用户态折中方案；需要设备级绝对屏蔽时必须使用 HID 过滤驱动。
- `block_all` 会影响所有标准 Volume Up 输入，请谨慎使用。

## 许可证

本项目使用 [MIT License](LICENSE)。
