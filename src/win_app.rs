#![allow(unsafe_op_in_unsafe_fn)]

use std::{
    collections::{HashMap, VecDeque},
    ffi::c_void,
    mem::{size_of, zeroed},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Devices::HumanInterfaceDevice::{
        HIDP_STATUS_SUCCESS, HidP_GetUsages, HidP_Input, HidP_MaxUsageListLength,
    },
    Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, POINT,
        WPARAM,
    },
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{
        Input::{
            GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList,
            KeyboardAndMouse::{
                INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP,
                MOD_NOREPEAT, RegisterHotKey, SendInput, UnregisterHotKey, VK_VOLUME_UP,
            },
            RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST, RAWINPUTHEADER, RID_DEVICE_INFO,
            RID_INPUT, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDEV_REMOVE, RIDI_DEVICEINFO,
            RIDI_DEVICENAME, RIDI_PREPARSEDDATA, RIM_TYPEHID, RegisterRawInputDevices,
        },
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
            Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CW_USEDEFAULT, CallNextHookEx, CreatePopupMenu, CreateWindowExW,
            DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, FindWindowW,
            GIDC_ARRIVAL, GIDC_REMOVAL, GetCursorPos, GetMessageW, HHOOK, IDI_APPLICATION,
            KBDLLHOOKSTRUCT, KillTimer, LLKHF_INJECTED, LoadIconW, MB_ICONERROR,
            MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG,
            MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
            SetForegroundWindow, SetTimer, SetWindowsHookExW, TPM_RIGHTBUTTON, TrackPopupMenu,
            TranslateMessage, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_APP, WM_COMMAND, WM_DESTROY,
            WM_HOTKEY, WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK,
            WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW,
        },
    },
};

use crate::{
    config::{Config, DJI_PRODUCT_ID, DJI_VENDOR_ID, VolumeUpMode},
    keymap::Chord,
    logger::{self, Level},
};

const WINDOW_CLASS: &str = "DJIMicMapper.HiddenWindow.1";
const WINDOW_TITLE: &str = "DJI Mic Mapper";
const MUTEX_NAME: &str = "Local\\DJIMicMapper.SingleInstance.1";

const WM_TRAY: u32 = WM_APP + 1;
const WM_RELOAD_CONFIG: u32 = WM_APP + 2;
const TIMER_FLUSH_VOLUME: usize = 1;
const MENU_STATUS: usize = 1000;
const MENU_RELOAD: usize = 1001;
const MENU_EXIT: usize = 1002;
const TRAY_ICON_ID: u32 = 1;
const BLOCK_VOLUME_HOTKEY_ID: i32 = 1;
const INJECTED_MARKER: usize = 0x444A_494D;

#[derive(Clone)]
struct RuntimeConfig {
    config: Config,
    chord: Chord,
}

#[derive(Default)]
struct DeviceState {
    pressed: bool,
    preparsed_data: Vec<usize>,
    max_usages: u32,
}

struct PendingVolumeEvent {
    at: Instant,
    key_up: bool,
    keyboard_time_ms: u32,
    scan_code: u32,
    mode: VolumeUpMode,
    observe_only: bool,
    injected: bool,
}

struct PressInterval {
    start: Instant,
    end: Instant,
}

struct AppState {
    hwnd: isize,
    config_path: PathBuf,
    runtime: Option<RuntimeConfig>,
    config_error: Option<String>,
    diagnose: bool,
    registered_usage: Option<(u16, u16)>,
    hook: isize,
    block_hotkey_registered: bool,
    devices: HashMap<usize, DeviceState>,
    active_presses: HashMap<usize, Instant>,
    completed_presses: VecDeque<PressInterval>,
    pending_volume: VecDeque<PendingVolumeEvent>,
}

static APP: OnceLock<Mutex<AppState>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();

