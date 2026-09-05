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
target = "RightAlt"
volume_up_mode = "best_effort"
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
- 可单独映射 `Alt`、`LeftAlt`、`RightAlt`、`LeftShift`、`RightShift`
- 组合键中可使用 `LAlt`、`RAlt`、`LShift`、`RShift` 等简写
- 常见标点键：`` ` ``、`-`、`=`、`[`、`]`、`\`、`;`、`'`、`,`、`.`、`/`
- 标点键也支持名称别名，例如 `OEM6`、`RIGHTBRACKET`、`OEM_PLUS`、`PLUS`
- 方向键、Home、End、PageUp、PageDown、Insert、Delete 等常用键
- 数字键盘和常用媒体键

例如：

```toml
target = "Ctrl+Alt+]"
```

需要发送带 `Shift` 的标点时，按物理按键组合配置，例如 `Shift+OEM6` 表示 `}`，
`Shift+OEM_PLUS` 表示 `+`。

## 诊断模式

```powershell
.\dji-mic-mapper.exe --diagnose
```

诊断模式不会发送映射按键，也不会屏蔽或重放 Volume Up。它会安装“只观察”的键盘钩子，同时记录 Raw Input 和 Volume Up 的微秒级到达时间，便于分析两路事件的顺序及时间差。

日志示例：

```text
1788612000.123456 [+1523.410ms] [TRACE] RAW t_us=1523380 device=... report=06 E9 00
1788612000.126104 [+1526.058ms] [DEBUG] VOLUME_HOOK t_us=1526001 edge=down ... action=observe_only
```

要获得完整诊断信息，使用 `--diagnose`，或者在普通模式中设置 `log_level = "trace"`。

`nearest_raw_press_delta_us` 表示 Volume Hook 时间减去最近一次 DJI Raw 按下时间：正数表示 Hook 较晚，负数表示 Hook 较早，`none` 表示没有发现可关联的 Raw 按下事件。若日志中有 `RAW_BUTTON` 而始终没有 `VOLUME_HOOK`，说明该设备的音量行为可能没有经过低级键盘钩子。

## Volume Up 屏蔽限制

`volume_up_mode` 支持三种模式：

- `off`：不拦截 Volume Up。
- `best_effort`：暂时截获全局 Volume Up，再与 DJI Raw Input 的按下区间进行时间关联。判断为 DJI 的事件会丢弃，无法确认来源的事件会延迟后重放。
- `block_all`：同时注册系统级 `VK_VOLUME_UP` 热键并安装低级键盘钩子，屏蔽所有被 Windows 识别为标准 Volume Up 的输入。启动或重新加载该配置时会显示警告；如果系统热键注册失败，程序会明确报错，而不会假装已经完全屏蔽。

> **警告：** `block_all` 不区分来源。程序运行期间，键盘、耳机、麦克风以及其他设备上的所有“音量增加”按键都会失效。音量降低和静音不受影响。

Raw Input 是监听接口，不能按设备阻止 Windows 消费 HID report。因此 `best_effort` 仍是用户态折中方案：

- 无法确认来源的事件会延迟约 `correlation_window_ms` 后重放，优先保证普通音量键可用。
- 极端时序下，DJI 的原始 Volume Up 仍可能漏过。要做到设备级绝对可靠屏蔽，需要安装 HID 过滤驱动。

## 构建

```powershell
cargo test
cargo build --release
powershell -ExecutionPolicy Bypass -File .\scripts\package.ps1
```

生成文件位于 `target\release\dji-mic-mapper.exe`。
便携发布包位于 `dist`，包含管理员清单版程序、配置文件和说明文档。
