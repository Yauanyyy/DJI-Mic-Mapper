# DJI Mic Mapper

[English](README.md) | [简体中文](README.zh-CN.md)

[![CI](https://github.com/Yauanyyy/DJI-Mic-Mapper/actions/workflows/ci.yml/badge.svg)](https://github.com/Yauanyyy/DJI-Mic-Mapper/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

DJI Mic Mapper 是一个轻量的 Windows 工具，可以将 DJI Mic Mini 接收器上的按键用作可配置的键盘按键或快捷键。

程序运行在系统托盘中，不需要持续打开窗口。它通过 Windows Raw Input 识别接收器，并可选地处理该设备同时产生的 `Volume Up` 输入。

## 功能

- 将 DJI Mic Mini 按键映射为单个按键或组合键。
- 支持 F1–F24、字母、数字、导航键、修饰键、标点键和常用媒体键。
- 提供 `off`、`best_effort` 和 `block_all` 三种 `Volume Up` 处理模式。
- 从托盘菜单查看当前映射和运行状态。
- 无需重启即可重新加载配置修改。
- 在排查设备识别或事件时序问题时，提供只观察的诊断模式。

## 下载与运行

请从 [Releases](https://github.com/Yauanyyy/DJI-Mic-Mapper/releases) 页面下载最新版本：

- `DJI-Mic-Mapper-<version>-windows-x64-setup.exe`：推荐。运行安装程序，然后使用开始菜单快捷方式启动。
- `DJI-Mic-Mapper-<version>-windows-x64-portable.zip`：便携版。解压后直接运行。

### 首次启动

1. 运行安装程序安装，或将便携版解压到可以访问的文件夹。
2. 启动 `dji-mic-mapper.exe`，程序会继续在系统托盘中运行。
3. 右键托盘图标，查看当前映射、配置文件路径和运行状态。
4. 如需修改映射，编辑程序旁边的 `config.toml`，然后从托盘菜单选择 **Reload config**。

程序启动时 Windows 可能显示 UAC 提示。当前版本请求管理员权限，以确保全局键盘钩子和输入注入能够处理不同权限级别的 Windows 程序。

## 配置

配置文件是程序目录下的 `config.toml`。可以使用文本编辑器创建或修改它，保存后从托盘菜单选择 **Reload config**，无需重启程序。

大多数用户只需要设置 `target` 和 `volume_up_mode`。仓库中提供了可直接修改的[示例配置](config.toml)，其中主要的用户配置项如下：

~~~toml
target = "Ctrl+Shift+Home"
volume_up_mode = "best_effort"
log_level = "error"
correlation_window_ms = 10
~~~

配置项说明：

| 配置项 | 控制内容 | 默认值 |
| --- | --- | --- |
| `target` | DJI 按键按下时发送的按键或快捷键 | `F13` |
| `volume_up_mode` | 如何处理接收器产生的 `Volume Up` 输入 | `best_effort` |
| `suppress_volume_up` | 旧版兼容配置；建议改用 `volume_up_mode` | 未设置 |
| `log_level` | 日志级别：`off`、`error`、`warn`、`info`、`debug` 或 `trace` | `info` |
| `correlation_window_ms` | `best_effort` 使用的时间窗口，范围为 5–100ms | `10` |
| `usage_page` | HID 顶层集合 Usage Page，通常仅在高级匹配时需要修改 | `0x000C` |
| `usage` | HID 顶层集合 Usage，通常仅在高级匹配时需要修改 | `0x0001` |
| `button_usage` | DJI 按键 Usage，通常仅在高级匹配时需要修改 | `0x00E9` |
| `report_id` | HID Report ID；`0` 表示取消预先限定的 Report ID | `6` |

### 选择目标按键

`target` 可以配置单个按键或快捷键。修饰键与目标键之间使用 `+` 连接：

~~~toml
target = "F13"
~~~

~~~toml
target = "Ctrl+Alt+]"
~~~

按键名称不区分大小写；名称中的空格、连字符和下划线会被忽略。因此 `Volume_Up`、`Volume-Up` 和 `volumeup` 等写法等价。

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
| OEM 标点 | `OEM1` (`Semicolon`、`;`)、`OEM_PLUS` (`Equal`、`Equals`、`Plus`、`=`)、`OEMComma` (`Comma`、`,`)、`OEMMinus` (`Minus`、`Hyphen`、`-`)、`OEMPeriod` (`Period`、`Dot`、`.`)、`OEM2` (`Slash`、`ForwardSlash`、`/`)、`OEM3` (`Backtick`、`Grave`、<code>`</code>)、`OEM4` (`LeftBracket`、`LBracket`、`[`)、`OEM5` (`Backslash`、`\\`)、`OEM6` (`RightBracket`、`RBracket`、`]`)、`OEM7` (`Apostrophe`、`Quote`、`'`) | 例如 `Ctrl+Alt+OEM6`；`Shift+OEM6` 表示 `}`，`Shift+OEM_PLUS` 表示 `+` |

`+` 是组合键分隔符，因此不能直接写成 `target = "+"`。如需映射加号，请使用 `PLUS`、`Equal` 或 `OEM_PLUS`。TOML 字符串中的反斜杠和引号仍需遵循 TOML 转义规则。

## Volume Up 行为

接收器可能会同时产生标准的 `Volume Up` 事件和 DJI Raw Input 按键事件。请根据需要选择模式：

### `off`

不拦截 `Volume Up`。DJI 按键产生的音量增加行为会保留。

### `best_effort`（默认）

暂时截获标准 `Volume Up`，并与 DJI Raw Input 事件进行比较：

- 判断为 DJI 接收器产生的事件会被丢弃。
- 无法确认来源的事件会在短暂的关联窗口结束后重放。

这是大多数用户的推荐模式。它属于用户态方案，无法保证在所有设备或极端时序下实现设备级别的绝对屏蔽。

### `block_all`

屏蔽 Windows 识别到的所有 `Volume Up` 事件，包括键盘、耳机、麦克风和其他设备产生的事件。只有在你确实希望屏蔽所有来源的音量增加时才使用此模式；启用时会显示警告。

音量降低和静音不受影响。

## 诊断

如果接收器无法识别，或需要检查事件时序，可以使用诊断模式：

~~~powershell
.\dji-mic-mapper.exe --diagnose
~~~

诊断模式只观察事件，不会发送映射按键，也不会屏蔽或重放 `Volume Up`。它会记录 Raw Input 和 Volume Up Hook 事件，便于确认设备识别以及两路事件的时序关系。

日志位于程序目录下的 `logs` 文件夹。需要更详细的日志时，在 `config.toml` 中设置以下选项并重新加载配置：

~~~toml
log_level = "trace"
~~~

## 高级设备匹配

默认配置匹配以下 DJI Mic Mini 接收器参数：

| 参数 | 默认值 |
| --- | --- |
| VID | `0x2CA3` |
| PID | `0x4011` |
| Usage Page | `0x000C` |
| Usage | `0x0001` |
| Button Usage | `0x00E9` |
| Report ID | `6` |

大多数用户无需修改这些值。排查其他 HID 接口时，可以在 `config.toml` 中覆盖 Usage 和 Report ID。VID/PID 固定为默认设备，以避免误匹配其他 HID 设备。

## 限制

- 程序目前仅支持 Windows。
- Raw Input 是监听接口，不能按设备阻止 Windows 消费 HID 报告。
- `best_effort` 无法提供设备级别的绝对屏蔽；这需要 HID 过滤驱动。
- `block_all` 会影响所有标准 `Volume Up` 输入，而不只是 DJI 接收器。

## 许可证

本项目使用 [MIT License](LICENSE)。