pub fn run() -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("cannot locate executable: {error}"))?;
    let base_directory = executable
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let config_path = base_directory.join("config.toml");
    let diagnose = std::env::args_os().any(|argument| argument == "--diagnose");
    let load_result = Config::load(&config_path);
    let configured_log_level = load_result
        .as_ref()
        .map_or(Level::Info, |(_, _, level)| *level);

    logger::init(
        &base_directory,
        if diagnose {
            Level::Trace
        } else {
            configured_log_level
        },
    );
    logger::log(
        Level::Info,
        if diagnose {
            "starting in diagnostic mode"
        } else {
            "starting"
        },
    );

    let mutex_name = wide(MUTEX_NAME);
    let instance_mutex = unsafe { CreateMutexW(null(), 1, mutex_name.as_ptr()) };
    if instance_mutex.is_null() {
        return Err(last_error("CreateMutexW"));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        let class_name = wide(WINDOW_CLASS);
        let existing = unsafe { FindWindowW(class_name.as_ptr(), null()) };
        if !existing.is_null() {
            unsafe { PostMessageW(existing, WM_RELOAD_CONFIG, 0, 0) };
        }
        unsafe { CloseHandle(instance_mutex) };
        return Ok(());
    }

    let (runtime, config_error) = match load_result {
        Ok((config, chord, _)) => (Some(RuntimeConfig { config, chord }), None),
        Err(error) => {
            logger::log(Level::Error, &error);
            (None, Some(error))
        }
    };

    let class_name = wide(WINDOW_CLASS);
    let window_title = wide(WINDOW_TITLE);
    let module = unsafe { GetModuleHandleW(null()) };
    if module.is_null() {
        unsafe { CloseHandle(instance_mutex) };
        return Err(last_error("GetModuleHandleW"));
    }

    let icon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
    let window_class = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(window_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: module,
        hIcon: icon,
        hCursor: null_mut(),
        hbrBackground: null_mut(),
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
    };
    if unsafe { RegisterClassW(&window_class) } == 0 {
        unsafe { CloseHandle(instance_mutex) };
        return Err(last_error("RegisterClassW"));
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            0,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            null_mut(),
            null_mut(),
            module,
            null(),
        )
    };
    if hwnd.is_null() {
        unsafe { CloseHandle(instance_mutex) };
        return Err(last_error("CreateWindowExW"));
    }

    let _ = TASKBAR_CREATED.set(unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) });
    APP.set(Mutex::new(AppState {
        hwnd: hwnd as isize,
        config_path,
        runtime,
        config_error,
        diagnose,
        registered_usage: None,
        hook: 0,
        block_hotkey_registered: false,
        devices: HashMap::new(),
        active_presses: HashMap::new(),
        completed_presses: VecDeque::new(),
        pending_volume: VecDeque::new(),
    }))
    .map_err(|_| "application state was initialized twice".to_owned())?;

    unsafe {
        add_or_update_tray_icon(hwnd, NIM_ADD);
        activate_runtime(hwnd);
    }

    if let Some(error) = current_config_error() {
        show_warning(&format!(
            "Configuration error; mapping is disabled.\n\n{error}"
        ));
    }

    let mut message: MSG = unsafe { zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if result == -1 {
            logger::log(Level::Error, &last_error("GetMessageW"));
            break;
        }
        if result == 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe { CloseHandle(instance_mutex) };
    logger::log(Level::Info, "stopped");
    Ok(())
}

pub fn show_fatal_error(message: &str) {
    let text = wide(message);
    let title = wide(WINDOW_TITLE);
    unsafe {
        MessageBoxW(
            null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if TASKBAR_CREATED.get().is_some_and(|id| *id == message) {
        add_or_update_tray_icon(hwnd, NIM_ADD);
        return 0;
    }

    match message {
        WM_INPUT => {
            handle_raw_input(lparam as *mut c_void);
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        WM_INPUT_DEVICE_CHANGE => {
            handle_device_change(wparam as u32, lparam as HANDLE);
            0
        }
        WM_TRAY => {
            match lparam as u32 {
                WM_RBUTTONUP => show_tray_menu(hwnd),
                WM_LBUTTONDBLCLK => show_status_dialog(),
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            match wparam & 0xFFFF {
                MENU_RELOAD => reload_configuration(hwnd),
                MENU_EXIT => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_RELOAD_CONFIG => {
            reload_configuration(hwnd);
            0
        }
        WM_HOTKEY if wparam as i32 == BLOCK_VOLUME_HOTKEY_ID => {
            logger::log_args(
                Level::Info,
                format_args!(
                    "VOLUME_HOTKEY t_us={} mode=block_all action=blocked",
                    event_micros(Instant::now())
                ),
            );
            0
        }
        WM_TIMER if wparam == TIMER_FLUSH_VOLUME => {
            flush_pending_volume(hwnd);
            0
        }
        WM_DESTROY => {
            shutdown(hwnd);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam != 0 {
        let event = &*(lparam as *const KBDLLHOOKSTRUCT);
        let keyboard_message = wparam as u32;
        let is_key_event = matches!(
            keyboard_message,
            WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP
        );
        let is_ours = event.dwExtraInfo == INJECTED_MARKER;
        let is_injected = event.flags & LLKHF_INJECTED != 0;
        if is_key_event
            && event.vkCode as u16 == VK_VOLUME_UP
            && !is_ours
            && let Some(app) = APP.get()
            && let Ok(mut app) = app.lock()
        {
            let volume_mode = app.runtime.as_ref().map_or(VolumeUpMode::Off, |runtime| {
                runtime.config.effective_volume_up_mode()
            });
            let observe_only = app.diagnose;
            let should_capture = observe_only
                || volume_mode == VolumeUpMode::BlockAll
                || (volume_mode == VolumeUpMode::BestEffort && !is_injected);
            if should_capture {
                app.pending_volume.push_back(PendingVolumeEvent {
                    at: Instant::now(),
                    key_up: matches!(keyboard_message, WM_KEYUP | WM_SYSKEYUP),
                    keyboard_time_ms: event.time,
                    scan_code: event.scanCode,
                    mode: volume_mode,
                    observe_only,
                    injected: is_injected,
                });
                SetTimer(app.hwnd as HWND, TIMER_FLUSH_VOLUME, 10, None);
                if !observe_only && volume_mode != VolumeUpMode::Off {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn activate_runtime(hwnd: HWND) {
    let mut error = None;
    let mut block_all_warning = false;
    if let Some(app_mutex) = APP.get()
        && let Ok(mut app) = app_mutex.lock()
    {
        app.devices.clear();
        app.active_presses.clear();
        app.completed_presses.clear();
        app.pending_volume.clear();

        let Some(runtime) = app.runtime.clone() else {
            drop(app);
            add_or_update_tray_icon(hwnd, NIM_MODIFY);
            return;
        };

        let raw_device = RAWINPUTDEVICE {
            usUsagePage: runtime.config.usage_page,
            usUsage: runtime.config.usage,
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        };
        if RegisterRawInputDevices(&raw_device, 1, size_of::<RAWINPUTDEVICE>() as u32) == 0 {
            error = Some(last_error("RegisterRawInputDevices"));
        } else {
            app.registered_usage = Some((runtime.config.usage_page, runtime.config.usage));
            enumerate_matching_devices(&mut app);
            logger::log_args(
                Level::Info,
                format_args!(
                    "listening for VID={DJI_VENDOR_ID:#06X} PID={DJI_PRODUCT_ID:#06X} TLC={:#06X}/{:#06X} button={:#06X} report_id={}",
                    runtime.config.usage_page,
                    runtime.config.usage,
                    runtime.config.button_usage,
                    runtime.config.report_id
                ),
            );
        }

        let volume_mode = runtime.config.effective_volume_up_mode();
        logger::log_args(
            Level::Info,
            format_args!("Volume Up mode: {}", volume_mode.label()),
        );
        if error.is_none() && !app.diagnose && volume_mode == VolumeUpMode::BlockAll {
            if RegisterHotKey(
                hwnd,
                BLOCK_VOLUME_HOTKEY_ID,
                MOD_NOREPEAT,
                VK_VOLUME_UP as u32,
            ) == 0
            {
                error = Some(last_error("RegisterHotKey(VK_VOLUME_UP)"));
            } else {
                app.block_hotkey_registered = true;
                logger::log(
                    Level::Warn,
                    "system-wide Volume Up hotkey reservation enabled",
                );
            }
        }
        if error.is_none() && (app.diagnose || volume_mode != VolumeUpMode::Off) {
            let module = GetModuleHandleW(null());
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), module, 0);
            if hook.is_null() {
                error = Some(last_error("SetWindowsHookExW"));
            } else {
                app.hook = hook as isize;
                if app.diagnose {
                    logger::log(
                        Level::Info,
                        "observe-only Volume Up hook enabled for diagnostics",
                    );
                } else if volume_mode == VolumeUpMode::BlockAll {
                    logger::log(Level::Warn, "all Volume Up inputs will be blocked");
                    block_all_warning = true;
                } else {
                    logger::log(Level::Info, "best-effort Volume Up suppression enabled");
                }
            }
        }
    }

    if let Some(error) = error {
        deactivate_runtime();
        logger::log(Level::Error, &error);
        if let Some(app_mutex) = APP.get()
            && let Ok(mut app) = app_mutex.lock()
        {
            app.config_error = Some(error.clone());
        }
        show_warning(&format!(
            "DJI Mic Mapper could not activate mapping.\n\n{error}"
        ));
    } else if block_all_warning {
        show_warning(
            "WARNING: volume_up_mode is set to block_all.\n\nEvery Volume Up input from every keyboard, headset, microphone, or other device will be blocked while DJI Mic Mapper is running.",
        );
    }
    add_or_update_tray_icon(hwnd, NIM_MODIFY);
}

unsafe fn deactivate_runtime() {
    if let Some(app_mutex) = APP.get()
        && let Ok(mut app) = app_mutex.lock()
    {
        if app.block_hotkey_registered {
            UnregisterHotKey(app.hwnd as HWND, BLOCK_VOLUME_HOTKEY_ID);
            app.block_hotkey_registered = false;
        }
        if app.hook != 0 {
            UnhookWindowsHookEx(app.hook as HHOOK);
            app.hook = 0;
        }
        if let Some((usage_page, usage)) = app.registered_usage.take() {
            let raw_device = RAWINPUTDEVICE {
                usUsagePage: usage_page,
                usUsage: usage,
                dwFlags: RIDEV_REMOVE,
                hwndTarget: null_mut(),
            };
            RegisterRawInputDevices(&raw_device, 1, size_of::<RAWINPUTDEVICE>() as u32);
        }
        let pending: Vec<_> = app.pending_volume.drain(..).collect();
        for event in pending {
            if event.mode == VolumeUpMode::BestEffort && !event.observe_only {
                send_volume_event(event.key_up);
            }
        }
        app.devices.clear();
        app.active_presses.clear();
        app.completed_presses.clear();
    }
}

unsafe fn reload_configuration(hwnd: HWND) {
    let (path, diagnose) = match APP.get().and_then(|app| app.lock().ok()) {
        Some(app) => (app.config_path.clone(), app.diagnose),
        None => return,
    };
    let result = Config::load(&path);
    deactivate_runtime();

    let mut error_to_show = None;
    if let Some(app_mutex) = APP.get()
        && let Ok(mut app) = app_mutex.lock()
    {
        match result {
            Ok((config, chord, level)) => {
                logger::set_level(if diagnose { Level::Trace } else { level });
                logger::log(Level::Info, "configuration reloaded");
                app.runtime = Some(RuntimeConfig { config, chord });
                app.config_error = None;
            }
            Err(error) => {
                logger::log(Level::Error, &error);
                app.runtime = None;
                app.config_error = Some(error.clone());
                error_to_show = Some(error);
            }
        }
    }
    activate_runtime(hwnd);
    if let Some(error) = error_to_show {
        show_warning(&format!(
            "Configuration error; mapping is disabled.\n\n{error}"
        ));
    }
}

unsafe fn handle_raw_input(raw_handle: *mut c_void) {
    let mut byte_count = 0u32;
    if GetRawInputData(
        raw_handle,
        RID_INPUT,
        null_mut(),
        &mut byte_count,
        size_of::<RAWINPUTHEADER>() as u32,
    ) == u32::MAX
        || byte_count < size_of::<RAWINPUTHEADER>() as u32
    {
        logger::log(Level::Warn, &last_error("GetRawInputData(size)"));
        return;
    }

    let word_count = (byte_count as usize).div_ceil(size_of::<usize>());
    let mut buffer = vec![0usize; word_count];
    let received = GetRawInputData(
        raw_handle,
        RID_INPUT,
        buffer.as_mut_ptr().cast(),
        &mut byte_count,
        size_of::<RAWINPUTHEADER>() as u32,
    );
    if received == u32::MAX {
        logger::log(Level::Warn, &last_error("GetRawInputData"));
        return;
    }

    let raw = &*(buffer.as_ptr() as *const RAWINPUT);
    if raw.header.dwType != RIM_TYPEHID {
        return;
    }
    let device_key = raw.header.hDevice as usize;
    let hid = raw.data.hid;
    let report_size = hid.dwSizeHid as usize;
    let report_count = hid.dwCount as usize;
    if report_size == 0 || report_count == 0 {
        return;
    }

    let (runtime, known_match) = match APP.get().and_then(|app| app.lock().ok()) {
        Some(app) => (app.runtime.clone(), app.devices.contains_key(&device_key)),
        None => return,
    };
    let Some(runtime) = runtime else {
        return;
    };

    if !known_match {
        let Some(identity) = query_device(raw.header.hDevice) else {
            return;
        };
        logger::log_args(
            Level::Debug,
            format_args!(
                "raw device {} VID={:#06X} PID={:#06X} TLC={:#06X}/{:#06X}",
                identity.name,
                identity.vendor_id,
                identity.product_id,
                identity.usage_page,
                identity.usage
            ),
        );
        if !identity.matches(&runtime.config) {
            return;
        }
        if let Some(app_mutex) = APP.get()
            && let Ok(mut app) = app_mutex.lock()
        {
            app.devices
                .entry(device_key)
                .or_insert_with(|| create_device_state(raw.header.hDevice, &runtime.config));
        }
        add_or_update_tray_icon(raw_window(), NIM_MODIFY);
    }

    let data = hid.bRawData.as_ptr();
    for index in 0..report_count {
        let received_at = Instant::now();
        let report = std::slice::from_raw_parts(data.add(index * report_size), report_size);
        if logger::enabled(Level::Trace) {
            logger::log_args(
                Level::Trace,
                format_args!(
                    "RAW t_us={} device={device_key:#X} report={}",
                    event_micros(received_at),
                    hex_bytes(report)
                ),
            );
        }
        if let Some(pressed) = parsed_button_state(device_key, report, &runtime.config) {
            process_button_state(device_key, pressed, received_at, &runtime);
        }
    }
}

unsafe fn process_button_state(
    device_key: usize,
    pressed: bool,
    received_at: Instant,
    runtime: &RuntimeConfig,
) {
    let mut trigger = false;
    if let Some(app_mutex) = APP.get()
        && let Ok(mut app) = app_mutex.lock()
    {
        let previous = app.devices.entry(device_key).or_default().pressed;
        if pressed && !previous {
            app.devices.get_mut(&device_key).unwrap().pressed = true;
            app.active_presses.insert(device_key, received_at);
            trigger = !app.diagnose;
            logger::log_args(
                Level::Info,
                format_args!(
                    "RAW_BUTTON t_us={} device={device_key:#X} edge=down",
                    event_micros(received_at)
                ),
            );
        } else if !pressed && previous {
            app.devices.get_mut(&device_key).unwrap().pressed = false;
            if let Some(start) = app.active_presses.remove(&device_key) {
                app.completed_presses.push_back(PressInterval {
                    start,
                    end: received_at,
                });
            }
            logger::log_args(
                Level::Info,
                format_args!(
                    "RAW_BUTTON t_us={} device={device_key:#X} edge=up",
                    event_micros(received_at)
                ),
            );
        }
    }

    if trigger {
        logger::log_args(
            Level::Info,
            format_args!("triggering {}", runtime.chord.display),
        );
        send_chord(&runtime.chord);
    }
}

unsafe fn handle_device_change(kind: u32, device: HANDLE) {
    let device_key = device as usize;
    let mut update_tray = false;
    match kind {
        GIDC_ARRIVAL => {
            let runtime = APP
                .get()
                .and_then(|app| app.lock().ok())
                .and_then(|app| app.runtime.clone());
            if let Some(runtime) = runtime
                && let Some(identity) = query_device(device)
            {
                logger::log_args(
                    Level::Debug,
                    format_args!(
                        "device arrived: {} VID={:#06X} PID={:#06X} TLC={:#06X}/{:#06X}",
                        identity.name,
                        identity.vendor_id,
                        identity.product_id,
                        identity.usage_page,
                        identity.usage
                    ),
                );
                if identity.matches(&runtime.config)
                    && let Some(app_mutex) = APP.get()
                    && let Ok(mut app) = app_mutex.lock()
                {
                    app.devices
                        .entry(device_key)
                        .or_insert_with(|| create_device_state(device, &runtime.config));
                    update_tray = true;
                }
            }
        }
        GIDC_REMOVAL => {
            if let Some(app_mutex) = APP.get()
                && let Ok(mut app) = app_mutex.lock()
                && app.devices.remove(&device_key).is_some()
            {
                if let Some(start) = app.active_presses.remove(&device_key) {
                    app.completed_presses.push_back(PressInterval {
                        start,
                        end: Instant::now(),
                    });
                }
                logger::log_args(Level::Info, format_args!("device removed: {device_key:#X}"));
                update_tray = true;
            }
        }
        _ => {}
    }
    if update_tray {
        add_or_update_tray_icon(raw_window(), NIM_MODIFY);
    }
}

unsafe fn enumerate_matching_devices(app: &mut AppState) {
    let Some(runtime) = app.runtime.as_ref() else {
        return;
    };
    let mut count = 0u32;
    if GetRawInputDeviceList(
        null_mut(),
        &mut count,
        size_of::<RAWINPUTDEVICELIST>() as u32,
    ) == u32::MAX
        || count == 0
    {
        return;
    }
    let mut devices = vec![RAWINPUTDEVICELIST::default(); count as usize];
    let received = GetRawInputDeviceList(
        devices.as_mut_ptr(),
        &mut count,
        size_of::<RAWINPUTDEVICELIST>() as u32,
    );
    if received == u32::MAX {
        return;
    }
    for device in devices.into_iter().take(received as usize) {
        if device.dwType == RIM_TYPEHID
            && let Some(identity) = query_device(device.hDevice)
            && identity.matches(&runtime.config)
        {
            logger::log_args(
                Level::Info,
                format_args!("matching device online: {}", identity.name),
            );
            app.devices
                .entry(device.hDevice as usize)
                .or_insert_with(|| create_device_state(device.hDevice, &runtime.config));
        }
    }
}

struct DeviceIdentity {
    vendor_id: u32,
    product_id: u32,
    usage_page: u16,
    usage: u16,
    name: String,
}

impl DeviceIdentity {
    fn matches(&self, config: &Config) -> bool {
        self.vendor_id == DJI_VENDOR_ID
            && self.product_id == DJI_PRODUCT_ID
            && self.usage_page == config.usage_page
            && self.usage == config.usage
    }
}

unsafe fn query_device(device: HANDLE) -> Option<DeviceIdentity> {
    let mut info = RID_DEVICE_INFO {
        cbSize: size_of::<RID_DEVICE_INFO>() as u32,
        ..RID_DEVICE_INFO::default()
    };
    let mut info_size = info.cbSize;
    let result = GetRawInputDeviceInfoW(
        device,
        RIDI_DEVICEINFO,
        (&mut info as *mut RID_DEVICE_INFO).cast(),
        &mut info_size,
    );
    if result == u32::MAX || info.dwType != RIM_TYPEHID {
        return None;
    }
    let hid = info.Anonymous.hid;
    Some(DeviceIdentity {
        vendor_id: hid.dwVendorId,
        product_id: hid.dwProductId,
        usage_page: hid.usUsagePage,
        usage: hid.usUsage,
        name: raw_device_name(device).unwrap_or_else(|| format!("handle={:#X}", device as usize)),
    })
}

unsafe fn raw_device_name(device: HANDLE) -> Option<String> {
    let mut char_count = 0u32;
    if GetRawInputDeviceInfoW(device, RIDI_DEVICENAME, null_mut(), &mut char_count) == u32::MAX
        || char_count == 0
    {
        return None;
    }
    let mut buffer = vec![0u16; char_count as usize + 1];
    if GetRawInputDeviceInfoW(
        device,
        RIDI_DEVICENAME,
        buffer.as_mut_ptr().cast(),
        &mut char_count,
    ) == u32::MAX
    {
        return None;
    }
    let end = buffer
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..end]))
}

unsafe fn create_device_state(device: HANDLE, config: &Config) -> DeviceState {
    let mut byte_count = 0u32;
    if GetRawInputDeviceInfoW(device, RIDI_PREPARSEDDATA, null_mut(), &mut byte_count) == u32::MAX
        || byte_count == 0
    {
        logger::log(
            Level::Warn,
            "could not query HID preparsed-data size; using report fallback",
        );
        return DeviceState::default();
    }

    let word_count = (byte_count as usize).div_ceil(size_of::<usize>());
    let mut preparsed_data = vec![0usize; word_count];
    if GetRawInputDeviceInfoW(
        device,
        RIDI_PREPARSEDDATA,
        preparsed_data.as_mut_ptr().cast(),
        &mut byte_count,
    ) == u32::MAX
    {
        logger::log(
            Level::Warn,
            "could not read HID preparsed data; using report fallback",
        );
        return DeviceState::default();
    }

    let max_usages = HidP_MaxUsageListLength(
        HidP_Input,
        config.usage_page,
        preparsed_data.as_ptr() as isize,
    );
    if max_usages == 0 {
        logger::log(
            Level::Warn,
            "HID parser reported no button usages; using report fallback",
        );
    } else {
        logger::log_args(
            Level::Info,
            format_args!("HID parser capacity: {max_usages} usages"),
        );
    }

    DeviceState {
        pressed: false,
        preparsed_data,
        max_usages,
    }
}

unsafe fn parsed_button_state(device_key: usize, report: &[u8], config: &Config) -> Option<bool> {
    if config.report_id != 0 && report.first().copied()? != config.report_id {
        return None;
    }

    if let Some(app_mutex) = APP.get()
        && let Ok(app) = app_mutex.lock()
        && let Some(device) = app.devices.get(&device_key)
        && !device.preparsed_data.is_empty()
        && device.max_usages > 0
    {
        let mut usages = vec![0u16; device.max_usages as usize];
        let mut usage_count = device.max_usages;
        let status = HidP_GetUsages(
            HidP_Input,
            config.usage_page,
            0,
            usages.as_mut_ptr(),
            &mut usage_count,
            device.preparsed_data.as_ptr() as isize,
            report.as_ptr() as *mut u8,
            report.len() as u32,
        );
        if status == HIDP_STATUS_SUCCESS {
            usages.truncate(usage_count as usize);
            return Some(usages.contains(&config.button_usage));
        }
        logger::log_args(
            Level::Warn,
            format_args!("HidP_GetUsages returned status {status:#010X}; using report fallback"),
        );
    }

    report_button_state(report, config)
}

fn report_button_state(report: &[u8], config: &Config) -> Option<bool> {
    let payload = if config.report_id == 0 {
        report
    } else {
        if report.first().copied()? != config.report_id {
            return None;
        }
        &report[1..]
    };
    if payload.is_empty() {
        return None;
    }

    let pressed = payload
        .as_chunks::<2>()
        .0
        .iter()
        .any(|bytes| u16::from_le_bytes(*bytes) == config.button_usage);
    Some(pressed)
}

unsafe fn flush_pending_volume(hwnd: HWND) {
    let now = Instant::now();
    let mut decisions = Vec::new();
    let mut still_pending = false;

    if let Some(app_mutex) = APP.get()
        && let Ok(mut app) = app_mutex.lock()
    {
        let window = app
            .runtime
            .as_ref()
            .map_or(Duration::from_millis(100), |runtime| {
                Duration::from_millis(runtime.config.correlation_window_ms)
            });

        while app
            .completed_presses
            .front()
            .is_some_and(|interval| now.duration_since(interval.end) > window * 4)
        {
            app.completed_presses.pop_front();
        }

        let pending = std::mem::take(&mut app.pending_volume);
        for event in pending {
            if event.observe_only {
                decisions.push((event, "observe_only", None));
            } else if event.mode == VolumeUpMode::BlockAll {
                decisions.push((event, "block_all", None));
            } else if now.duration_since(event.at) < window {
                app.pending_volume.push_back(event);
            } else {
                let correlation = correlate_volume_event(&app, event.at, window);
                let action = if correlation.matched {
                    "suppress_dji"
                } else {
                    "replay_uncorrelated"
                };
                decisions.push((event, action, correlation.nearest_press_delta_us));
            }
        }
        still_pending = !app.pending_volume.is_empty();
    }

    for (event, action, nearest_delta) in decisions {
        logger::log_args(
            Level::Info,
            format_args!(
                "VOLUME_HOOK t_us={} edge={} injected={} kbd_time_ms={} scan_code={:#X} mode={} action={} nearest_raw_press_delta_us={}",
                event_micros(event.at),
                if event.key_up { "up" } else { "down" },
                event.injected,
                event.keyboard_time_ms,
                event.scan_code,
                if event.observe_only {
                    "observe_only"
                } else {
                    event.mode.label()
                },
                action,
                nearest_delta.map_or_else(|| "none".to_owned(), |delta| delta.to_string())
            ),
        );
        if action == "replay_uncorrelated" {
            send_volume_event(event.key_up);
        }
    }
    if !still_pending {
        KillTimer(hwnd, TIMER_FLUSH_VOLUME);
    }
}

struct CorrelationResult {
    matched: bool,
    nearest_press_delta_us: Option<i128>,
}

fn correlate_volume_event(
    app: &AppState,
    event_time: Instant,
    window: Duration,
) -> CorrelationResult {
    let near_active = app.active_presses.values().any(|start| {
        event_time >= start.checked_sub(window).unwrap_or(*start) && event_time <= Instant::now()
    });
    let matched = near_active
        || app.completed_presses.iter().any(|interval| {
            event_time >= interval.start.checked_sub(window).unwrap_or(interval.start)
                && event_time <= interval.end + window
        });
    let nearest_press_delta_us = app
        .active_presses
        .values()
        .copied()
        .chain(app.completed_presses.iter().map(|interval| interval.start))
        .map(|press_time| signed_micros(event_time, press_time))
        .min_by_key(|delta| delta.unsigned_abs());
    CorrelationResult {
        matched,
        nearest_press_delta_us,
    }
}

unsafe fn send_chord(chord: &Chord) {
    let mut inputs = Vec::with_capacity(chord.modifiers.len() * 2 + 2);
    for modifier in &chord.modifiers {
        inputs.push(key_input(*modifier, false));
    }
    inputs.push(key_input(chord.key, false));
    inputs.push(key_input(chord.key, true));
    for modifier in chord.modifiers.iter().rev() {
        inputs.push(key_input(*modifier, true));
    }
    let sent = SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        size_of::<INPUT>() as i32,
    );
    if sent != inputs.len() as u32 {
        logger::log(Level::Error, &last_error("SendInput"));
    }
}

unsafe fn send_volume_event(key_up: bool) {
    let input = key_input(VK_VOLUME_UP, key_up);
    if SendInput(1, &input, size_of::<INPUT>() as i32) != 1 {
        logger::log(Level::Error, &last_error("SendInput(Volume Up)"));
    }
}

fn key_input(key: u16, key_up: bool) -> INPUT {
    let mut flags = if key_up { KEYEVENTF_KEYUP } else { 0 };
    if is_extended_key(key) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: INJECTED_MARKER,
            },
        },
    }
}

fn is_extended_key(key: u16) -> bool {
    matches!(key, 0x21..=0x28 | 0x2D | 0x2E | 0x5B | 0x5C | 0x6F | 0xA5)
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let status = current_status();
    let status_wide = wide(&status);
    let reload = wide("Reload config");
    let exit = wide("Exit");
    AppendMenuW(
        menu,
        MF_STRING | MF_GRAYED,
        MENU_STATUS,
        status_wide.as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, MENU_RELOAD, reload.as_ptr());
    AppendMenuW(menu, MF_STRING, MENU_EXIT, exit.as_ptr());

    let mut point = POINT { x: 0, y: 0 };
    GetCursorPos(&mut point);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, null());
    DestroyMenu(menu);
}

fn show_status_dialog() {
    let text = wide(&current_status());
    let title = wide(WINDOW_TITLE);
    unsafe {
        MessageBoxW(
            raw_window(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

fn show_warning(message: &str) {
    let text = wide(message);
    let title = wide(WINDOW_TITLE);
    unsafe {
        MessageBoxW(
            raw_window(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONWARNING,
        );
    }
}

unsafe fn add_or_update_tray_icon(hwnd: HWND, operation: u32) {
    if hwnd.is_null() {
        return;
    }
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY,
        hIcon: LoadIconW(null_mut(), IDI_APPLICATION),
        ..NOTIFYICONDATAW::default()
    };
    copy_wide(&mut data.szTip, &current_tooltip());
    Shell_NotifyIconW(operation, &data);
}

unsafe fn shutdown(hwnd: HWND) {
    deactivate_runtime();
    KillTimer(hwnd, TIMER_FLUSH_VOLUME);
    let data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..NOTIFYICONDATAW::default()
    };
    Shell_NotifyIconW(NIM_DELETE, &data);
}

fn current_status() -> String {
    match APP.get().and_then(|app| app.lock().ok()) {
        Some(app) => {
            if let Some(error) = &app.config_error {
                format!("Mapping disabled: {error}")
            } else if let Some(runtime) = &app.runtime {
                let mode = if app.diagnose { "diagnostic" } else { "active" };
                format!(
                    "{mode}; matching devices: {}; target: {}; volume: {}",
                    app.devices.len(),
                    runtime.chord.display,
                    runtime.config.effective_volume_up_mode().label()
                )
            } else {
                "Mapping disabled".to_owned()
            }
        }
        None => "Starting".to_owned(),
    }
}

fn current_tooltip() -> String {
    let status = current_status();
    let mut tooltip = format!("DJI Mic Mapper - {status}");
    if tooltip.encode_utf16().count() > 126 {
        tooltip = "DJI Mic Mapper - see menu for status".to_owned();
    }
    tooltip
}

fn current_config_error() -> Option<String> {
    APP.get()
        .and_then(|app| app.lock().ok())
        .and_then(|app| app.config_error.clone())
}

fn raw_window() -> HWND {
    APP.get()
        .and_then(|app| app.lock().ok())
        .map_or(null_mut(), |app| app.hwnd as HWND)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn copy_wide<const N: usize>(destination: &mut [u16; N], value: &str) {
    destination.fill(0);
    for (slot, character) in destination
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(value.encode_utf16())
    {
        *slot = character;
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn event_micros(instant: Instant) -> u128 {
    logger::instant_micros(instant).unwrap_or(0)
}

fn signed_micros(left: Instant, right: Instant) -> i128 {
    if left >= right {
        left.duration_since(right).as_micros() as i128
    } else {
        -(right.duration_since(left).as_micros() as i128)
    }
}

fn last_error(operation: &str) -> String {
    format!("{operation} failed: {}", std::io::Error::last_os_error())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_id_and_usage_are_both_required() {
        let config = Config::default();
        assert_eq!(report_button_state(&[6, 0xE9, 0x00], &config), Some(true));
        assert_eq!(report_button_state(&[6, 0x00, 0x00], &config), Some(false));
        assert_eq!(report_button_state(&[7, 0xE9, 0x00], &config), None);
    }

    #[test]
    fn other_consumer_usage_is_not_the_button() {
        let config = Config::default();
        assert_eq!(report_button_state(&[6, 0xEA, 0x00], &config), Some(false));
    }

    #[test]
    fn right_alt_uses_the_extended_key_flag() {
        assert!(is_extended_key(0xA5));
        assert!(!is_extended_key(0xA4));
        assert!(!is_extended_key(0xA1));
    }
}
