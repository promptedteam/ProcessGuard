#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::ffi::{OsStr, c_void};
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};
use std::slice;
use std::thread;
use std::time::Instant;

use windows_sys::Win32::Foundation::{
    COLORREF, CloseHandle, FILETIME, GetLastError, HANDLE, HINSTANCE, HWND, INVALID_HANDLE_VALUE,
    LPARAM, LRESULT, RECT, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_USE_IMMERSIVE_DARK_MODE,
    DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush,
    DEFAULT_CHARSET, DEFAULT_QUALITY, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE,
    DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, EndPaint, FF_DONTCARE, FW_BOLD, FillRect, HDC,
    HGDIOBJ, InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT, SRCCOPY, SelectObject, SetBkMode,
    SetTextColor, TRANSPARENT, UpdateWindow,
};
use windows_sys::Win32::Security::Credentials::{
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW,
    CredWriteW,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    BELOW_NORMAL_PRIORITY_CLASS, GetCurrentProcess, GetCurrentProcessId, GetProcessAffinityMask,
    GetProcessHandleCount, GetProcessIoCounters, GetProcessTimes, HIGH_PRIORITY_CLASS, IO_COUNTERS,
    NORMAL_PRIORITY_CLASS, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SET_INFORMATION, PROCESS_TERMINATE, PROCESS_VM_READ, QueryFullProcessImageNameW,
    SetPriorityClass, SetProcessAffinityMask, TerminateProcess,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, SetFocus, VK_CONTROL, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LEFT, VK_NEXT,
    VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW, Shell_NotifyIconW, ShellExecuteW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetClientRect, GetMessageW, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowRect, ICON_BIG, ICON_SMALL, IDC_ARROW, IMAGE_ICON, KillTimer,
    LR_LOADFROMFILE, LoadCursorW, LoadImageW, MB_DEFBUTTON2, MB_ICONERROR, MB_ICONINFORMATION,
    MB_ICONWARNING, MB_OK, MB_YESNO, MSG, MessageBoxW, PostMessageW, PostQuitMessage,
    RegisterClassW, SIZE_MINIMIZED, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_MAXIMIZE, SW_RESTORE,
    SW_SHOWNORMAL, SWP_NOMOVE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetTimer,
    SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage, WM_CHAR,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETICON,
    WM_SIZE, WM_TIMER, WNDCLASSW, WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};

const WM_REFRESH_PROCESSES: u32 = 0x8001;
const WM_AI_DONE: u32 = 0x8002;
const WM_TRAY_ICON: u32 = 0x8003;
const WM_SENTINEL_TEST_DONE: u32 = 0x8004;
const TIMER_AUTO_REFRESH: usize = 1;
const TRAY_UID: u32 = 1;
const AI_WINDOW_CLASS: &str = "ProcessGuardSentinelWindow";
const MONITOR_WINDOW_CLASS: &str = "ProcessGuardMonitorWindow";

const ROW_H: i32 = 30;
const HEADER_H: i32 = 30;
const TOP_H: i32 = 166;
const STATUS_H: i32 = 26;
const DETAILS_W: i32 = 365;
const HSCROLL_H: i32 = 15;
const FROZEN_NAME_W: i32 = 280;
const MENU_ITEM_H: i32 = 44;
const CREATE_NO_WINDOW: u32 = 0x08000000;
const AI_NAME: &str = "Sentinel";
const AI_TITLE: &str = "SENTINEL AI CHAT";
const SPEAKER_AI: &str = "Sentinel";
const SPEAKER_USER: &str = "You";
const SPEAKER_LOCAL: &str = "Local scan";
const PROCESS_SUSPEND_RESUME_ACCESS: u32 = 0x0800;
const APP_VERSION: &str = "1.0.2";
const CF_UNICODETEXT: u32 = 13;
const SENTINEL_CREDENTIAL_PREFIX: &str = "ProcessGuard/Sentinel/";

#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtSuspendProcess(process: HANDLE) -> i32;
    fn NtResumeProcess(process: HANDLE) -> i32;
}
const SUGGESTED_QUESTIONS: [(&str, &str); 4] = [
    (
        "Safe to end?",
        "Is it safe to end this process, and what will happen?",
    ),
    (
        "High RAM?",
        "Why is this using so much RAM, and what can I do?",
    ),
    (
        "What is it?",
        "What app or Windows feature does this process belong to?",
    ),
    (
        "Reduce usage",
        "How can I reduce its resource usage without breaking anything?",
    ),
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SafetyLevel {
    Safe,
    Caution,
    Unknown,
    Blocked,
}

impl SafetyLevel {
    fn label(self) -> &'static str {
        match self {
            SafetyLevel::Safe => "Safe",
            SafetyLevel::Caution => "Caution",
            SafetyLevel::Unknown => "Unknown",
            SafetyLevel::Blocked => "Blocked",
        }
    }

    fn can_end(self) -> bool {
        self == SafetyLevel::Safe
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortMode {
    Memory,
    Cpu,
    Io,
    Network,
    Threads,
    Handles,
    Risk,
    Name,
}

#[derive(Clone, PartialEq, Eq)]
enum FieldFilter {
    Type(String),
    Safety(SafetyLevel),
    SafeOnly,
}

#[derive(Clone)]
struct ProcessInfo {
    pid: u32,
    parent_pid: u32,
    name: String,
    category: String,
    description: String,
    company: String,
    product: String,
    original_filename: String,
    path: String,
    memory_kb: u64,
    cpu_total_100ns: u64,
    io_total_bytes: u64,
    cpu_percent: f32,
    io_rate_kbps: f32,
    network_connections: u32,
    thread_count: u32,
    handle_count: u32,
    safety: SafetyLevel,
    reason: String,
    risk_score: u8,
}

#[derive(Clone, Default)]
struct FileMetadata {
    description: String,
    company: String,
    product: String,
    original_filename: String,
}

struct ProcessGroup {
    key: String,
    name: String,
    category: String,
    description: String,
    company: String,
    product: String,
    path: String,
    process_indices: Vec<usize>,
    total_memory_kb: u64,
    total_cpu_percent: f32,
    total_io_rate_kbps: f32,
    total_network_connections: u32,
    total_threads: u32,
    total_handles: u32,
    safe_count: usize,
    caution_count: usize,
    unknown_count: usize,
    blocked_count: usize,
    max_risk_score: u8,
}

#[derive(Clone, Copy)]
enum RowKind {
    Group(usize),
    Process(usize),
}

#[derive(Clone, Copy)]
enum RuleKind {
    Watch,
    Alert,
    Automation,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Refresh,
    EndSafe,
    Explain,
    History,
    ToggleView,
    SelectSafe,
    Export,
    AutoRefresh,
    Admin,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuKind {
    File,
    View,
    Search,
    Monitor,
    Control,
    Tools,
    Help,
}

impl MenuKind {
    fn label(self) -> &'static str {
        match self {
            MenuKind::File => "File",
            MenuKind::View => "View",
            MenuKind::Search => "Search",
            MenuKind::Monitor => "Monitor",
            MenuKind::Control => "Control",
            MenuKind::Tools => "Tools",
            MenuKind::Help => "Help",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuCommand {
    Action(Action),
    FocusSearch,
    ClearFilters,
    StartProcess,
    MinimizeTray,
    Exit,
    ExpandSelected,
    CollapseAll,
    ClearSelection,
    OpenLocation,
    FilterSelectedType,
    LivePerformance,
    ProcessTree,
    NetworkInspector,
    VerifySignature,
    ToggleAlerts,
    ToggleWatchlist,
    SaveSnapshot,
    CompareSnapshot,
    FullExecutableDetails,
    PriorityHigh,
    PriorityNormal,
    EfficiencyMode,
    LimitAffinity,
    SuspendSelected,
    ResumeSelected,
    ToggleAutomation,
    StartupManager,
    ServicesManager,
    SentinelSecurityReport,
    SentinelSettings,
    About,
}

fn menu_command_accent(command: MenuCommand) -> COLORREF {
    match command {
        MenuCommand::Action(Action::EndSafe)
        | MenuCommand::SuspendSelected
        | MenuCommand::ResumeSelected => C_BLOCK_TEXT,
        MenuCommand::PriorityHigh
        | MenuCommand::PriorityNormal
        | MenuCommand::EfficiencyMode
        | MenuCommand::LimitAffinity
        | MenuCommand::ToggleAutomation => C_WARN_TEXT,
        MenuCommand::LivePerformance
        | MenuCommand::ProcessTree
        | MenuCommand::NetworkInspector
        | MenuCommand::VerifySignature
        | MenuCommand::ToggleAlerts
        | MenuCommand::ToggleWatchlist
        | MenuCommand::SaveSnapshot
        | MenuCommand::CompareSnapshot
        | MenuCommand::FullExecutableDetails => C_GREEN,
        MenuCommand::Action(Action::Explain)
        | MenuCommand::Action(Action::History)
        | MenuCommand::SentinelSecurityReport
        | MenuCommand::SentinelSettings => C_CYAN,
        _ => C_BORDER,
    }
}

fn menu_command_description(command: MenuCommand) -> &'static str {
    match command {
        MenuCommand::Action(Action::Refresh) => "Rescan processes and update every live metric.",
        MenuCommand::Action(Action::EndSafe) => "End selected Safe processes after confirmation.",
        MenuCommand::Action(Action::Explain) => {
            "Open Sentinel AI for the selected process context."
        }
        MenuCommand::Action(Action::History) => "Open previous Sentinel AI conversations.",
        MenuCommand::Action(Action::ToggleView) => {
            "Switch between app groups and individual processes."
        }
        MenuCommand::Action(Action::SelectSafe) => "Select all currently visible Safe targets.",
        MenuCommand::Action(Action::Export) => "Write the complete scan to a local text report.",
        MenuCommand::Action(Action::AutoRefresh) => {
            "Update live process metrics every five seconds."
        }
        MenuCommand::Action(Action::Admin) => {
            "Restart Process Guard with the opposite privilege mode."
        }
        MenuCommand::FocusSearch => "Move keyboard focus to process search.",
        MenuCommand::ClearFilters => "Remove search text and all column filters.",
        MenuCommand::StartProcess => "Start an executable, path, or Windows command.",
        MenuCommand::MinimizeTray => "Keep monitoring while hidden in the system tray.",
        MenuCommand::Exit => "Close Process Guard and its detached windows.",
        MenuCommand::ExpandSelected => "Show child PIDs inside selected application groups.",
        MenuCommand::CollapseAll => "Close every expanded application group.",
        MenuCommand::ClearSelection => "Remove the current process selection.",
        MenuCommand::OpenLocation => "Open Explorer at the selected executable.",
        MenuCommand::FilterSelectedType => "Show only processes matching the selected type.",
        MenuCommand::LivePerformance => "Open Monitor Center with live metrics and CPU graphs.",
        MenuCommand::ProcessTree => "Show selected parent, ancestor, and child relationships.",
        MenuCommand::NetworkInspector => "List selected TCP connections and UDP endpoints.",
        MenuCommand::VerifySignature => "Check executable signatures and certificate owners.",
        MenuCommand::ToggleAlerts => "Alert on high CPU, RAM, or disk use for selected apps.",
        MenuCommand::ToggleWatchlist => "Notify when selected apps start or stop.",
        MenuCommand::SaveSnapshot => "Save the current process state for later comparison.",
        MenuCommand::CompareSnapshot => "Show new, missing, and changed processes.",
        MenuCommand::FullExecutableDetails => {
            "Show owner, command line, hash, version, and file data."
        }
        MenuCommand::PriorityHigh => "Give selected non-core processes more CPU priority.",
        MenuCommand::PriorityNormal => "Restore selected processes to normal priority.",
        MenuCommand::EfficiencyMode => "Lower selected process priority to reduce resource use.",
        MenuCommand::LimitAffinity => "Limit selected processes to part of the available CPUs.",
        MenuCommand::SuspendSelected => "Freeze only selected processes classified Safe.",
        MenuCommand::ResumeSelected => "Resume selected processes that were suspended.",
        MenuCommand::ToggleAutomation => "Automatically lower heavy Safe apps to efficiency mode.",
        MenuCommand::StartupManager => "Open Windows controls for startup applications.",
        MenuCommand::ServicesManager => "Open Windows controls for system services.",
        MenuCommand::SentinelSecurityReport => "Ask Sentinel AI for one combined risk report.",
        MenuCommand::SentinelSettings => {
            "Choose Sentinel's CLI or API provider, model, and local credential."
        }
        MenuCommand::About => "Show application purpose and version information.",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Column {
    Name,
    Items,
    Type,
    Cpu,
    Memory,
    Io,
    Network,
    Threads,
    Handles,
    Risk,
    Safety,
    Reason,
}

struct Layout {
    menus: Vec<(MenuKind, RECT)>,
    search: RECT,
    search_clear: RECT,
    table: RECT,
    details: RECT,
    status: RECT,
    buttons: Vec<(Action, RECT)>,
    columns: Vec<(Column, RECT)>,
}

#[derive(Clone)]
struct AiMessage {
    speaker: &'static str,
    text: String,
}

impl AiMessage {
    fn new(speaker: &'static str, text: String) -> Self {
        Self { speaker, text }
    }
}

#[derive(Clone)]
struct AiSession {
    title: String,
    context: String,
    messages: Vec<AiMessage>,
    pinned: bool,
}

impl AiSession {
    fn new(title: String, context: String, messages: Vec<AiMessage>) -> Self {
        Self {
            title,
            context,
            messages,
            pinned: false,
        }
    }
}

struct AiRenderLine {
    text: String,
    color: COLORREF,
    bold: bool,
    marker: Option<COLORREF>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HoverTarget {
    Main(Action),
    ClearSearch,
    AiAsk,
    AiRegenerate,
    AiPin,
    AiClearOne,
    AiClearAll,
    AiSmaller,
    AiLarger,
    AiFit,
    AiResize,
    AiSuggestion(usize),
    AiSession(usize),
}

impl HoverTarget {
    fn text(self) -> &'static str {
        match self {
            HoverTarget::Main(Action::Refresh) => "Refresh the process list now.",
            HoverTarget::Main(Action::EndSafe) => {
                "End only selected processes marked Safe. Blocked/caution/unknown are skipped."
            }
            HoverTarget::Main(Action::Explain) => {
                "Open Sentinel chat for the selected process, or reopen saved Sentinel history."
            }
            HoverTarget::Main(Action::History) => {
                "Open saved Sentinel chats without starting a new analysis."
            }
            HoverTarget::Main(Action::ToggleView) => {
                "Switch between grouped apps and individual process rows."
            }
            HoverTarget::Main(Action::SelectSafe) => {
                "Select every visible row that contains at least one Safe target."
            }
            HoverTarget::Main(Action::Export) => "Save a text report of the current process scan.",
            HoverTarget::Main(Action::AutoRefresh) => {
                "Toggle automatic refresh every five seconds."
            }
            HoverTarget::Main(Action::Admin) => {
                "Toggle admin mode by reopening Process Guard in the opposite privilege mode."
            }
            HoverTarget::ClearSearch => "Clear the current search text.",
            HoverTarget::AiAsk => "Send the typed follow-up question to Sentinel.",
            HoverTarget::AiRegenerate => "Ask Sentinel to regenerate the latest explanation.",
            HoverTarget::AiPin => "Pin or unpin the current Sentinel chat.",
            HoverTarget::AiClearOne => {
                "Delete the selected Sentinel chat. Pinned chats must be unpinned first."
            }
            HoverTarget::AiClearAll => "Delete every unpinned Sentinel chat. Pinned chats remain.",
            HoverTarget::AiSmaller => "Make the Sentinel chat popup smaller.",
            HoverTarget::AiLarger => "Make the Sentinel chat popup larger.",
            HoverTarget::AiFit => "Reset the Sentinel popup size and center it on this screen.",
            HoverTarget::AiResize => "Drag to resize the Sentinel chat popup.",
            HoverTarget::AiSuggestion(index) => SUGGESTED_QUESTIONS
                .get(index)
                .map(|(_, question)| *question)
                .unwrap_or("Use this quick follow-up question."),
            HoverTarget::AiSession(_) => "Open this saved Sentinel chat.",
        }
    }
}

struct AiLayout {
    popup: RECT,
    drag: RECT,
    resize: RECT,
    history: RECT,
    body: RECT,
    input: RECT,
    ask: RECT,
    regenerate: RECT,
    pin: RECT,
    clear_one: RECT,
    clear_all: RECT,
    smaller: RECT,
    larger: RECT,
    fit: RECT,
    suggestions: [RECT; 4],
}

#[derive(Clone, Copy)]
struct ContextMenu {
    x: i32,
    y: i32,
}

struct LauncherLayout {
    popup: RECT,
    input: RECT,
    start: RECT,
    cancel: RECT,
    close: RECT,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SentinelBackendKind {
    CodexCli,
    ClaudeCli,
    GeminiCli,
    OpenAi,
    Anthropic,
    GeminiApi,
    XAi,
    Groq,
    Mistral,
    Cohere,
    DeepSeek,
    OpenRouter,
    Together,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SentinelModelTier {
    Included,
    Free,
    Standard,
    Premium,
}

impl SentinelModelTier {
    fn label(self) -> &'static str {
        match self {
            SentinelModelTier::Included => "INCLUDED",
            SentinelModelTier::Free => "FREE TIER",
            SentinelModelTier::Standard => "STANDARD",
            SentinelModelTier::Premium => "PREMIUM",
        }
    }

    fn color(self) -> COLORREF {
        match self {
            SentinelModelTier::Included | SentinelModelTier::Free => C_GREEN,
            SentinelModelTier::Standard => C_CYAN,
            SentinelModelTier::Premium => C_WARN_TEXT,
        }
    }
}

struct SentinelProvider {
    id: &'static str,
    label: &'static str,
    kind: SentinelBackendKind,
    description: &'static str,
    models: &'static [&'static str],
}

impl SentinelProvider {
    fn is_api(&self) -> bool {
        !matches!(
            self.kind,
            SentinelBackendKind::CodexCli
                | SentinelBackendKind::ClaudeCli
                | SentinelBackendKind::GeminiCli
        )
    }

    fn mode_label(&self) -> &'static str {
        if self.is_api() { "API" } else { "CLI" }
    }
}

const SENTINEL_PROVIDERS: &[SentinelProvider] = &[
    SentinelProvider {
        id: "codex-cli",
        label: "Codex CLI",
        kind: SentinelBackendKind::CodexCli,
        description: "Uses the locally installed Codex CLI and its existing sign-in. No API key is stored by Process Guard.",
        models: &["default", "gpt-5.3-codex", "gpt-5.6"],
    },
    SentinelProvider {
        id: "claude-cli",
        label: "Claude CLI",
        kind: SentinelBackendKind::ClaudeCli,
        description: "Uses the locally installed Claude Code CLI and its existing authentication in read-only plan mode.",
        models: &["default", "sonnet", "opus", "haiku"],
    },
    SentinelProvider {
        id: "gemini-cli",
        label: "Gemini CLI",
        kind: SentinelBackendKind::GeminiCli,
        description: "Uses the locally installed Gemini CLI and its existing authentication for headless text responses.",
        models: &[
            "auto",
            "gemini-3.5-flash",
            "gemini-3.1-pro-preview",
            "gemini-3.1-flash-lite",
        ],
    },
    SentinelProvider {
        id: "openai",
        label: "OpenAI",
        kind: SentinelBackendKind::OpenAi,
        description: "Connects to the OpenAI Chat Completions API with your own OpenAI API key.",
        models: &[
            "gpt-5.4-mini",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.6-sol",
        ],
    },
    SentinelProvider {
        id: "anthropic",
        label: "Anthropic",
        kind: SentinelBackendKind::Anthropic,
        description: "Connects directly to Anthropic Messages API with your own Anthropic API key.",
        models: &["claude-haiku-4-5", "claude-sonnet-4-6", "claude-opus-4-6"],
    },
    SentinelProvider {
        id: "gemini-api",
        label: "Google Gemini",
        kind: SentinelBackendKind::GeminiApi,
        description: "Connects directly to the Google Gemini generateContent API with your Gemini API key.",
        models: &[
            "gemini-3.5-flash",
            "gemini-3.1-flash-lite",
            "gemini-3.1-pro-preview",
        ],
    },
    SentinelProvider {
        id: "xai",
        label: "xAI",
        kind: SentinelBackendKind::XAi,
        description: "Connects to the xAI OpenAI-compatible API with your own xAI API key.",
        models: &["grok-4.3", "grok-latest", "grok-4.5"],
    },
    SentinelProvider {
        id: "groq",
        label: "Groq",
        kind: SentinelBackendKind::Groq,
        description: "Connects to Groq's OpenAI-compatible API for low-latency hosted models.",
        models: &[
            "openai/gpt-oss-20b",
            "openai/gpt-oss-120b",
            "qwen/qwen3.6-27b",
        ],
    },
    SentinelProvider {
        id: "mistral",
        label: "Mistral AI",
        kind: SentinelBackendKind::Mistral,
        description: "Connects to Mistral's chat API with your own Mistral API key.",
        models: &[
            "mistral-small-latest",
            "codestral-latest",
            "mistral-large-latest",
        ],
    },
    SentinelProvider {
        id: "cohere",
        label: "Cohere",
        kind: SentinelBackendKind::Cohere,
        description: "Connects to Cohere Chat v2 with your own Cohere API key.",
        models: &["command-a-03-2025", "command-a-plus-05-2026"],
    },
    SentinelProvider {
        id: "deepseek",
        label: "DeepSeek",
        kind: SentinelBackendKind::DeepSeek,
        description: "Connects to the DeepSeek OpenAI-compatible chat API with your own API key.",
        models: &["deepseek-v4-flash", "deepseek-v4-pro"],
    },
    SentinelProvider {
        id: "openrouter",
        label: "OpenRouter",
        kind: SentinelBackendKind::OpenRouter,
        description: "Uses one OpenRouter key to route Sentinel requests to a model available in your OpenRouter account.",
        models: &[
            "openrouter/free",
            "openrouter/auto",
            "anthropic/claude-sonnet-4.6",
            "openai/gpt-5.6-terra",
        ],
    },
    SentinelProvider {
        id: "together",
        label: "Together AI",
        kind: SentinelBackendKind::Together,
        description: "Connects to Together AI's OpenAI-compatible endpoint with your own API key.",
        models: &[
            "openai/gpt-oss-120b",
            "Qwen/Qwen3.6-Plus",
            "moonshotai/Kimi-K2.6",
            "deepseek-ai/DeepSeek-V4-Pro",
        ],
    },
];

#[derive(Clone)]
struct SentinelConfig {
    provider_id: String,
    model: String,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            provider_id: "codex-cli".to_string(),
            model: "default".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SentinelSettingsFocus {
    Model,
    ApiKey,
}

struct SentinelSettingsLayout {
    popup: RECT,
    close: RECT,
    providers: RECT,
    model_input: RECT,
    model_choices: [RECT; 4],
    key_input: RECT,
    clear_key: RECT,
    save: RECT,
    cancel: RECT,
}

struct AppState {
    processes: Vec<ProcessInfo>,
    groups: Vec<ProcessGroup>,
    visible_rows: Vec<RowKind>,
    expanded_groups: BTreeSet<String>,
    selected: BTreeSet<String>,
    grouped_view: bool,
    search: String,
    search_focus: bool,
    field_filter: Option<FieldFilter>,
    sort_mode: SortMode,
    scroll: usize,
    hscroll: i32,
    status: String,
    details: String,
    ai_text: String,
    ai_context: String,
    ai_input: String,
    ai_cursor: usize,
    ai_input_all_selected: bool,
    ai_input_focus: bool,
    ai_messages: Vec<AiMessage>,
    ai_sessions: Vec<AiSession>,
    active_ai_session: Option<usize>,
    ai_body_scroll: usize,
    ai_session_scroll: usize,
    ai_popup_pos: Option<(i32, i32)>,
    ai_popup_size: Option<(i32, i32)>,
    ai_hwnd: HWND,
    ai_dragging: bool,
    ai_resizing: bool,
    ai_drag_dx: i32,
    ai_drag_dy: i32,
    ai_resize_dx: i32,
    ai_resize_dy: i32,
    ai_running: bool,
    ai_overlay: bool,
    sentinel_config: SentinelConfig,
    sentinel_settings_open: bool,
    sentinel_selected_provider: usize,
    sentinel_model_input: String,
    sentinel_key_input: String,
    sentinel_settings_focus: SentinelSettingsFocus,
    sentinel_text_all_selected: bool,
    sentinel_key_stored: bool,
    sentinel_test_running: bool,
    sentinel_test_message: String,
    sentinel_test_success: bool,
    installed_copy: bool,
    monitor_hwnd: HWND,
    monitor_title: String,
    monitor_report: String,
    monitor_scroll: usize,
    launcher_open: bool,
    launcher_input: String,
    launcher_focus: bool,
    open_menu: Option<MenuKind>,
    context_menu: Option<ContextMenu>,
    tray_visible: bool,
    elevated: bool,
    auto_refresh: bool,
    performance_previous: HashMap<u32, (u64, u64)>,
    performance_history: HashMap<u32, VecDeque<f32>>,
    last_performance_sample: Instant,
    watchlist: BTreeSet<String>,
    alert_rules: BTreeSet<String>,
    automation_rules: BTreeSet<String>,
    active_alerts: BTreeSet<u32>,
    last_watch_counts: HashMap<String, usize>,
    last_pids: BTreeSet<u32>,
    hover: Option<HoverTarget>,
    hover_x: i32,
    hover_y: i32,
}

fn main() {
    unsafe {
        run_app();
    }
}

unsafe fn run_app() {
    let instance = GetModuleHandleW(null()) as HINSTANCE;
    let class_name = wide("ProcessGuardHackerWindow");
    let ai_class_name = wide(AI_WINDOW_CLASS);
    let monitor_class_name = wide(MONITOR_WINDOW_CLASS);
    let title = wide("Process Guard");

    let window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: null_mut(),
        lpszClassName: class_name.as_ptr(),
        ..zeroed()
    };

    RegisterClassW(&window_class);

    let ai_window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
        lpfnWndProc: Some(ai_window_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: null_mut(),
        lpszClassName: ai_class_name.as_ptr(),
        ..zeroed()
    };

    RegisterClassW(&ai_window_class);

    let monitor_window_class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(monitor_window_proc),
        hInstance: instance,
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: null_mut(),
        lpszClassName: monitor_class_name.as_ptr(),
        ..zeroed()
    };

    RegisterClassW(&monitor_window_class);

    let sw = GetSystemMetrics(SM_CXSCREEN).max(980);
    let sh = GetSystemMetrics(SM_CYSCREEN).max(680);
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        (sw - 80).clamp(960, 1280),
        (sh - 90).clamp(680, 820),
        null_mut(),
        null_mut(),
        instance,
        null_mut(),
    );

    if hwnd == null_mut() {
        return;
    }

    load_app_icon(hwnd);
    apply_dark_title_bar(hwnd);
    ShowWindow(hwnd, SW_MAXIMIZE);
    UpdateWindow(hwnd);

    let mut message: MSG = zeroed();
    while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
}

unsafe extern "system" fn ai_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let main_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
    match message {
        WM_PAINT => {
            if let Some(state) = app_state(main_hwnd) {
                state.paint_ai_window(hwnd);
            } else {
                DefWindowProcW(hwnd, message, wparam, lparam);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
            if let Some(state) = app_state(main_hwnd) {
                SetFocus(hwnd);
                let (x, y) = mouse_xy(lparam);
                state.on_ai_click(hwnd, main_hwnd, x, y);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_LBUTTONUP => {
            if let Some(state) = app_state(main_hwnd) {
                state.ai_dragging = false;
                state.ai_resizing = false;
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Some(state) = app_state(main_hwnd) {
                let (x, y) = mouse_xy(lparam);
                if state.on_ai_mouse_move(hwnd, x, y) {
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_MOUSEWHEEL => {
            if let Some(state) = app_state(main_hwnd) {
                let mut client: RECT = zeroed();
                GetClientRect(hwnd, &mut client);
                let delta = ((wparam >> 16) as i16) as i32;
                state.scroll_ai_at_hover(client, delta);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_CHAR => {
            if let Some(state) = app_state(main_hwnd) {
                state.handle_ai_char(main_hwnd, wparam as u32);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_KEYDOWN => {
            if let Some(state) = app_state(main_hwnd) {
                if wparam as u16 == VK_ESCAPE {
                    unsafe { state.close_ai_window() };
                } else {
                    state.handle_ai_keydown(main_hwnd, wparam as u16);
                }
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_SIZE => {
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_DESTROY => {
            if let Some(state) = app_state(main_hwnd) {
                if state.ai_hwnd == hwnd {
                    state.ai_hwnd = null_mut();
                    state.ai_overlay = false;
                    state.ai_input_focus = false;
                    state.ai_dragging = false;
                    state.ai_resizing = false;
                    InvalidateRect(main_hwnd, null(), 0);
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe extern "system" fn monitor_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let main_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
    match message {
        WM_PAINT => {
            if let Some(state) = app_state(main_hwnd) {
                state.paint_monitor_window(hwnd);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_MOUSEWHEEL => {
            if let Some(state) = app_state(main_hwnd) {
                let mut client: RECT = zeroed();
                GetClientRect(hwnd, &mut client);
                let delta = ((wparam >> 16) as i16) as i32;
                if delta > 0 {
                    state.monitor_scroll = state.monitor_scroll.saturating_sub(3);
                } else {
                    state.monitor_scroll =
                        (state.monitor_scroll + 3).min(state.monitor_max_scroll(client));
                }
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_KEYDOWN => {
            if let Some(state) = app_state(main_hwnd) {
                let mut client: RECT = zeroed();
                GetClientRect(hwnd, &mut client);
                match wparam as u16 {
                    VK_ESCAPE => {
                        DestroyWindow(hwnd);
                    }
                    VK_UP => state.monitor_scroll = state.monitor_scroll.saturating_sub(1),
                    VK_DOWN => {
                        state.monitor_scroll =
                            (state.monitor_scroll + 1).min(state.monitor_max_scroll(client))
                    }
                    VK_PRIOR => state.monitor_scroll = state.monitor_scroll.saturating_sub(12),
                    VK_NEXT => {
                        state.monitor_scroll =
                            (state.monitor_scroll + 12).min(state.monitor_max_scroll(client))
                    }
                    VK_HOME => state.monitor_scroll = 0,
                    VK_END => state.monitor_scroll = state.monitor_max_scroll(client),
                    _ => {}
                }
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_SIZE => {
            if let Some(state) = app_state(main_hwnd) {
                let mut client: RECT = zeroed();
                GetClientRect(hwnd, &mut client);
                state.monitor_scroll = state.monitor_scroll.min(state.monitor_max_scroll(client));
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_DESTROY => {
            if let Some(state) = app_state(main_hwnd) {
                if state.monitor_hwnd == hwnd {
                    state.monitor_hwnd = null_mut();
                    state.monitor_scroll = 0;
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let mut state = Box::new(AppState::new());
            state.status = "Scanning processes...".to_string();
            if state.auto_refresh {
                SetTimer(hwnd, TIMER_AUTO_REFRESH, 5000, None);
            }
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            PostMessageW(hwnd, WM_REFRESH_PROCESSES, 0, 0);
            0
        }
        WM_REFRESH_PROCESSES => {
            if let Some(state) = app_state(hwnd) {
                state.refresh();
                state.clamp_scroll_to_window(hwnd);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_AI_DONE => {
            if let Some(state) = app_state(hwnd) {
                if lparam != 0 {
                    let text = Box::from_raw(lparam as *mut String);
                    let answer = clean_ai_message(&text);
                    state.ai_text = answer.clone();
                    state.ai_messages.push(AiMessage::new(SPEAKER_AI, answer));
                    state.ai_body_scroll = 0;
                    state.sync_active_ai_session();
                    state.ai_running = false;
                    state.ai_overlay = true;
                    state.ensure_ai_window(hwnd);
                    state.status =
                        "Sentinel answer ready. Use the separate Sentinel window for follow-ups."
                            .to_string();
                    InvalidateRect(hwnd, null(), 0);
                    if !state.ai_hwnd.is_null() {
                        InvalidateRect(state.ai_hwnd, null(), 0);
                    }
                }
            }
            0
        }
        WM_SENTINEL_TEST_DONE => {
            if let Some(state) = app_state(hwnd) {
                if lparam != 0 {
                    let result = Box::from_raw(lparam as *mut Result<String, String>);
                    state.sentinel_test_running = false;
                    match *result {
                        Ok(message) => {
                            state.sentinel_test_success = true;
                            state.sentinel_test_message = message;
                            state.commit_sentinel_settings(hwnd);
                        }
                        Err(message) => {
                            state.sentinel_test_success = false;
                            state.sentinel_test_message = message;
                            state.status =
                                "Sentinel test failed. Settings were not changed.".to_string();
                        }
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_TRAY_ICON => {
            if let Some(state) = app_state(hwnd) {
                let event = lparam as u32;
                if event == WM_LBUTTONDBLCLK || event == WM_LBUTTONDOWN || event == WM_RBUTTONUP {
                    state.restore_from_tray(hwnd);
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_PAINT => {
            if let Some(state) = app_state(hwnd) {
                state.paint(hwnd);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_LBUTTONDOWN => {
            if let Some(state) = app_state(hwnd) {
                SetFocus(hwnd);
                let (x, y) = mouse_xy(lparam);
                state.on_click(hwnd, x, y, false);
                state.clamp_scroll_to_window(hwnd);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            if let Some(state) = app_state(hwnd) {
                let (x, y) = mouse_xy(lparam);
                state.on_click(hwnd, x, y, true);
                state.clamp_scroll_to_window(hwnd);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_LBUTTONUP => {
            if let Some(state) = app_state(hwnd) {
                state.ai_dragging = false;
                state.ai_resizing = false;
            }
            0
        }
        WM_RBUTTONDOWN => {
            if let Some(state) = app_state(hwnd) {
                SetFocus(hwnd);
                let (x, y) = mouse_xy(lparam);
                state.on_right_click(hwnd, x, y);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_MOUSEMOVE => {
            if let Some(state) = app_state(hwnd) {
                let (x, y) = mouse_xy(lparam);
                if state.on_mouse_move(hwnd, x, y) {
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_MOUSEWHEEL => {
            if let Some(state) = app_state(hwnd) {
                let mut client: RECT = zeroed();
                GetClientRect(hwnd, &mut client);
                if state.sentinel_settings_open {
                    return 0;
                }
                if state.ai_overlay && state.ai_hwnd.is_null() {
                    let delta = ((wparam >> 16) as i16) as i32;
                    state.scroll_ai_at_hover(client, delta);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                let delta = ((wparam >> 16) as i16) as i32;
                if GetKeyState(VK_SHIFT as i32) < 0 {
                    let table = state.layout(client).table;
                    let max = state.max_hscroll_for_table(table);
                    if delta > 0 {
                        state.hscroll = state.hscroll.saturating_sub(120);
                    } else {
                        state.hscroll = (state.hscroll + 120).min(max);
                    }
                    state.status =
                        "Horizontal table scroll: Process/Group remains frozen.".to_string();
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                let max_scroll = state.max_scroll_for_client(client);
                if delta > 0 {
                    state.scroll = state.scroll.saturating_sub(3);
                } else {
                    state.scroll = (state.scroll + 3).min(max_scroll);
                }
                state.scroll = state.scroll.min(max_scroll);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_CHAR => {
            if let Some(state) = app_state(hwnd) {
                if state.sentinel_settings_open {
                    state.handle_sentinel_settings_char(wparam as u32);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                if state.launcher_open && state.launcher_focus {
                    state.handle_launcher_char(hwnd, wparam as u32);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                if state.ai_overlay && state.ai_hwnd.is_null() && state.ai_input_focus {
                    state.handle_ai_char(hwnd, wparam as u32);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                if state.search_focus {
                    match wparam as u32 {
                        8 => {
                            state.search.pop();
                            state.apply_filters();
                            state.clamp_scroll_to_window(hwnd);
                        }
                        13 => {
                            state.search_focus = false;
                        }
                        27 => {
                            state.search.clear();
                            state.field_filter = None;
                            state.search_focus = false;
                            state.apply_filters();
                            state.clamp_scroll_to_window(hwnd);
                        }
                        ch if ch >= 32 && ch < 127 => {
                            if let Some(ch) = char::from_u32(ch) {
                                if state.search.len() < 64 {
                                    state.search.push(ch);
                                    state.field_filter = None;
                                    state.apply_filters();
                                    state.clamp_scroll_to_window(hwnd);
                                }
                            }
                        }
                        _ => {}
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_KEYDOWN => {
            if let Some(state) = app_state(hwnd) {
                if state.sentinel_settings_open {
                    state.handle_sentinel_settings_keydown(hwnd, wparam as u16);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                if state.ai_overlay
                    && state.ai_hwnd.is_null()
                    && state.ai_input_focus
                    && wparam as u16 != VK_ESCAPE
                {
                    state.handle_ai_keydown(hwnd, wparam as u16);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                match wparam as u16 {
                    VK_ESCAPE => {
                        if state.ai_hwnd.is_null() {
                            state.ai_overlay = false;
                            state.ai_input_focus = false;
                        } else {
                            state.close_ai_window();
                        }
                        state.launcher_open = false;
                        state.launcher_focus = false;
                        state.search_focus = false;
                        state.open_menu = None;
                        state.context_menu = None;
                    }
                    VK_LEFT => state.hscroll = state.hscroll.saturating_sub(80),
                    VK_RIGHT => {
                        let mut client: RECT = zeroed();
                        GetClientRect(hwnd, &mut client);
                        let table = state.layout(client).table;
                        state.hscroll =
                            (state.hscroll + 80).min(state.max_hscroll_for_table(table));
                    }
                    VK_UP => {
                        if !state.ai_overlay || !state.ai_hwnd.is_null() {
                            state.scroll = state.scroll.saturating_sub(1)
                        }
                    }
                    VK_DOWN => {
                        if !state.ai_overlay || !state.ai_hwnd.is_null() {
                            let mut client: RECT = zeroed();
                            GetClientRect(hwnd, &mut client);
                            state.scroll =
                                (state.scroll + 1).min(state.max_scroll_for_client(client))
                        }
                    }
                    VK_PRIOR => {
                        if !state.ai_overlay || !state.ai_hwnd.is_null() {
                            state.scroll = state.scroll.saturating_sub(12)
                        }
                    }
                    VK_NEXT => {
                        if !state.ai_overlay || !state.ai_hwnd.is_null() {
                            let mut client: RECT = zeroed();
                            GetClientRect(hwnd, &mut client);
                            state.scroll =
                                (state.scroll + 12).min(state.max_scroll_for_client(client))
                        }
                    }
                    VK_RETURN => {
                        if state.launcher_open && state.launcher_focus {
                            state.start_launcher_process(hwnd);
                        } else if state.ai_overlay
                            && state.ai_hwnd.is_null()
                            && state.ai_input_focus
                        {
                            state.submit_ai_question(hwnd);
                        } else if !state.ai_overlay || !state.ai_hwnd.is_null() {
                            state.expand_selected_groups();
                        }
                    }
                    _ => {}
                }
                state.clamp_scroll_to_window(hwnd);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_TIMER => {
            if wparam == TIMER_AUTO_REFRESH {
                if let Some(state) = app_state(hwnd) {
                    state.refresh();
                    if state.tray_visible
                        && (state.status.contains("resource alert")
                            || state.status.contains("watchlist"))
                    {
                        show_tray_notification(hwnd, "Process Guard Alert", &state.status);
                    }
                    state.clamp_scroll_to_window(hwnd);
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_SIZE => {
            if let Some(state) = app_state(hwnd) {
                if wparam as u32 == SIZE_MINIMIZED {
                    state.minimize_to_tray(hwnd);
                    return 0;
                }
                state.clamp_scroll_to_window(hwnd);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_AUTO_REFRESH);
            remove_tray_icon(hwnd);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
            if !ptr.is_null() {
                if !(*ptr).ai_hwnd.is_null() {
                    DestroyWindow((*ptr).ai_hwnd);
                    (*ptr).ai_hwnd = null_mut();
                }
                if !(*ptr).monitor_hwnd.is_null() {
                    DestroyWindow((*ptr).monitor_hwnd);
                    (*ptr).monitor_hwnd = null_mut();
                }
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            0
        }
        WM_COMMAND => 0,
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn app_state(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if ptr.is_null() { None } else { Some(&mut *ptr) }
}

impl AppState {
    fn new() -> Self {
        let (watchlist, alert_rules, automation_rules) = load_guard_settings();
        let installed_copy = is_installed_copy();
        let mut sentinel_config = load_sentinel_config();
        if provider_by_id(&sentinel_config.provider_id)
            .map(|provider| provider.is_api())
            .unwrap_or(true)
            && !installed_copy
        {
            sentinel_config = SentinelConfig::default();
        }
        let sentinel_selected_provider = provider_index(&sentinel_config.provider_id).unwrap_or(0);
        Self {
            processes: Vec::new(),
            groups: Vec::new(),
            visible_rows: Vec::new(),
            expanded_groups: BTreeSet::new(),
            selected: BTreeSet::new(),
            grouped_view: true,
            search: String::new(),
            search_focus: false,
            field_filter: None,
            sort_mode: SortMode::Memory,
            scroll: 0,
            hscroll: 0,
            status: "Ready".to_string(),
            details: "Select a group or process. Double-click a group to expand it.".to_string(),
            ai_text: String::new(),
            ai_context: String::new(),
            ai_input: String::new(),
            ai_cursor: 0,
            ai_input_all_selected: false,
            ai_input_focus: false,
            ai_messages: Vec::new(),
            ai_sessions: load_ai_sessions(),
            active_ai_session: None,
            ai_body_scroll: 0,
            ai_session_scroll: 0,
            ai_popup_pos: None,
            ai_popup_size: None,
            ai_hwnd: null_mut(),
            ai_dragging: false,
            ai_resizing: false,
            ai_drag_dx: 0,
            ai_drag_dy: 0,
            ai_resize_dx: 0,
            ai_resize_dy: 0,
            ai_running: false,
            ai_overlay: false,
            sentinel_model_input: sentinel_config.model.clone(),
            sentinel_config,
            sentinel_settings_open: false,
            sentinel_selected_provider,
            sentinel_key_input: String::new(),
            sentinel_settings_focus: SentinelSettingsFocus::Model,
            sentinel_text_all_selected: false,
            sentinel_key_stored: false,
            sentinel_test_running: false,
            sentinel_test_message: String::new(),
            sentinel_test_success: false,
            installed_copy,
            monitor_hwnd: null_mut(),
            monitor_title: String::new(),
            monitor_report: String::new(),
            monitor_scroll: 0,
            launcher_open: false,
            launcher_input: String::new(),
            launcher_focus: false,
            open_menu: None,
            context_menu: None,
            tray_visible: false,
            elevated: unsafe { is_process_elevated() },
            auto_refresh: true,
            performance_previous: HashMap::new(),
            performance_history: HashMap::new(),
            last_performance_sample: Instant::now(),
            watchlist,
            alert_rules,
            automation_rules,
            active_alerts: BTreeSet::new(),
            last_watch_counts: HashMap::new(),
            last_pids: BTreeSet::new(),
            hover: None,
            hover_x: 0,
            hover_y: 0,
        }
    }

    fn refresh(&mut self) {
        let previous = self.last_pids.clone();
        let selected = self.selected.clone();
        self.processes = enumerate_processes();
        let elapsed = self
            .last_performance_sample
            .elapsed()
            .as_secs_f64()
            .max(0.1);
        self.last_performance_sample = Instant::now();
        let logical_cpus = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1) as f64;
        let mut next_samples = HashMap::new();
        for process in &mut self.processes {
            if let Some((old_cpu, old_io)) = self.performance_previous.get(&process.pid) {
                let cpu_delta = process.cpu_total_100ns.saturating_sub(*old_cpu) as f64;
                process.cpu_percent = ((cpu_delta / 10_000_000.0) / elapsed / logical_cpus * 100.0)
                    .clamp(0.0, 100.0) as f32;
                let io_delta = process.io_total_bytes.saturating_sub(*old_io) as f64;
                process.io_rate_kbps = (io_delta / 1024.0 / elapsed).max(0.0) as f32;
            }
            next_samples.insert(
                process.pid,
                (process.cpu_total_100ns, process.io_total_bytes),
            );
            let history = self.performance_history.entry(process.pid).or_default();
            history.push_back(process.cpu_percent);
            while history.len() > 30 {
                history.pop_front();
            }
        }
        self.performance_previous = next_samples;
        self.performance_history
            .retain(|pid, _| self.processes.iter().any(|process| process.pid == *pid));
        self.groups = build_groups(&self.processes);
        self.last_pids = self.processes.iter().map(|p| p.pid).collect();
        let started = self.last_pids.difference(&previous).count();
        let stopped = previous.difference(&self.last_pids).count();
        self.apply_filters();
        self.selected = selected
            .into_iter()
            .filter(|id| self.visible_rows.iter().any(|r| self.row_id(*r) == *id))
            .collect();
        let base_status = format!(
            "{} visible rows | {} processes | {} groups | +{} -{} | filter: {}",
            self.visible_rows.len(),
            self.processes.len(),
            self.groups.len(),
            started,
            stopped,
            self.filter_label()
        );
        self.status = self.apply_monitoring_rules().unwrap_or(base_status);
        if !self.selected.is_empty() {
            self.details = self.build_selection_details();
        }
        if !self.monitor_hwnd.is_null() && self.monitor_title == "Live Performance" {
            self.monitor_report = self.build_live_performance_report();
            unsafe {
                InvalidateRect(self.monitor_hwnd, null(), 0);
            }
        }
    }

    fn sync_active_ai_session(&mut self) {
        if let Some(index) = self.active_ai_session {
            if let Some(session) = self.ai_sessions.get_mut(index) {
                session.context = self.ai_context.clone();
                session.messages = self.ai_messages.clone();
            }
        }
        save_ai_sessions(&self.ai_sessions);
    }

    fn load_ai_session(&mut self, index: usize) {
        if self.ai_running {
            self.status =
                "Sentinel is still answering. Wait before opening another saved chat.".to_string();
            return;
        }
        self.sync_active_ai_session();
        if let Some(session) = self.ai_sessions.get(index).cloned() {
            self.active_ai_session = Some(index);
            self.ai_context = session.context;
            self.ai_messages = session.messages;
            self.ai_input.clear();
            self.ai_cursor = 0;
            self.ai_input_all_selected = false;
            self.ai_overlay = true;
            self.ai_input_focus = true;
            self.ai_body_scroll = 0;
            self.status = format!("Opened Sentinel history: {}", self.active_ai_title());
        }
    }

    fn open_ai_history(&mut self) {
        self.ai_overlay = true;
        self.ai_input_focus = false;
        self.ai_dragging = false;
        if self.active_ai_session.is_none() {
            if let Some(index) = self.ordered_ai_session_indices().first().copied() {
                self.load_ai_session(index);
                return;
            }
        }
        self.status = if self.ai_sessions.is_empty() {
            "Sentinel history is empty. Select a process and click Ask Sentinel to create one."
                .to_string()
        } else {
            "Sentinel history opened. Select a saved chat on the left.".to_string()
        };
    }

    unsafe fn ensure_ai_window(&mut self, main_hwnd: HWND) {
        if !self.ai_overlay {
            return;
        }
        if !self.ai_hwnd.is_null() {
            ShowWindow(self.ai_hwnd, SW_SHOWNORMAL);
            SetForegroundWindow(self.ai_hwnd);
            InvalidateRect(self.ai_hwnd, null(), 0);
            return;
        }

        let instance = GetModuleHandleW(null()) as HINSTANCE;
        let class_name = wide(AI_WINDOW_CLASS);
        let title = wide("Sentinel AI Chat");
        let sw = GetSystemMetrics(SM_CXSCREEN).max(760);
        let sh = GetSystemMetrics(SM_CYSCREEN).max(520);
        let (w, h) = self.ai_popup_size.unwrap_or((980, 640));
        let w = w.clamp(620, (sw - 80).max(620));
        let h = h.clamp(420, (sh - 90).max(420));
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            ((sw - w) / 2).max(20),
            ((sh - h) / 2).max(20),
            w,
            h,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        if hwnd.is_null() {
            self.status = "Windows could not open the detached Sentinel chat.".to_string();
            return;
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, main_hwnd as isize);
        self.ai_hwnd = hwnd;
        self.ai_popup_pos = None;
        load_app_icon(hwnd);
        apply_dark_title_bar(hwnd);
        ShowWindow(hwnd, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd);
        UpdateWindow(hwnd);
    }

    unsafe fn close_ai_window(&mut self) {
        let hwnd = self.ai_hwnd;
        self.ai_hwnd = null_mut();
        self.ai_overlay = false;
        self.ai_input_focus = false;
        self.ai_dragging = false;
        self.ai_resizing = false;
        self.hover = None;
        if !hwnd.is_null() {
            DestroyWindow(hwnd);
        }
    }

    fn create_ai_session(&mut self, title: String, context: String, messages: Vec<AiMessage>) {
        self.sync_active_ai_session();
        self.ai_context = context.clone();
        self.ai_messages = messages.clone();
        self.ai_input.clear();
        self.ai_cursor = 0;
        self.ai_input_all_selected = false;
        self.ai_body_scroll = 0;
        self.ai_sessions
            .push(AiSession::new(title, context, messages));
        self.active_ai_session = Some(self.ai_sessions.len() - 1);
        save_ai_sessions(&self.ai_sessions);
    }

    fn ordered_ai_session_indices(&self) -> Vec<usize> {
        let mut pinned = Vec::new();
        let mut normal = Vec::new();
        for index in (0..self.ai_sessions.len()).rev() {
            if self.ai_sessions[index].pinned {
                pinned.push(index);
            } else {
                normal.push(index);
            }
        }
        pinned.extend(normal);
        pinned
    }

    fn active_ai_title(&self) -> String {
        self.active_ai_session
            .and_then(|index| self.ai_sessions.get(index))
            .map(|session| session.title.clone())
            .unwrap_or_else(|| "No chat selected".to_string())
    }

    fn toggle_pin_active_ai(&mut self) {
        let Some(index) = self.active_ai_session else {
            self.status = "Open a Sentinel chat first.".to_string();
            return;
        };
        if let Some(session) = self.ai_sessions.get_mut(index) {
            session.pinned = !session.pinned;
            self.status = if session.pinned {
                format!("Pinned Sentinel chat: {}", session.title)
            } else {
                format!("Unpinned Sentinel chat: {}", session.title)
            };
            save_ai_sessions(&self.ai_sessions);
        }
    }

    fn clear_active_ai_session(&mut self) {
        if self.ai_running {
            self.status = "Sentinel is still answering. Wait before deleting chats.".to_string();
            return;
        }
        let Some(index) = self.active_ai_session else {
            self.status = "No Sentinel chat selected.".to_string();
            return;
        };
        if self
            .ai_sessions
            .get(index)
            .map(|session| session.pinned)
            .unwrap_or(false)
        {
            self.status =
                "Pinned chats are protected. Unpin this chat before deleting it.".to_string();
            return;
        }
        if index < self.ai_sessions.len() {
            let title = self.ai_sessions[index].title.clone();
            self.ai_sessions.remove(index);
            self.active_ai_session = None;
            self.ai_context.clear();
            self.ai_messages.clear();
            self.ai_input.clear();
            self.ai_cursor = 0;
            self.ai_input_all_selected = false;
            self.ai_body_scroll = 0;
            if let Some(next) = self.ordered_ai_session_indices().first().copied() {
                self.load_ai_session(next);
            }
            self.status = format!("Deleted Sentinel chat: {}", title);
            save_ai_sessions(&self.ai_sessions);
        }
    }

    fn clear_unpinned_ai_sessions(&mut self) {
        if self.ai_running {
            self.status = "Sentinel is still answering. Wait before clearing history.".to_string();
            return;
        }
        let before = self.ai_sessions.len();
        self.ai_sessions.retain(|session| session.pinned);
        let removed = before.saturating_sub(self.ai_sessions.len());
        self.active_ai_session = None;
        self.ai_context.clear();
        self.ai_messages.clear();
        self.ai_input.clear();
        self.ai_cursor = 0;
        self.ai_input_all_selected = false;
        self.ai_body_scroll = 0;
        self.ai_session_scroll = 0;
        if let Some(next) = self.ordered_ai_session_indices().first().copied() {
            self.load_ai_session(next);
        }
        self.status = format!(
            "Cleared {} unpinned Sentinel chat(s). Pinned chats remain.",
            removed
        );
        save_ai_sessions(&self.ai_sessions);
    }

    fn apply_filters(&mut self) {
        self.visible_rows.clear();
        if !self.grouped_view {
            for index in self.sorted_process_indices() {
                if self.process_matches_filter(&self.processes[index]) {
                    self.visible_rows.push(RowKind::Process(index));
                }
            }
        } else {
            for group_index in self.sorted_group_indices() {
                let group = &self.groups[group_index];
                let group_match = self.group_matches_filter(group);
                let child_match = group
                    .process_indices
                    .iter()
                    .any(|i| self.process_matches_filter(&self.processes[*i]));
                if !group_match && !child_match {
                    continue;
                }
                self.visible_rows.push(RowKind::Group(group_index));
                if self.expanded_groups.contains(&group.key) {
                    for process_index in self.sorted_child_indices(group) {
                        if group_match
                            || self.process_matches_filter(&self.processes[process_index])
                        {
                            self.visible_rows.push(RowKind::Process(process_index));
                        }
                    }
                }
            }
        }
        self.scroll = self.scroll.min(self.visible_rows.len().saturating_sub(1));
    }

    fn paint(&self, hwnd: HWND) {
        unsafe {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut client: RECT = zeroed();
            GetClientRect(hwnd, &mut client);
            let width = (client.right - client.left).max(1);
            let height = (client.bottom - client.top).max(1);
            let layout = self.layout(client);

            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap = if mem_dc.is_null() {
                null_mut()
            } else {
                CreateCompatibleBitmap(hdc, width, height)
            };
            let buffered = !mem_dc.is_null() && !mem_bitmap.is_null();
            let old_bitmap = if buffered {
                SelectObject(mem_dc, mem_bitmap as HGDIOBJ)
            } else {
                null_mut()
            };
            let canvas = if buffered { mem_dc } else { hdc };

            let font_body = create_font(15, 400);
            let font_bold = create_font(15, FW_BOLD as i32);
            let font_title = create_font(20, FW_BOLD as i32);
            let old_font = SelectObject(canvas, font_body as HGDIOBJ);
            SetBkMode(canvas, TRANSPARENT as i32);

            fill(canvas, client, C_BG);
            self.draw_top(canvas, &layout, font_title, font_bold);
            self.draw_table(canvas, &layout, font_body, font_bold);
            self.draw_details(canvas, &layout, font_body, font_bold);
            self.draw_status(canvas, &layout, font_body);
            self.draw_menu_dropdown(canvas, &layout, font_body, font_bold);
            self.draw_context_menu(canvas, client, font_body, font_bold);
            if self.launcher_open {
                self.draw_launcher(canvas, client, font_body, font_bold);
            }
            if self.ai_overlay && self.ai_hwnd.is_null() {
                self.draw_ai_overlay(canvas, client, font_body, font_bold);
            }
            if self.sentinel_settings_open {
                self.draw_sentinel_settings(canvas, client, font_body, font_bold);
            }
            self.draw_hover_tip(canvas, client, font_body);

            if buffered {
                BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);
            }

            SelectObject(canvas, old_font);
            DeleteObject(font_body as HGDIOBJ);
            DeleteObject(font_bold as HGDIOBJ);
            DeleteObject(font_title as HGDIOBJ);
            if buffered {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(mem_bitmap as HGDIOBJ);
            }
            if !mem_dc.is_null() {
                DeleteDC(mem_dc);
            }
            EndPaint(hwnd, &ps);
        }
    }

    fn paint_ai_window(&self, hwnd: HWND) {
        unsafe {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut client: RECT = zeroed();
            GetClientRect(hwnd, &mut client);
            let width = (client.right - client.left).max(1);
            let height = (client.bottom - client.top).max(1);

            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap = if mem_dc.is_null() {
                null_mut()
            } else {
                CreateCompatibleBitmap(hdc, width, height)
            };
            let buffered = !mem_dc.is_null() && !mem_bitmap.is_null();
            let old_bitmap = if buffered {
                SelectObject(mem_dc, mem_bitmap as HGDIOBJ)
            } else {
                null_mut()
            };
            let canvas = if buffered { mem_dc } else { hdc };

            let font_body = create_font(15, 400);
            let font_bold = create_font(15, FW_BOLD as i32);
            let old_font = SelectObject(canvas, font_body as HGDIOBJ);
            SetBkMode(canvas, TRANSPARENT as i32);

            fill(canvas, client, C_BG);
            self.draw_ai_overlay(canvas, client, font_body, font_bold);
            self.draw_hover_tip(canvas, client, font_body);

            if buffered {
                BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);
            }

            SelectObject(canvas, old_font);
            DeleteObject(font_body as HGDIOBJ);
            DeleteObject(font_bold as HGDIOBJ);
            if buffered {
                SelectObject(mem_dc, old_bitmap);
                DeleteObject(mem_bitmap as HGDIOBJ);
            }
            if !mem_dc.is_null() {
                DeleteDC(mem_dc);
            }
            EndPaint(hwnd, &ps);
        }
    }

    fn monitor_lines(&self, client: RECT) -> Vec<String> {
        let max_chars = (((client.right - client.left - 48) / 8).max(30)) as usize;
        wrapped_lines(&self.monitor_report, max_chars)
    }

    fn monitor_max_scroll(&self, client: RECT) -> usize {
        let visible = (((client.bottom - client.top - 92) / 18).max(1)) as usize;
        self.monitor_lines(client).len().saturating_sub(visible)
    }

    fn paint_monitor_window(&self, hwnd: HWND) {
        unsafe {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut client: RECT = zeroed();
            GetClientRect(hwnd, &mut client);
            let width = (client.right - client.left).max(1);
            let height = (client.bottom - client.top).max(1);
            let mem_dc = CreateCompatibleDC(hdc);
            let bitmap = CreateCompatibleBitmap(hdc, width, height);
            let old_bitmap = SelectObject(mem_dc, bitmap as HGDIOBJ);
            let font = create_font(15, 400);
            let bold = create_font(16, FW_BOLD as i32);
            let old_font = SelectObject(mem_dc, font as HGDIOBJ);
            SetBkMode(mem_dc, TRANSPARENT as i32);

            fill(mem_dc, client, C_BG);
            fill(mem_dc, rect(0, 0, width, 52), C_PANEL);
            line_bottom(mem_dc, rect(0, 0, width, 52), C_CYAN);
            select(mem_dc, bold, C_CYAN);
            draw_text(
                mem_dc,
                "MONITOR CENTER",
                rect(18, 10, 220, 22),
                DT_LEFT | DT_SINGLELINE,
            );
            select(mem_dc, font, C_TEXT);
            draw_text(
                mem_dc,
                &self.monitor_title,
                rect(190, 11, width - 210, 22),
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );

            let body = rect(14, 62, width - 28, height - 88);
            fill(mem_dc, body, C_TABLE);
            frame(mem_dc, body, C_BORDER);
            let lines = self.monitor_lines(client);
            let visible = (((body.bottom - body.top - 16) / 18).max(1)) as usize;
            let scroll = self.monitor_scroll.min(lines.len().saturating_sub(visible));
            for (row, line) in lines.iter().skip(scroll).take(visible).enumerate() {
                let trimmed = line.trim_start();
                let color = if trimmed.starts_with('+') {
                    C_SAFE_TEXT
                } else if trimmed.starts_with('-') {
                    C_BLOCK_TEXT
                } else if trimmed.starts_with('*') {
                    C_WARN_TEXT
                } else if !trimmed.is_empty()
                    && trimmed
                        .chars()
                        .all(|ch| !ch.is_ascii_alphabetic() || ch.is_ascii_uppercase())
                {
                    C_CYAN
                } else {
                    C_TEXT
                };
                select(mem_dc, if color == C_CYAN { bold } else { font }, color);
                draw_text(
                    mem_dc,
                    line,
                    rect(
                        body.left + 12,
                        body.top + 8 + row as i32 * 18,
                        body.right - body.left - 28,
                        18,
                    ),
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
            }
            if lines.len() > visible {
                let track = rect(body.right - 7, body.top + 4, 4, body.bottom - body.top - 8);
                fill(mem_dc, track, C_GRID);
                let thumb_h = ((track.bottom - track.top) as f32
                    * (visible as f32 / lines.len() as f32))
                    .max(24.0) as i32;
                let max_scroll = lines.len().saturating_sub(visible).max(1);
                let y = track.top
                    + (((track.bottom - track.top - thumb_h) as f32)
                        * (scroll as f32 / max_scroll as f32)) as i32;
                fill(mem_dc, rect(track.left - 1, y, 6, thumb_h), C_CYAN);
            }
            select(mem_dc, font, C_MUTED);
            draw_text(
                mem_dc,
                "Mouse wheel or arrow keys scroll. Esc closes Monitor Center.",
                rect(16, height - 22, width - 32, 18),
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );

            BitBlt(hdc, 0, 0, width, height, mem_dc, 0, 0, SRCCOPY);
            SelectObject(mem_dc, old_font);
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(font as HGDIOBJ);
            DeleteObject(bold as HGDIOBJ);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(mem_dc);
            EndPaint(hwnd, &ps);
        }
    }

    fn layout(&self, client: RECT) -> Layout {
        let w = client.right - client.left;
        let h = client.bottom - client.top;
        let details_w = DETAILS_W.min((w / 3).max(280));
        let details = rect(
            w - details_w - 14,
            TOP_H + 10,
            details_w,
            h - TOP_H - STATUS_H - 28,
        );
        let table = rect(
            14,
            TOP_H + 10,
            w - details_w - 42,
            h - TOP_H - STATUS_H - 28,
        );
        let status = rect(14, h - STATUS_H - 8, w - 28, STATUS_H);
        let search = rect(14, 94, 260, 28);
        let search_clear = rect(search.right - 28, search.top + 4, 20, 20);

        let menu_specs = [
            (MenuKind::File, 54),
            (MenuKind::View, 58),
            (MenuKind::Search, 76),
            (MenuKind::Monitor, 82),
            (MenuKind::Control, 82),
            (MenuKind::Tools, 62),
            (MenuKind::Help, 58),
        ];
        let mut menus = Vec::new();
        let mut mx = 8;
        for (kind, width) in menu_specs {
            menus.push((kind, rect(mx, 3, width, 23)));
            mx += width + 2;
        }

        let mut buttons = Vec::new();
        let specs = [
            (Action::Refresh, "Refresh", 90),
            (Action::EndSafe, "End Safe", 92),
            (Action::Explain, "Ask Sentinel", 118),
            (Action::History, "History", 84),
            (
                Action::ToggleView,
                if self.grouped_view {
                    "Flat View"
                } else {
                    "Grouped"
                },
                98,
            ),
            (Action::SelectSafe, "Select Safe", 104),
            (Action::Export, "Export", 82),
            (
                Action::AutoRefresh,
                if self.auto_refresh {
                    "Auto On"
                } else {
                    "Auto Off"
                },
                88,
            ),
            (Action::Admin, "Admin", 104),
        ];
        let mut x = 14;
        for (action, _, width) in specs {
            buttons.push((action, rect(x, 58, width, 28)));
            x += width + 8;
        }

        let widths = [
            (Column::Name, FROZEN_NAME_W),
            (Column::Items, 72),
            (Column::Type, 150),
            (Column::Cpu, 76),
            (Column::Memory, 110),
            (Column::Io, 120),
            (Column::Network, 80),
            (Column::Threads, 84),
            (Column::Handles, 84),
            (Column::Risk, 72),
            (Column::Safety, 100),
            (Column::Reason, 480),
        ];
        let mut logical_x = table.left;
        let mut columns = Vec::new();
        for (index, (column, width)) in widths.into_iter().enumerate() {
            let left = if index == 0 {
                table.left
            } else {
                logical_x - self.hscroll
            };
            columns.push((column, rect(left, table.top, width, HEADER_H)));
            logical_x += width;
        }

        Layout {
            menus,
            search,
            search_clear,
            table,
            details,
            status,
            buttons,
            columns,
        }
    }

    fn draw_top(&self, hdc: HDC, layout: &Layout, title_font: isize, bold_font: isize) {
        unsafe {
            fill(hdc, rect(0, 0, 10000, TOP_H + 2), C_PANEL);
            fill(hdc, rect(0, 0, 10000, 30), C_MENU_BG);
            for (kind, r) in &layout.menus {
                let active = self.open_menu == Some(*kind);
                fill(hdc, *r, if active { C_MENU_ACTIVE } else { C_MENU_BG });
                frame(hdc, *r, if active { C_CYAN } else { C_MENU_BG });
                select(hdc, bold_font, if active { C_CYAN } else { C_TEXT });
                draw_text(
                    hdc,
                    kind.label(),
                    inset(*r, 8, 0),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );
            }
            select(hdc, title_font, C_TEXT);
            draw_text(
                hdc,
                "PROCESS GUARD",
                rect(14, 32, 260, 24),
                DT_LEFT | DT_SINGLELINE,
            );
            select(hdc, bold_font, C_MUTED);
            draw_text(
                hdc,
                "native process control / grouped safety view",
                rect(190, 35, 420, 20),
                DT_LEFT | DT_SINGLELINE,
            );

            for (action, r) in &layout.buttons {
                let label = match action {
                    Action::Refresh => "Refresh",
                    Action::EndSafe => "End Safe",
                    Action::Explain => {
                        if self.ai_running {
                            "Sentinel..."
                        } else {
                            "Ask Sentinel"
                        }
                    }
                    Action::History => "History",
                    Action::ToggleView => {
                        if self.grouped_view {
                            "Flat View"
                        } else {
                            "Grouped"
                        }
                    }
                    Action::SelectSafe => "Select Safe",
                    Action::Export => "Export",
                    Action::AutoRefresh => {
                        if self.auto_refresh {
                            "Auto On"
                        } else {
                            "Auto Off"
                        }
                    }
                    Action::Admin => {
                        if self.elevated {
                            "Admin On"
                        } else {
                            "Admin Off"
                        }
                    }
                };
                let active = (*action == Action::Explain && self.ai_running)
                    || (*action == Action::AutoRefresh && self.auto_refresh)
                    || (*action == Action::Admin && self.elevated);
                draw_button(hdc, *r, label, active);
            }

            draw_search(
                hdc,
                layout.search,
                layout.search_clear,
                &self.search,
                self.search_focus,
            );
            let filter = format!(
                "Header: Name/RAM/Risk sort, Type/Safety/Can End filter | {}",
                self.filter_label()
            );
            select(hdc, bold_font, C_MUTED);
            draw_text(
                hdc,
                &filter,
                rect(layout.search.right + 12, 96, 800, 24),
                DT_LEFT | DT_SINGLELINE,
            );

            let cpu: f32 = self
                .processes
                .iter()
                .map(|process| process.cpu_percent)
                .sum();
            let ram: u64 = self.processes.iter().map(|process| process.memory_kb).sum();
            let disk: f32 = self
                .processes
                .iter()
                .map(|process| process.io_rate_kbps)
                .sum();
            let network: u32 = self
                .processes
                .iter()
                .map(|process| process.network_connections)
                .sum();
            let threads: u32 = self
                .processes
                .iter()
                .map(|process| process.thread_count)
                .sum();
            let handles: u32 = self
                .processes
                .iter()
                .map(|process| process.handle_count)
                .sum();
            let live = rect(14, 128, layout.status.right - 28, 28);
            fill(hdc, live, C_TABLE);
            frame(hdc, live, C_BORDER);
            select(hdc, bold_font, C_GREEN);
            draw_text(
                hdc,
                "LIVE",
                rect(live.left + 9, live.top, 44, live.bottom - live.top),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            select(hdc, bold_font, C_TEXT);
            draw_text(
                hdc,
                &format!(
                    "CPU {:.1}%   RAM {}   DISK {}   NET {}   THREADS {}   HANDLES {}",
                    cpu.min(100.0),
                    format_memory(ram),
                    format_rate(disk),
                    network,
                    threads,
                    handles
                ),
                rect(
                    live.left + 58,
                    live.top,
                    live.right - live.left - 350,
                    live.bottom - live.top,
                ),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            let legend_left = (live.right - 292).max(live.left + 420);
            fill(hdc, rect(legend_left, live.top + 6, 12, 16), C_SAFE_BG_A);
            fill(
                hdc,
                rect(legend_left + 82, live.top + 6, 12, 16),
                C_CAUTION_BG,
            );
            fill(
                hdc,
                rect(legend_left + 190, live.top + 6, 12, 16),
                C_DANGER_BG,
            );
            select(hdc, bold_font, C_MUTED);
            draw_text(
                hdc,
                "Safe",
                rect(legend_left + 17, live.top, 58, 28),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            draw_text(
                hdc,
                "Caution",
                rect(legend_left + 99, live.top, 78, 28),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            draw_text(
                hdc,
                "High risk",
                rect(legend_left + 207, live.top, 78, 28),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    fn draw_table(&self, hdc: HDC, layout: &Layout, body_font: isize, bold_font: isize) {
        unsafe {
            fill(hdc, layout.table, C_TABLE);
            frame(hdc, layout.table, C_BORDER);
            let frozen_right = (layout.table.left + FROZEN_NAME_W).min(layout.table.right);
            let rows_top = layout.table.top + HEADER_H;
            let rows_bottom = layout.table.bottom - HSCROLL_H;
            let visible_count = ((rows_bottom - rows_top) / ROW_H).max(0) as usize;

            let scroll_view = RECT {
                left: frozen_right,
                top: layout.table.top,
                right: layout.table.right,
                bottom: rows_bottom,
            };
            for (col, r) in layout.columns.iter().skip(1) {
                let Some(visible) = intersect_rect(*r, scroll_view) else {
                    continue;
                };
                fill(hdc, visible, C_HEADER);
                frame(hdc, visible, C_BORDER);
                if r.left < frozen_right {
                    continue;
                }
                select(hdc, bold_font, C_CYAN);
                draw_text(
                    hdc,
                    col.label(),
                    inset(visible, 8, 0),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
            for screen_row in 0..visible_count {
                let row_index = self.scroll + screen_row;
                if row_index >= self.visible_rows.len() {
                    break;
                }
                let row_rect = rect(
                    layout.table.left,
                    rows_top + (screen_row as i32 * ROW_H),
                    layout.table.right - layout.table.left,
                    ROW_H,
                );
                let row = self.visible_rows[row_index];
                let id = self.row_id(row);
                let selected = self.selected.contains(&id);
                let colors = self.row_colors(row, selected, screen_row);
                fill(hdc, row_rect, colors.1);
                if selected {
                    frame(hdc, row_rect, C_CYAN);
                } else {
                    line_bottom(hdc, row_rect, C_GRID);
                }

                let cells = self.row_cells(row);
                for (idx, (_, col_rect)) in layout.columns.iter().enumerate().skip(1) {
                    let cell_rect = rect(
                        col_rect.left,
                        row_rect.top,
                        col_rect.right - col_rect.left,
                        ROW_H,
                    );
                    let Some(visible) = intersect_rect(cell_rect, scroll_view) else {
                        continue;
                    };
                    if col_rect.left >= frozen_right {
                        line_left(hdc, visible, C_GRID);
                    } else {
                        continue;
                    }
                    let text = cells.get(idx).map(|s| s.as_str()).unwrap_or("");
                    select(
                        hdc,
                        if matches!(row, RowKind::Group(_)) {
                            bold_font
                        } else {
                            body_font
                        },
                        colors.0,
                    );
                    draw_text(
                        hdc,
                        text,
                        inset(visible, 8, 0),
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                    );
                }
            }
            if self.visible_rows.len() > visible_count && visible_count > 0 {
                let track = rect(layout.table.right - 5, rows_top, 3, rows_bottom - rows_top);
                fill(hdc, track, C_GRID);
                let ratio = visible_count as f32 / self.visible_rows.len() as f32;
                let thumb_h = ((rows_bottom - rows_top) as f32 * ratio).max(28.0) as i32;
                let max_scroll = self.visible_rows.len().saturating_sub(visible_count).max(1);
                let y = rows_top
                    + (((rows_bottom - rows_top - thumb_h) as f32)
                        * (self.scroll.min(max_scroll) as f32 / max_scroll as f32))
                        as i32;
                fill(hdc, rect(layout.table.right - 6, y, 5, thumb_h), C_CYAN);
            }
            let name_header = rect(
                layout.table.left,
                layout.table.top,
                frozen_right - layout.table.left,
                HEADER_H,
            );
            fill(hdc, name_header, C_HEADER);
            frame(hdc, name_header, C_CYAN);
            select(hdc, bold_font, C_CYAN);
            draw_text(
                hdc,
                Column::Name.label(),
                inset(name_header, 8, 0),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            for screen_row in 0..visible_count {
                let row_index = self.scroll + screen_row;
                if row_index >= self.visible_rows.len() {
                    break;
                }
                let row = self.visible_rows[row_index];
                let selected = self.selected.contains(&self.row_id(row));
                let colors = self.row_colors(row, selected, screen_row);
                let cell = rect(
                    layout.table.left,
                    rows_top + screen_row as i32 * ROW_H,
                    frozen_right - layout.table.left,
                    ROW_H,
                );
                fill(hdc, cell, colors.1);
                line_bottom(hdc, cell, if selected { C_CYAN } else { C_GRID });
                frame(hdc, rect(cell.right - 1, cell.top, 1, ROW_H), C_CYAN);
                select(
                    hdc,
                    if matches!(row, RowKind::Group(_)) {
                        bold_font
                    } else {
                        body_font
                    },
                    colors.0,
                );
                let name = self.row_cells(row).into_iter().next().unwrap_or_default();
                draw_text(
                    hdc,
                    &name,
                    inset(cell, 8, 0),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
            let bar = rect(
                layout.table.left,
                rows_bottom,
                layout.table.right - layout.table.left,
                HSCROLL_H,
            );
            fill(hdc, bar, C_PANEL);
            line_bottom(hdc, bar, C_BORDER);
            select(hdc, body_font, C_MUTED);
            draw_text(
                hdc,
                "Shift+wheel or Left/Right",
                rect(bar.left + 8, bar.top, FROZEN_NAME_W - 16, HSCROLL_H),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
            let track = rect(
                frozen_right + 8,
                bar.top + 5,
                (layout.table.right - frozen_right - 16).max(20),
                5,
            );
            fill(hdc, track, C_GRID);
            let max_hscroll = self.max_hscroll_for_table(layout.table);
            if max_hscroll > 0 {
                let visible_width = (layout.table.right - frozen_right).max(1);
                let scrollable_width = (table_content_width() - FROZEN_NAME_W).max(1);
                let thumb_w = ((track.right - track.left) as f32
                    * (visible_width as f32 / scrollable_width as f32))
                    .max(32.0) as i32;
                let thumb_x = track.left
                    + (((track.right - track.left - thumb_w) as f32)
                        * (self.hscroll as f32 / max_hscroll as f32)) as i32;
                fill(hdc, rect(thumb_x, bar.top + 3, thumb_w, 9), C_CYAN);
            }
        }
    }

    fn draw_details(&self, hdc: HDC, layout: &Layout, body_font: isize, bold_font: isize) {
        unsafe {
            fill(hdc, layout.details, C_SIDE);
            frame(hdc, layout.details, C_BORDER);
            select(hdc, bold_font, C_CYAN);
            draw_text(
                hdc,
                "INSPECTOR",
                inset(layout.details, 12, 8),
                DT_LEFT | DT_SINGLELINE,
            );
            select(hdc, body_font, C_TEXT);
            draw_multiline(
                hdc,
                &self.details,
                rect(
                    layout.details.left + 12,
                    layout.details.top + 42,
                    layout.details.right - layout.details.left - 24,
                    layout.details.bottom - layout.details.top - 54,
                ),
                18,
            );
        }
    }

    fn draw_status(&self, hdc: HDC, layout: &Layout, body_font: isize) {
        unsafe {
            fill(hdc, layout.status, C_PANEL);
            select(hdc, body_font, C_GREEN);
            draw_text(
                hdc,
                &self.status,
                inset(layout.status, 10, 0),
                DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
            );
        }
    }

    fn draw_menu_dropdown(&self, hdc: HDC, layout: &Layout, body_font: isize, bold_font: isize) {
        let Some(kind) = self.open_menu else {
            return;
        };
        unsafe {
            let items = self.menu_dropdown_rects(layout, kind);
            if items.is_empty() {
                return;
            }
            let bounds = rect(
                items[0].1.left,
                items[0].1.top,
                items[0].1.right - items[0].1.left,
                (items.len() as i32 * MENU_ITEM_H) + 2,
            );
            fill(hdc, bounds, C_MENU_BG);
            frame(hdc, bounds, C_CYAN);
            for (index, (_, r, label)) in items.iter().enumerate() {
                fill(hdc, *r, if index % 2 == 0 { C_TABLE } else { C_ROW_B });
                fill(
                    hdc,
                    rect(r.left, r.top, 3, r.bottom - r.top),
                    menu_command_accent(items[index].0),
                );
                line_bottom(hdc, *r, C_GRID);
                if index > 0 {
                    fill(
                        hdc,
                        rect(r.left + 3, r.top, r.right - r.left - 3, 1),
                        C_BORDER,
                    );
                }
                select(hdc, bold_font, C_TEXT);
                draw_text(
                    hdc,
                    label,
                    rect(r.left + 12, r.top + 4, r.right - r.left - 20, 18),
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
                select(hdc, body_font, C_MUTED);
                draw_text(
                    hdc,
                    menu_command_description(items[index].0),
                    rect(r.left + 12, r.top + 23, r.right - r.left - 20, 16),
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
            }
        }
    }

    fn draw_context_menu(&self, hdc: HDC, client: RECT, body_font: isize, bold_font: isize) {
        let Some(menu) = self.context_menu else {
            return;
        };
        unsafe {
            let items = self.context_menu_rects(client, menu);
            if items.is_empty() {
                return;
            }
            let bounds = rect(
                items[0].1.left,
                items[0].1.top,
                items[0].1.right - items[0].1.left,
                (items.len() as i32 * 28) + 2,
            );
            fill(hdc, bounds, C_MENU_BG);
            frame(hdc, bounds, C_CYAN);
            for (index, (_, r, label)) in items.iter().enumerate() {
                fill(hdc, *r, if index % 2 == 0 { C_TABLE } else { C_ROW_B });
                line_bottom(hdc, *r, C_GRID);
                select(hdc, if index == 0 { bold_font } else { body_font }, C_TEXT);
                draw_text(
                    hdc,
                    label,
                    inset(*r, 10, 0),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }
        }
    }

    fn draw_launcher(&self, hdc: HDC, client: RECT, body_font: isize, bold_font: isize) {
        unsafe {
            let launcher = launcher_layout(client);
            fill(hdc, launcher.popup, C_MODAL);
            frame(hdc, launcher.popup, C_CYAN);
            select(hdc, bold_font, C_CYAN);
            draw_text(
                hdc,
                "START PROCESS",
                inset(launcher.popup, 16, 14),
                DT_LEFT | DT_SINGLELINE,
            );
            draw_button(hdc, launcher.close, "X", false);
            select(hdc, body_font, C_MUTED);
            draw_text(
                hdc,
                "Type an app name, executable path, or command. Quote paths that contain spaces.",
                rect(
                    launcher.popup.left + 16,
                    launcher.popup.top + 48,
                    launcher.popup.right - launcher.popup.left - 32,
                    22,
                ),
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            draw_text_box(
                hdc,
                launcher.input,
                &self.launcher_input,
                "notepad.exe  |  powershell.exe  |  \"C:\\path\\app.exe\" args",
                self.launcher_focus,
            );
            draw_button(hdc, launcher.start, "Start", false);
            draw_button(hdc, launcher.cancel, "Cancel", false);
        }
    }

    fn draw_sentinel_settings(&self, hdc: HDC, client: RECT, body_font: isize, bold_font: isize) {
        unsafe {
            let ui = sentinel_settings_layout(client);
            let provider = &SENTINEL_PROVIDERS[self.sentinel_selected_provider];
            fill(hdc, ui.popup, C_MODAL);
            frame(hdc, ui.popup, C_CYAN);
            fill(
                hdc,
                rect(
                    ui.popup.left,
                    ui.popup.top,
                    ui.popup.right - ui.popup.left,
                    48,
                ),
                C_HEADER,
            );
            line_bottom(
                hdc,
                rect(
                    ui.popup.left,
                    ui.popup.top,
                    ui.popup.right - ui.popup.left,
                    48,
                ),
                C_CYAN,
            );
            select(hdc, bold_font, C_CYAN);
            draw_text(
                hdc,
                "SENTINEL AI SETTINGS",
                rect(ui.popup.left + 16, ui.popup.top + 13, 300, 22),
                DT_LEFT | DT_SINGLELINE,
            );
            select(hdc, body_font, C_MUTED);
            draw_text(
                hdc,
                "Choose an engine and model. Process Guard tests it before saving.",
                rect(
                    ui.popup.left + 280,
                    ui.popup.top + 14,
                    ui.popup.right - ui.popup.left - 350,
                    20,
                ),
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            draw_button(hdc, ui.close, "X", false);

            fill(hdc, ui.providers, C_SIDE);
            frame(hdc, ui.providers, C_BORDER);
            select(hdc, bold_font, C_GREEN);
            draw_text(
                hdc,
                "ENGINES",
                rect(
                    ui.providers.left + 10,
                    ui.providers.top + 9,
                    ui.providers.right - ui.providers.left - 20,
                    20,
                ),
                DT_LEFT | DT_SINGLELINE,
            );
            for (index, item) in SENTINEL_PROVIDERS.iter().enumerate() {
                let row = sentinel_provider_row(&ui, index);
                let disabled = item.is_api() && !self.installed_copy;
                let selected = index == self.sentinel_selected_provider;
                if selected {
                    fill(hdc, row, C_SELECT_BG);
                    line_left(hdc, row, C_CYAN);
                } else if index % 2 == 1 {
                    fill(hdc, row, C_ROW_B);
                }
                select(
                    hdc,
                    if selected { bold_font } else { body_font },
                    if disabled {
                        C_MUTED
                    } else if selected {
                        C_SELECT_TEXT
                    } else {
                        C_TEXT
                    },
                );
                draw_text(
                    hdc,
                    item.label,
                    rect(
                        row.left + 10,
                        row.top,
                        row.right - row.left - 62,
                        row.bottom - row.top,
                    ),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
                select(
                    hdc,
                    body_font,
                    if disabled {
                        C_MUTED
                    } else if item.is_api() {
                        C_WARN_TEXT
                    } else {
                        C_GREEN
                    },
                );
                draw_text(
                    hdc,
                    item.mode_label(),
                    rect(row.right - 48, row.top, 38, row.bottom - row.top),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                );
                line_bottom(hdc, row, C_GRID);
            }

            let content_left = ui.providers.right + 20;
            let content_width = ui.popup.right - content_left - 18;
            select(hdc, bold_font, C_CYAN);
            draw_text(
                hdc,
                provider.label,
                rect(content_left, ui.popup.top + 64, content_width, 24),
                DT_LEFT | DT_SINGLELINE,
            );
            select(hdc, body_font, C_TEXT);
            draw_multiline(
                hdc,
                provider.description,
                rect(content_left, ui.popup.top + 94, content_width, 54),
                18,
            );

            select(hdc, bold_font, C_GREEN);
            draw_text(
                hdc,
                "MODEL ID",
                rect(content_left, ui.model_input.top - 25, content_width, 20),
                DT_LEFT | DT_SINGLELINE,
            );
            draw_text_box(
                hdc,
                ui.model_input,
                &self.sentinel_model_input,
                "enter provider model ID",
                self.sentinel_settings_focus == SentinelSettingsFocus::Model,
            );
            select(hdc, body_font, C_MUTED);
            draw_text(
                hdc,
                "Suggested models (the field remains editable for future model IDs):",
                rect(
                    content_left,
                    ui.model_choices[0].top - 22,
                    content_width,
                    18,
                ),
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            for (index, button) in ui.model_choices.iter().enumerate() {
                let label = provider.models.get(index).copied().unwrap_or("");
                if !label.is_empty() {
                    draw_sentinel_model_choice(
                        hdc,
                        *button,
                        label,
                        sentinel_model_tier(provider, index),
                        self.sentinel_model_input == label,
                        body_font,
                        bold_font,
                    );
                }
            }
            let selected_tier = selected_sentinel_model_tier(provider, &self.sentinel_model_input);
            select(hdc, body_font, selected_tier.color());
            draw_multiline(
                hdc,
                &format!(
                    "{} | {}",
                    selected_tier.label(),
                    sentinel_model_note(selected_tier)
                ),
                rect(
                    content_left,
                    ui.model_choices[0].bottom + 7,
                    content_width,
                    40,
                ),
                17,
            );

            let key_label_top = ui.key_input.top - 25;
            select(
                hdc,
                bold_font,
                if provider.is_api() {
                    C_WARN_TEXT
                } else {
                    C_GREEN
                },
            );
            draw_text(
                hdc,
                if provider.is_api() {
                    "API KEY"
                } else {
                    "AUTHENTICATION"
                },
                rect(content_left, key_label_top, content_width, 20),
                DT_LEFT | DT_SINGLELINE,
            );
            if provider.is_api() && self.installed_copy {
                let masked = if self.sentinel_key_input.is_empty() {
                    String::new()
                } else {
                    "*".repeat(self.sentinel_key_input.chars().count().clamp(8, 52))
                };
                draw_text_box(
                    hdc,
                    ui.key_input,
                    &masked,
                    if self.sentinel_key_stored {
                        "A key is stored securely. Paste here only to replace it."
                    } else {
                        "paste your API key (stored in Windows Credential Manager)"
                    },
                    self.sentinel_settings_focus == SentinelSettingsFocus::ApiKey,
                );
                draw_button(hdc, ui.clear_key, "Clear Stored Key", false);
                select(
                    hdc,
                    body_font,
                    if self.sentinel_key_stored {
                        C_SAFE_TEXT
                    } else {
                        C_MUTED
                    },
                );
                draw_text(
                    hdc,
                    if self.sentinel_key_stored {
                        "Stored for this Windows account. The key is never written to app settings, source, ZIPs, or GitHub."
                    } else {
                        "No key is currently stored for this provider."
                    },
                    rect(content_left, ui.key_input.bottom + 8, content_width, 36),
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
            } else if provider.is_api() {
                fill(hdc, ui.key_input, C_DANGER_BG);
                frame(hdc, ui.key_input, C_BLOCK_TEXT);
                select(hdc, body_font, C_BLOCK_TEXT);
                draw_text(
                    hdc,
                    "API providers are disabled in the portable EXE. Install Process Guard with Setup first.",
                    inset(ui.key_input, 8, 0),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            } else {
                fill(hdc, ui.key_input, C_SAFE_BG_A);
                frame(hdc, ui.key_input, C_GREEN);
                select(hdc, body_font, C_SAFE_TEXT);
                draw_text(
                    hdc,
                    "Uses the CLI's existing local sign-in. Process Guard does not request or store a key.",
                    inset(ui.key_input, 8, 0),
                    DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                );
            }

            if !self.sentinel_test_message.is_empty() {
                select(
                    hdc,
                    bold_font,
                    if self.sentinel_test_success {
                        C_SAFE_TEXT
                    } else {
                        C_BLOCK_TEXT
                    },
                );
                draw_multiline(
                    hdc,
                    &self.sentinel_test_message,
                    rect(content_left, ui.popup.bottom - 168, content_width, 38),
                    17,
                );
            }
            select(hdc, body_font, C_WARN_TEXT);
            draw_multiline(
                hdc,
                "Privacy: testing sends a tiny prompt to the selected engine. API mode may use quota or incur a small provider charge. Normal chats send selected process context.",
                rect(content_left, ui.popup.bottom - 118, content_width, 40),
                18,
            );
            draw_button(
                hdc,
                ui.save,
                if self.sentinel_test_running {
                    "Testing model..."
                } else {
                    "Test + Save + Restart"
                },
                self.sentinel_test_running,
            );
            draw_button(hdc, ui.cancel, "Cancel", false);
        }
    }

    fn menu_items(&self, kind: MenuKind) -> Vec<(MenuCommand, String)> {
        match kind {
            MenuKind::File => vec![
                (
                    MenuCommand::Action(Action::Refresh),
                    "Refresh Now".to_string(),
                ),
                (
                    MenuCommand::Action(Action::Export),
                    "Export Report".to_string(),
                ),
                (MenuCommand::MinimizeTray, "Minimize To Tray".to_string()),
                (MenuCommand::Exit, "Exit".to_string()),
            ],
            MenuKind::View => vec![
                (
                    MenuCommand::Action(Action::ToggleView),
                    if self.grouped_view {
                        "Switch To Flat View"
                    } else {
                        "Switch To Grouped View"
                    }
                    .to_string(),
                ),
                (
                    MenuCommand::ExpandSelected,
                    "Expand Selected Groups".to_string(),
                ),
                (MenuCommand::CollapseAll, "Collapse All Groups".to_string()),
                (
                    MenuCommand::Action(Action::SelectSafe),
                    "Select All Safe".to_string(),
                ),
                (MenuCommand::ClearSelection, "Clear Selection".to_string()),
            ],
            MenuKind::Search => vec![
                (MenuCommand::FocusSearch, "Focus Search Box".to_string()),
                (
                    MenuCommand::ClearFilters,
                    "Clear Search And Filters".to_string(),
                ),
                (MenuCommand::StartProcess, "Start Process...".to_string()),
            ],
            MenuKind::Monitor => vec![
                (
                    MenuCommand::LivePerformance,
                    "Live Performance + Graphs".to_string(),
                ),
                (MenuCommand::ProcessTree, "Process Family Tree".to_string()),
                (
                    MenuCommand::NetworkInspector,
                    "Network Connections".to_string(),
                ),
                (
                    MenuCommand::VerifySignature,
                    "Verify Digital Signature".to_string(),
                ),
                (
                    MenuCommand::FullExecutableDetails,
                    "Full Executable Details".to_string(),
                ),
                (
                    MenuCommand::ToggleAlerts,
                    "Toggle Resource Alert".to_string(),
                ),
                (MenuCommand::ToggleWatchlist, "Toggle Watchlist".to_string()),
                (
                    MenuCommand::SaveSnapshot,
                    "Save System Snapshot".to_string(),
                ),
                (
                    MenuCommand::CompareSnapshot,
                    "Compare With Snapshot".to_string(),
                ),
            ],
            MenuKind::Control => vec![
                (MenuCommand::PriorityHigh, "Set High Priority".to_string()),
                (
                    MenuCommand::PriorityNormal,
                    "Set Normal Priority".to_string(),
                ),
                (
                    MenuCommand::EfficiencyMode,
                    "Enable Efficiency Mode".to_string(),
                ),
                (MenuCommand::LimitAffinity, "Limit CPU Affinity".to_string()),
                (
                    MenuCommand::SuspendSelected,
                    "Suspend Safe Selection".to_string(),
                ),
                (MenuCommand::ResumeSelected, "Resume Selection".to_string()),
                (
                    MenuCommand::ToggleAutomation,
                    "Toggle Automatic Rule".to_string(),
                ),
                (
                    MenuCommand::StartupManager,
                    "Windows Startup Manager".to_string(),
                ),
                (
                    MenuCommand::ServicesManager,
                    "Windows Services Manager".to_string(),
                ),
            ],
            MenuKind::Tools => vec![
                (
                    MenuCommand::Action(Action::Explain),
                    "Ask Sentinel".to_string(),
                ),
                (
                    MenuCommand::Action(Action::History),
                    "Sentinel History".to_string(),
                ),
                (
                    MenuCommand::SentinelSecurityReport,
                    "Sentinel Security Report".to_string(),
                ),
                (
                    MenuCommand::Action(Action::EndSafe),
                    "End Safe Selection".to_string(),
                ),
                (MenuCommand::OpenLocation, "Open File Location".to_string()),
                (
                    MenuCommand::Action(Action::AutoRefresh),
                    if self.auto_refresh {
                        "Disable Auto Refresh"
                    } else {
                        "Enable Auto Refresh"
                    }
                    .to_string(),
                ),
                (
                    MenuCommand::Action(Action::Admin),
                    if self.elevated {
                        "Turn Admin Off"
                    } else {
                        "Restart As Admin"
                    }
                    .to_string(),
                ),
            ],
            MenuKind::Help => vec![
                (
                    MenuCommand::SentinelSettings,
                    "Sentinel AI Settings...".to_string(),
                ),
                (MenuCommand::About, "About Process Guard".to_string()),
                (
                    MenuCommand::Action(Action::History),
                    "Open Sentinel History".to_string(),
                ),
            ],
        }
    }

    fn menu_dropdown_rects(
        &self,
        layout: &Layout,
        kind: MenuKind,
    ) -> Vec<(MenuCommand, RECT, String)> {
        let Some((_, anchor)) = layout.menus.iter().find(|(menu, _)| *menu == kind) else {
            return Vec::new();
        };
        let items = self.menu_items(kind);
        let width = match kind {
            MenuKind::File => 420,
            MenuKind::View => 430,
            MenuKind::Search => 450,
            MenuKind::Monitor => 540,
            MenuKind::Control => 530,
            MenuKind::Tools => 510,
            MenuKind::Help => 420,
        };
        items
            .into_iter()
            .enumerate()
            .map(|(index, (command, label))| {
                (
                    command,
                    rect(
                        anchor.left,
                        anchor.bottom + 4 + (index as i32 * MENU_ITEM_H),
                        width,
                        MENU_ITEM_H,
                    ),
                    label,
                )
            })
            .collect()
    }

    fn context_items(&self) -> Vec<(MenuCommand, String)> {
        vec![
            (
                MenuCommand::Action(Action::Explain),
                "Ask Sentinel".to_string(),
            ),
            (
                MenuCommand::Action(Action::EndSafe),
                "End Safe Selection".to_string(),
            ),
            (MenuCommand::LivePerformance, "Live Performance".to_string()),
            (
                MenuCommand::NetworkInspector,
                "Network Connections".to_string(),
            ),
            (
                MenuCommand::FullExecutableDetails,
                "Full Executable Details".to_string(),
            ),
            (MenuCommand::ToggleWatchlist, "Toggle Watchlist".to_string()),
            (
                MenuCommand::SuspendSelected,
                "Suspend Safe Selection".to_string(),
            ),
            (MenuCommand::OpenLocation, "Open File Location".to_string()),
            (
                MenuCommand::FilterSelectedType,
                "Filter To This Type".to_string(),
            ),
            (
                MenuCommand::ExpandSelected,
                "Expand Selected Groups".to_string(),
            ),
            (MenuCommand::ClearSelection, "Clear Selection".to_string()),
        ]
    }

    fn context_menu_rects(
        &self,
        client: RECT,
        menu: ContextMenu,
    ) -> Vec<(MenuCommand, RECT, String)> {
        let items = self.context_items();
        let width = 230;
        let height = (items.len() as i32 * 28) + 2;
        let x = menu.x.clamp(
            client.left + 6,
            (client.right - width - 6).max(client.left + 6),
        );
        let y = menu.y.clamp(
            client.top + 6,
            (client.bottom - height - 6).max(client.top + 6),
        );
        items
            .into_iter()
            .enumerate()
            .map(|(index, (command, label))| {
                (command, rect(x, y + (index as i32 * 28), width, 28), label)
            })
            .collect()
    }

    fn draw_ai_overlay(&self, hdc: HDC, client: RECT, body_font: isize, bold_font: isize) {
        unsafe {
            let ai = self.ai_layout(client);
            let r = ai.popup;
            fill(hdc, r, C_MODAL);
            frame(hdc, r, C_CYAN);
            select(hdc, bold_font, C_CYAN);
            draw_text(hdc, AI_TITLE, inset(r, 16, 12), DT_LEFT | DT_SINGLELINE);
            select(hdc, body_font, C_MUTED);
            let engine = provider_by_id(&self.sentinel_config.provider_id)
                .map(|provider| provider.label)
                .unwrap_or("Codex CLI");
            draw_text(
                hdc,
                &format!(
                    "{}  |  {} / {}",
                    self.active_ai_title(),
                    engine,
                    self.sentinel_config.model
                ),
                rect(r.left + 174, r.top + 13, r.right - r.left - 326, 20),
                DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
            );
            draw_ai_size_icon(
                hdc,
                ai.smaller,
                false,
                self.hover == Some(HoverTarget::AiSmaller),
            );
            draw_ai_size_icon(
                hdc,
                ai.larger,
                true,
                self.hover == Some(HoverTarget::AiLarger),
            );
            draw_button(hdc, ai.fit, "Fit", self.hover == Some(HoverTarget::AiFit));
            self.draw_ai_session_history(hdc, &ai, body_font, bold_font);
            self.draw_ai_history(hdc, ai.body, body_font, bold_font);
            draw_ai_input(
                hdc,
                ai.input,
                &self.ai_input,
                self.ai_input_focus,
                self.ai_input_all_selected,
                self.ai_cursor,
            );
            draw_button(hdc, ai.ask, "Ask", self.hover == Some(HoverTarget::AiAsk));
            draw_button(
                hdc,
                ai.regenerate,
                "Regenerate",
                self.hover == Some(HoverTarget::AiRegenerate),
            );
            draw_button(
                hdc,
                ai.pin,
                if self
                    .active_ai_session
                    .and_then(|index| self.ai_sessions.get(index))
                    .map(|session| session.pinned)
                    .unwrap_or(false)
                {
                    "Unpin"
                } else {
                    "Pin"
                },
                self.hover == Some(HoverTarget::AiPin),
            );
            draw_button(
                hdc,
                ai.clear_one,
                "Clear 1",
                self.hover == Some(HoverTarget::AiClearOne),
            );
            draw_button(
                hdc,
                ai.clear_all,
                "Clear All Unpinned",
                self.hover == Some(HoverTarget::AiClearAll),
            );
            for (idx, suggestion) in ai.suggestions.iter().enumerate() {
                draw_button(
                    hdc,
                    *suggestion,
                    SUGGESTED_QUESTIONS[idx].0,
                    self.hover == Some(HoverTarget::AiSuggestion(idx)),
                );
            }
            select(hdc, body_font, C_MUTED);
            draw_text(
                hdc,
                if self.ai_running {
                    "Sentinel is thinking in the background. Enter sends the question."
                } else if !self.ai_hwnd.is_null() {
                    "This is a separate window. Move or resize it like any normal Windows app."
                } else {
                    "Drag title to move. Drag corner to resize. Wheel scrolls. Enter sends. Esc closes."
                },
                rect(r.left + 18, r.bottom - 24, r.right - r.left - 36, 18),
                DT_LEFT | DT_SINGLELINE,
            );
            if self.ai_hwnd.is_null() {
                fill(
                    hdc,
                    rect(ai.resize.right - 18, ai.resize.bottom - 3, 14, 2),
                    C_CYAN,
                );
                fill(
                    hdc,
                    rect(ai.resize.right - 12, ai.resize.bottom - 8, 8, 2),
                    C_CYAN,
                );
                fill(
                    hdc,
                    rect(ai.resize.right - 6, ai.resize.bottom - 13, 2, 2),
                    C_CYAN,
                );
            }
        }
    }

    fn draw_ai_session_history(&self, hdc: HDC, ai: &AiLayout, body_font: isize, bold_font: isize) {
        unsafe {
            fill(hdc, ai.history, C_SIDE);
            frame(hdc, ai.history, C_BORDER);
            select(hdc, bold_font, C_CYAN);
            draw_text(
                hdc,
                "HISTORY",
                rect(
                    ai.history.left + 8,
                    ai.history.top + 8,
                    ai.history.right - ai.history.left - 16,
                    20,
                ),
                DT_LEFT | DT_SINGLELINE,
            );

            let ordered = self.ordered_ai_session_indices();
            let capacity = ai_session_row_capacity(ai);
            if ordered.is_empty() {
                select(hdc, body_font, C_MUTED);
                draw_multiline(
                    hdc,
                    "No saved Sentinel chats yet.",
                    rect(
                        ai.history.left + 8,
                        ai.history.top + 38,
                        ai.history.right - ai.history.left - 16,
                        70,
                    ),
                    16,
                );
            } else {
                let max_scroll = ordered.len().saturating_sub(capacity);
                let scroll = self.ai_session_scroll.min(max_scroll);
                for (row, session_index) in
                    ordered.into_iter().skip(scroll).take(capacity).enumerate()
                {
                    let Some(session) = self.ai_sessions.get(session_index) else {
                        continue;
                    };
                    let r = ai_session_row_rect(ai, row);
                    let active = self.active_ai_session == Some(session_index);
                    fill(hdc, r, if active { C_SELECT_BG } else { C_TABLE });
                    frame(
                        hdc,
                        r,
                        if session.pinned {
                            C_WARN_TEXT
                        } else if active {
                            C_CYAN
                        } else {
                            C_GRID
                        },
                    );
                    select(
                        hdc,
                        if active || session.pinned {
                            bold_font
                        } else {
                            body_font
                        },
                        if session.pinned {
                            C_WARN_TEXT
                        } else if active {
                            C_SELECT_TEXT
                        } else {
                            C_TEXT
                        },
                    );
                    let title = if session.pinned {
                        format!("PIN {}", session.title)
                    } else {
                        session.title.clone()
                    };
                    draw_text(
                        hdc,
                        &title,
                        inset(r, 7, 0),
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
                    );
                }
                let total = self.ai_sessions.len();
                if total > capacity && capacity > 0 {
                    let track = rect(
                        ai.history.right - 7,
                        ai.history.top + 34,
                        4,
                        ai.history.bottom - ai.history.top - 128,
                    );
                    fill(hdc, track, C_GRID);
                    let thumb_h = ((track.bottom - track.top) as f32
                        * (capacity as f32 / total as f32))
                        .max(20.0) as i32;
                    let thumb_y = track.top
                        + (((track.bottom - track.top - thumb_h) as f32)
                            * (scroll as f32 / max_scroll.max(1) as f32))
                            as i32;
                    fill(hdc, rect(track.left - 1, thumb_y, 6, thumb_h), C_CYAN);
                }
            }
        }
    }

    fn draw_ai_history(&self, hdc: HDC, body: RECT, body_font: isize, bold_font: isize) {
        unsafe {
            fill(hdc, body, C_TABLE);
            frame(hdc, body, C_GRID);
            let lines = self.ai_render_lines(body);
            let visible = self.ai_body_visible_lines(body).max(1);
            let max_scroll = lines.len().saturating_sub(visible);
            let scroll = self.ai_body_scroll.min(max_scroll);
            let mut y = body.top + 8;
            for line in lines.iter().skip(scroll).take(visible) {
                if let Some(color) = line.marker {
                    fill(hdc, rect(body.left + 5, y + 3, 3, 14), color);
                }
                select(
                    hdc,
                    if line.bold { bold_font } else { body_font },
                    line.color,
                );
                draw_text(
                    hdc,
                    &line.text,
                    rect(body.left + 14, y, body.right - body.left - 28, 18),
                    DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
                );
                y += 18;
            }
            if lines.len() > visible {
                let track = rect(body.right - 7, body.top + 6, 4, body.bottom - body.top - 12);
                fill(hdc, track, C_GRID);
                let thumb_h = ((track.bottom - track.top) as f32
                    * (visible as f32 / lines.len() as f32))
                    .max(24.0) as i32;
                let thumb_y = track.top
                    + (((track.bottom - track.top - thumb_h) as f32)
                        * (scroll as f32 / max_scroll.max(1) as f32)) as i32;
                fill(hdc, rect(track.left - 1, thumb_y, 6, thumb_h), C_CYAN);
            }
        }
    }

    fn ai_body_visible_lines(&self, body: RECT) -> usize {
        ((body.bottom - body.top - 16) / 18).max(1) as usize
    }

    fn max_ai_body_scroll(&self, body: RECT) -> usize {
        self.ai_render_lines(body)
            .len()
            .saturating_sub(self.ai_body_visible_lines(body))
    }

    fn scroll_ai_at_hover(&mut self, client: RECT, delta: i32) {
        let ai = self.ai_layout(client);
        if point_in(ai.body, self.hover_x, self.hover_y) {
            let max_scroll = self.max_ai_body_scroll(ai.body);
            if delta > 0 {
                self.ai_body_scroll = self.ai_body_scroll.saturating_sub(4);
            } else {
                self.ai_body_scroll = (self.ai_body_scroll + 4).min(max_scroll);
            }
            return;
        }
        if point_in(ai.history, self.hover_x, self.hover_y) {
            let max_scroll = self
                .ordered_ai_session_indices()
                .len()
                .saturating_sub(ai_session_row_capacity(&ai));
            if delta > 0 {
                self.ai_session_scroll = self.ai_session_scroll.saturating_sub(2);
            } else {
                self.ai_session_scroll = (self.ai_session_scroll + 2).min(max_scroll);
            }
            return;
        }
        let max_scroll = self.max_ai_body_scroll(ai.body);
        if delta > 0 {
            self.ai_body_scroll = self.ai_body_scroll.saturating_sub(4);
        } else {
            self.ai_body_scroll = (self.ai_body_scroll + 4).min(max_scroll);
        }
    }

    fn ai_render_lines(&self, body: RECT) -> Vec<AiRenderLine> {
        let max_chars = (((body.right - body.left - 28) / 8).max(24)) as usize;
        let mut lines = Vec::new();
        if self.ai_messages.is_empty() && !self.ai_running {
            push_ai_block(
                &mut lines,
                "Sentinel history",
                "No Sentinel history yet. Select a row, click Ask Sentinel, then ask follow-up questions here.",
                C_CYAN,
                C_MUTED,
                max_chars,
            );
            return lines;
        }
        if self.ai_running {
            push_ai_block(
                &mut lines,
                "Status",
                &format!("{} is thinking in the background...", AI_NAME),
                C_WARN_TEXT,
                C_MUTED,
                max_chars,
            );
        }
        for message in self.ai_messages.iter().rev() {
            let (header_color, body_color) = ai_message_colors(message.speaker);
            push_ai_block(
                &mut lines,
                message.speaker,
                &message.text,
                header_color,
                body_color,
                max_chars,
            );
        }
        lines
    }

    fn draw_hover_tip(&self, hdc: HDC, client: RECT, body_font: isize) {
        let Some(target) = self.hover else {
            return;
        };
        unsafe {
            let text = target.text();
            let width = 390.min((client.right - client.left - 30).max(180));
            let height = 44;
            let mut x = self.hover_x + 16;
            let mut y = self.hover_y + 18;
            if x + width > client.right - 10 {
                x = client.right - width - 10;
            }
            if y + height > client.bottom - 10 {
                y = self.hover_y - height - 12;
            }
            let r = rect(x.max(10), y.max(10), width, height);
            fill(hdc, r, 0x11180a);
            frame(hdc, r, C_CYAN);
            select(hdc, body_font, C_TEXT);
            draw_multiline(hdc, text, inset(r, 8, 6), 16);
        }
    }

    fn on_click(&mut self, hwnd: HWND, x: i32, y: i32, double: bool) {
        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(hwnd, &mut client);
        }
        let layout = self.layout(client);

        if self.sentinel_settings_open {
            self.on_sentinel_settings_click(hwnd, client, x, y);
            return;
        }

        if self.launcher_open {
            let launcher = launcher_layout(client);
            if !point_in(launcher.popup, x, y) || point_in(launcher.close, x, y) {
                self.launcher_open = false;
                self.launcher_focus = false;
                return;
            }
            self.launcher_focus = false;
            if point_in(launcher.input, x, y) {
                self.launcher_focus = true;
                return;
            }
            if point_in(launcher.start, x, y) {
                self.start_launcher_process(hwnd);
                return;
            }
            if point_in(launcher.cancel, x, y) {
                self.launcher_open = false;
                return;
            }
            return;
        }

        if self.ai_overlay && self.ai_hwnd.is_null() {
            self.on_ai_click(hwnd, hwnd, x, y);
            return;
        }

        if let Some(kind) = self.open_menu {
            for (command, r, _) in self.menu_dropdown_rects(&layout, kind) {
                if point_in(r, x, y) {
                    self.open_menu = None;
                    self.context_menu = None;
                    self.run_menu_command(hwnd, command);
                    return;
                }
            }
        }

        for (kind, r) in &layout.menus {
            if point_in(*r, x, y) {
                self.open_menu = if self.open_menu == Some(*kind) {
                    None
                } else {
                    Some(*kind)
                };
                self.context_menu = None;
                self.search_focus = false;
                return;
            }
        }

        if let Some(menu) = self.context_menu {
            for (command, r, _) in self.context_menu_rects(client, menu) {
                if point_in(r, x, y) {
                    self.context_menu = None;
                    self.open_menu = None;
                    self.run_menu_command(hwnd, command);
                    return;
                }
            }
            self.context_menu = None;
        }

        self.open_menu = None;

        for (action, r) in &layout.buttons {
            if point_in(*r, x, y) {
                self.run_action(hwnd, *action);
                return;
            }
        }

        if !self.search.is_empty() && point_in(layout.search_clear, x, y) {
            self.search.clear();
            self.apply_filters();
            self.clamp_scroll_to_window(hwnd);
            self.search_focus = true;
            self.status = "Search cleared.".to_string();
            return;
        }

        if point_in(layout.search, x, y) {
            self.search_focus = true;
            return;
        }
        self.search_focus = false;

        let bar_top = layout.table.bottom - HSCROLL_H;
        if y >= bar_top
            && y <= layout.table.bottom
            && x >= layout.table.left + FROZEN_NAME_W
            && x <= layout.table.right
        {
            let track_left = layout.table.left + FROZEN_NAME_W + 8;
            let track_width = (layout.table.right - track_left - 8).max(1);
            let ratio = ((x - track_left) as f32 / track_width as f32).clamp(0.0, 1.0);
            self.hscroll = (ratio * self.max_hscroll_for_table(layout.table) as f32) as i32;
            self.status =
                "Horizontal table position changed. Process/Group stays visible.".to_string();
            return;
        }

        if x >= layout.table.left
            && x <= layout.table.right
            && y >= layout.table.top
            && y <= layout.table.top + HEADER_H
        {
            for (col, r) in &layout.columns {
                if point_in(*r, x, y) {
                    self.handle_header(*col);
                    return;
                }
            }
        }

        let rows_top = layout.table.top + HEADER_H;
        if x >= layout.table.left
            && x <= layout.table.right
            && y >= rows_top
            && y <= layout.table.bottom - HSCROLL_H
        {
            let row_index = self.scroll + ((y - rows_top) / ROW_H) as usize;
            if row_index < self.visible_rows.len() {
                let row = self.visible_rows[row_index];
                if double || (matches!(row, RowKind::Group(_)) && x < layout.table.left + 42) {
                    self.toggle_group(row);
                    return;
                }
                let id = self.row_id(row);
                let ctrl = unsafe { GetKeyState(VK_CONTROL as i32) } < 0;
                if ctrl {
                    if !self.selected.remove(&id) {
                        self.selected.insert(id);
                    }
                } else {
                    self.selected.clear();
                    self.selected.insert(id);
                }
                self.details = self.build_selection_details();
                self.status = format!("{} selected | {}", self.selected.len(), self.status_base());
            }
        }
    }

    fn open_sentinel_settings(&mut self) {
        self.sentinel_selected_provider =
            provider_index(&self.sentinel_config.provider_id).unwrap_or(0);
        self.sentinel_model_input = self.sentinel_config.model.clone();
        if self.sentinel_model_input.trim().is_empty() {
            self.sentinel_model_input = SENTINEL_PROVIDERS[self.sentinel_selected_provider]
                .models
                .first()
                .copied()
                .unwrap_or("default")
                .to_string();
        }
        self.sentinel_key_input.clear();
        self.sentinel_key_stored = self.installed_copy
            && SENTINEL_PROVIDERS[self.sentinel_selected_provider].is_api()
            && read_api_key(SENTINEL_PROVIDERS[self.sentinel_selected_provider].id).is_some();
        self.sentinel_settings_focus = SentinelSettingsFocus::Model;
        self.sentinel_text_all_selected = false;
        self.sentinel_test_running = false;
        self.sentinel_test_message.clear();
        self.sentinel_test_success = false;
        self.sentinel_settings_open = true;
        self.launcher_open = false;
        self.search_focus = false;
        self.open_menu = None;
        self.context_menu = None;
        self.hover = None;
        self.status = "Sentinel AI settings opened.".to_string();
    }

    fn select_sentinel_provider(&mut self, index: usize) {
        if self.sentinel_test_running {
            return;
        }
        let Some(provider) = SENTINEL_PROVIDERS.get(index) else {
            return;
        };
        self.sentinel_selected_provider = index;
        self.sentinel_model_input = if self.sentinel_config.provider_id == provider.id {
            self.sentinel_config.model.clone()
        } else {
            provider
                .models
                .first()
                .copied()
                .unwrap_or("default")
                .to_string()
        };
        self.sentinel_key_input.clear();
        self.sentinel_key_stored = provider.is_api() && read_api_key(provider.id).is_some();
        self.sentinel_settings_focus = SentinelSettingsFocus::Model;
        self.sentinel_text_all_selected = false;
        self.sentinel_test_message.clear();
        self.sentinel_test_success = false;
        self.status = if provider.is_api() && !self.installed_copy {
            format!(
                "{} details opened. Install Process Guard with Setup to configure this API.",
                provider.label
            )
        } else {
            format!("Selected Sentinel engine: {}.", provider.label)
        };
    }

    fn on_sentinel_settings_click(&mut self, hwnd: HWND, client: RECT, x: i32, y: i32) {
        let ui = sentinel_settings_layout(client);
        if self.sentinel_test_running {
            self.status =
                "Sentinel is testing the selected engine and model. Please wait.".to_string();
            return;
        }
        if !point_in(ui.popup, x, y) || point_in(ui.close, x, y) || point_in(ui.cancel, x, y) {
            self.sentinel_settings_open = false;
            self.sentinel_key_input.clear();
            self.sentinel_text_all_selected = false;
            self.status = "Sentinel settings closed without changes.".to_string();
            return;
        }
        for index in 0..SENTINEL_PROVIDERS.len() {
            if point_in(sentinel_provider_row(&ui, index), x, y) {
                self.select_sentinel_provider(index);
                return;
            }
        }
        if point_in(ui.model_input, x, y) {
            self.sentinel_settings_focus = SentinelSettingsFocus::Model;
            self.sentinel_text_all_selected = false;
            return;
        }
        let provider = &SENTINEL_PROVIDERS[self.sentinel_selected_provider];
        for (index, button) in ui.model_choices.iter().enumerate() {
            if point_in(*button, x, y) {
                if let Some(model) = provider.models.get(index) {
                    self.sentinel_model_input = (*model).to_string();
                    self.sentinel_settings_focus = SentinelSettingsFocus::Model;
                    self.sentinel_text_all_selected = false;
                    self.sentinel_test_message.clear();
                    self.sentinel_test_success = false;
                }
                return;
            }
        }
        if provider.is_api() && self.installed_copy && point_in(ui.key_input, x, y) {
            self.sentinel_settings_focus = SentinelSettingsFocus::ApiKey;
            self.sentinel_text_all_selected = false;
            return;
        }
        if provider.is_api() && self.installed_copy && point_in(ui.clear_key, x, y) {
            self.clear_selected_api_key();
            return;
        }
        if point_in(ui.save, x, y) {
            self.save_sentinel_settings(hwnd);
        }
    }

    fn clear_selected_api_key(&mut self) {
        let provider = &SENTINEL_PROVIDERS[self.sentinel_selected_provider];
        if !provider.is_api() || !self.installed_copy {
            return;
        }
        if delete_api_key(provider.id) {
            self.sentinel_key_input.clear();
            self.sentinel_key_stored = false;
            self.sentinel_test_message.clear();
            self.sentinel_test_success = false;
            self.status = format!("Removed the stored {} API key.", provider.label);
        } else {
            self.status = format!(
                "Windows could not remove the stored {} API key.",
                provider.label
            );
        }
    }

    fn handle_sentinel_settings_char(&mut self, ch: u32) {
        if self.sentinel_test_running {
            return;
        }
        self.sentinel_test_message.clear();
        self.sentinel_test_success = false;
        match ch {
            8 => {
                if self.sentinel_text_all_selected {
                    self.clear_sentinel_active_text();
                    self.sentinel_text_all_selected = false;
                } else {
                    match self.sentinel_settings_focus {
                        SentinelSettingsFocus::Model => {
                            self.sentinel_model_input.pop();
                        }
                        SentinelSettingsFocus::ApiKey => {
                            self.sentinel_key_input.pop();
                        }
                    }
                }
            }
            13 | 27 => {}
            value if (32..127).contains(&value) => {
                if let Some(character) = char::from_u32(value) {
                    self.insert_sentinel_text(&character.to_string());
                }
            }
            _ => {}
        }
    }

    fn handle_sentinel_settings_keydown(&mut self, hwnd: HWND, key: u16) {
        if self.sentinel_test_running {
            return;
        }
        let control = unsafe { GetKeyState(VK_CONTROL as i32) } < 0;
        if control && key == b'A' as u16 {
            self.sentinel_text_all_selected = true;
            return;
        }
        if control && key == b'V' as u16 {
            if let Some(value) = unsafe { clipboard_text(hwnd) } {
                self.insert_sentinel_text(value.trim());
            }
            return;
        }
        match key {
            VK_TAB => {
                let provider = &SENTINEL_PROVIDERS[self.sentinel_selected_provider];
                self.sentinel_settings_focus = if self.sentinel_settings_focus
                    == SentinelSettingsFocus::Model
                    && provider.is_api()
                    && self.installed_copy
                {
                    SentinelSettingsFocus::ApiKey
                } else {
                    SentinelSettingsFocus::Model
                };
                self.sentinel_text_all_selected = false;
            }
            VK_RETURN => self.save_sentinel_settings(hwnd),
            VK_ESCAPE => {
                self.sentinel_settings_open = false;
                self.sentinel_key_input.clear();
                self.sentinel_text_all_selected = false;
                self.status = "Sentinel settings closed without changes.".to_string();
            }
            _ => {}
        }
    }

    fn insert_sentinel_text(&mut self, value: &str) {
        self.sentinel_test_message.clear();
        self.sentinel_test_success = false;
        if self.sentinel_text_all_selected {
            self.clear_sentinel_active_text();
            self.sentinel_text_all_selected = false;
        }
        let sanitized = value
            .chars()
            .filter(|character| !character.is_control())
            .collect::<String>();
        match self.sentinel_settings_focus {
            SentinelSettingsFocus::Model => {
                let remaining = 160usize.saturating_sub(self.sentinel_model_input.chars().count());
                self.sentinel_model_input
                    .extend(sanitized.chars().take(remaining));
            }
            SentinelSettingsFocus::ApiKey => {
                let remaining = 2400usize.saturating_sub(self.sentinel_key_input.chars().count());
                self.sentinel_key_input
                    .extend(sanitized.chars().take(remaining));
            }
        }
    }

    fn clear_sentinel_active_text(&mut self) {
        match self.sentinel_settings_focus {
            SentinelSettingsFocus::Model => self.sentinel_model_input.clear(),
            SentinelSettingsFocus::ApiKey => self.sentinel_key_input.clear(),
        }
    }

    fn save_sentinel_settings(&mut self, hwnd: HWND) {
        if self.sentinel_test_running {
            return;
        }
        let provider = &SENTINEL_PROVIDERS[self.sentinel_selected_provider];
        let model = self.sentinel_model_input.trim().to_string();
        if model.is_empty() {
            self.status = "Enter a model ID or select a suggested model.".to_string();
            self.sentinel_settings_focus = SentinelSettingsFocus::Model;
            return;
        }
        let api_key = if provider.is_api() {
            if !self.installed_copy {
                self.status = "API providers are available only in the installed app.".to_string();
                return;
            }
            let new_key = self.sentinel_key_input.trim().to_string();
            if new_key.is_empty() && !self.sentinel_key_stored {
                self.status = format!("Paste a {} API key before saving.", provider.label);
                self.sentinel_settings_focus = SentinelSettingsFocus::ApiKey;
                return;
            }
            if new_key.is_empty() {
                read_api_key(provider.id)
            } else {
                Some(new_key)
            }
        } else {
            None
        };
        let config = SentinelConfig {
            provider_id: provider.id.to_string(),
            model,
        };
        self.sentinel_test_running = true;
        self.sentinel_test_success = false;
        self.sentinel_test_message = format!(
            "Testing {} / {} before saving...",
            provider.label, config.model
        );
        self.status = self.sentinel_test_message.clone();
        let hwnd_value = hwnd as isize;
        thread::spawn(move || {
            let result = test_sentinel_config(&config, api_key.as_deref());
            unsafe {
                PostMessageW(
                    hwnd_value as HWND,
                    WM_SENTINEL_TEST_DONE,
                    0,
                    Box::into_raw(Box::new(result)) as LPARAM,
                );
            }
        });
    }

    fn commit_sentinel_settings(&mut self, hwnd: HWND) {
        let provider = &SENTINEL_PROVIDERS[self.sentinel_selected_provider];
        let model = self.sentinel_model_input.trim().to_string();
        let new_key = self.sentinel_key_input.trim().to_string();
        if provider.is_api() && !new_key.is_empty() {
            if let Err(error) = write_api_key(provider.id, &new_key) {
                self.sentinel_test_success = false;
                self.sentinel_test_message = format!(
                    "Model test passed, but Windows Credential Manager could not store the key (error {}).",
                    error
                );
                self.status = self.sentinel_test_message.clone();
                return;
            }
            self.sentinel_key_stored = true;
        }
        self.sentinel_config = SentinelConfig {
            provider_id: provider.id.to_string(),
            model,
        };
        save_guard_settings(
            &self.watchlist,
            &self.alert_rules,
            &self.automation_rules,
            &self.sentinel_config,
        );
        self.sentinel_key_input.clear();
        self.status = format!(
            "Test passed. Saved {} for Sentinel. Restarting Process Guard...",
            provider.label
        );
        if unsafe { relaunch_current_copy(hwnd) } {
            unsafe {
                DestroyWindow(hwnd);
            }
        } else {
            self.status =
                "Settings were saved, but Windows could not restart Process Guard.".to_string();
            self.sentinel_settings_open = false;
        }
    }

    fn on_mouse_move(&mut self, hwnd: HWND, x: i32, y: i32) -> bool {
        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(hwnd, &mut client);
        }
        let layout = self.layout(client);
        let previous = self.hover;
        self.hover_x = x;
        self.hover_y = y;

        if self.sentinel_settings_open {
            self.hover = None;
            return previous != self.hover;
        }

        if self.ai_overlay && self.ai_hwnd.is_null() && self.ai_dragging {
            self.ai_popup_pos = Some((x - self.ai_drag_dx, y - self.ai_drag_dy));
            let ai = self.ai_layout(client);
            self.ai_popup_pos = Some((ai.popup.left, ai.popup.top));
            return true;
        }

        if self.ai_overlay && self.ai_hwnd.is_null() && self.ai_resizing {
            let ai = self.ai_layout(client);
            self.ai_popup_size = Some((
                x - ai.popup.left + self.ai_resize_dx,
                y - ai.popup.top + self.ai_resize_dy,
            ));
            let ai = self.ai_layout(client);
            self.ai_popup_pos = Some((ai.popup.left, ai.popup.top));
            self.ai_popup_size = Some((
                ai.popup.right - ai.popup.left,
                ai.popup.bottom - ai.popup.top,
            ));
            return true;
        }

        self.hover = None;
        if self.launcher_open {
            return previous != self.hover;
        }
        if self.ai_overlay && self.ai_hwnd.is_null() {
            self.hover = self.ai_hover_for_point(client, x, y);
        } else {
            if !self.search.is_empty() && point_in(layout.search_clear, x, y) {
                self.hover = Some(HoverTarget::ClearSearch);
                return previous != self.hover;
            }
            for (action, r) in &layout.buttons {
                if point_in(*r, x, y) {
                    self.hover = Some(HoverTarget::Main(*action));
                    break;
                }
            }
        }

        previous != self.hover
    }

    fn on_ai_click(&mut self, event_hwnd: HWND, main_hwnd: HWND, x: i32, y: i32) {
        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(event_hwnd, &mut client);
        }
        let ai = self.ai_layout(client);
        if !point_in(ai.popup, x, y) {
            if self.ai_hwnd.is_null() {
                self.ai_overlay = false;
                self.ai_input_focus = false;
                self.hover = None;
            }
            return;
        }
        self.ai_input_focus = false;
        if point_in(ai.smaller, x, y) {
            self.change_ai_popup_size(client, -110, -70);
            return;
        }
        if point_in(ai.larger, x, y) {
            self.change_ai_popup_size(client, 110, 70);
            return;
        }
        if point_in(ai.fit, x, y) {
            if self.ai_hwnd.is_null() {
                self.ai_popup_pos = None;
                self.ai_popup_size = None;
            } else {
                unsafe {
                    self.fit_ai_window();
                }
            }
            self.ai_body_scroll = 0;
            return;
        }
        if self.ai_hwnd.is_null() && point_in(ai.resize, x, y) {
            self.ai_resizing = true;
            self.ai_resize_dx = ai.popup.right - x;
            self.ai_resize_dy = ai.popup.bottom - y;
            self.ai_input_focus = false;
            return;
        }
        if self.ai_hwnd.is_null() && point_in(ai.drag, x, y) {
            self.ai_dragging = true;
            self.ai_drag_dx = x - ai.popup.left;
            self.ai_drag_dy = y - ai.popup.top;
            self.ai_input_focus = false;
            return;
        }
        if point_in(ai.input, x, y) {
            self.ai_input_focus = true;
            self.ai_cursor = self.ai_input.chars().count();
            self.ai_input_all_selected = false;
            return;
        }
        if point_in(ai.ask, x, y) {
            self.submit_ai_question(main_hwnd);
            return;
        }
        if point_in(ai.regenerate, x, y) {
            self.regenerate_ai(main_hwnd);
            return;
        }
        if point_in(ai.pin, x, y) {
            self.toggle_pin_active_ai();
            return;
        }
        if point_in(ai.clear_one, x, y) {
            self.clear_active_ai_session();
            return;
        }
        if point_in(ai.clear_all, x, y) {
            self.clear_unpinned_ai_sessions();
            return;
        }
        let ordered = self.ordered_ai_session_indices();
        let session_scroll = self
            .ai_session_scroll
            .min(ordered.len().saturating_sub(ai_session_row_capacity(&ai)));
        for (row, session_index) in ordered
            .into_iter()
            .skip(session_scroll)
            .take(ai_session_row_capacity(&ai))
            .enumerate()
        {
            if point_in(ai_session_row_rect(&ai, row), x, y) {
                self.load_ai_session(session_index);
                return;
            }
        }
        for (idx, suggestion) in ai.suggestions.iter().enumerate() {
            if point_in(*suggestion, x, y) {
                self.ai_input = SUGGESTED_QUESTIONS[idx].1.to_string();
                self.ai_cursor = self.ai_input.chars().count();
                self.ai_input_all_selected = false;
                self.submit_ai_question(main_hwnd);
                return;
            }
        }
    }

    fn on_ai_mouse_move(&mut self, hwnd: HWND, x: i32, y: i32) -> bool {
        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(hwnd, &mut client);
        }
        let previous = self.hover;
        self.hover_x = x;
        self.hover_y = y;
        self.hover = self.ai_hover_for_point(client, x, y);
        previous != self.hover
    }

    fn ai_hover_for_point(&self, client: RECT, x: i32, y: i32) -> Option<HoverTarget> {
        let ai = self.ai_layout(client);
        if point_in(ai.ask, x, y) {
            Some(HoverTarget::AiAsk)
        } else if point_in(ai.regenerate, x, y) {
            Some(HoverTarget::AiRegenerate)
        } else if point_in(ai.pin, x, y) {
            Some(HoverTarget::AiPin)
        } else if point_in(ai.clear_one, x, y) {
            Some(HoverTarget::AiClearOne)
        } else if point_in(ai.clear_all, x, y) {
            Some(HoverTarget::AiClearAll)
        } else if point_in(ai.smaller, x, y) {
            Some(HoverTarget::AiSmaller)
        } else if point_in(ai.larger, x, y) {
            Some(HoverTarget::AiLarger)
        } else if point_in(ai.fit, x, y) {
            Some(HoverTarget::AiFit)
        } else if self.ai_hwnd.is_null() && point_in(ai.resize, x, y) {
            Some(HoverTarget::AiResize)
        } else {
            let ordered = self.ordered_ai_session_indices();
            let session_scroll = self
                .ai_session_scroll
                .min(ordered.len().saturating_sub(ai_session_row_capacity(&ai)));
            let session_hover = ordered
                .iter()
                .skip(session_scroll)
                .take(ai_session_row_capacity(&ai))
                .enumerate()
                .find(|(row, _)| point_in(ai_session_row_rect(&ai, *row), x, y))
                .map(|(_, index)| HoverTarget::AiSession(*index));
            session_hover.or_else(|| {
                ai.suggestions
                    .iter()
                    .enumerate()
                    .find(|(_, r)| point_in(**r, x, y))
                    .map(|(idx, _)| HoverTarget::AiSuggestion(idx))
            })
        }
    }

    fn on_right_click(&mut self, _hwnd: HWND, x: i32, y: i32) {
        if self.sentinel_settings_open
            || (self.ai_overlay && self.ai_hwnd.is_null())
            || self.launcher_open
        {
            self.context_menu = None;
            self.open_menu = None;
            return;
        }

        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(_hwnd, &mut client);
        }
        let layout = self.layout(client);
        self.open_menu = None;
        self.search_focus = false;

        let Some(row) = self.row_at_point(&layout, x, y) else {
            self.context_menu = None;
            return;
        };
        let id = self.row_id(row);
        if !self.selected.contains(&id) {
            self.selected.clear();
            self.selected.insert(id);
        }
        self.details = self.build_selection_details();
        self.context_menu = Some(ContextMenu { x, y });
        self.status = "Right-click menu opened for selected process row.".to_string();
    }

    fn row_at_point(&self, layout: &Layout, x: i32, y: i32) -> Option<RowKind> {
        let rows_top = layout.table.top + HEADER_H;
        if x < layout.table.left
            || x > layout.table.right
            || y < rows_top
            || y > layout.table.bottom - HSCROLL_H
        {
            return None;
        }
        let row_index = self.scroll + ((y - rows_top) / ROW_H) as usize;
        self.visible_rows.get(row_index).copied()
    }

    fn handle_ai_char(&mut self, _hwnd: HWND, ch: u32) {
        match ch {
            8 => {
                if self.ai_input_all_selected {
                    self.ai_input.clear();
                    self.ai_cursor = 0;
                    self.ai_input_all_selected = false;
                } else if self.ai_cursor > 0 {
                    let end = byte_index_at_char(&self.ai_input, self.ai_cursor);
                    let start = byte_index_at_char(&self.ai_input, self.ai_cursor - 1);
                    self.ai_input.replace_range(start..end, "");
                    self.ai_cursor -= 1;
                }
            }
            13 => {}
            27 => {
                if self.ai_hwnd.is_null() {
                    self.ai_overlay = false;
                    self.ai_input_focus = false;
                } else {
                    unsafe {
                        self.close_ai_window();
                    }
                }
            }
            value if value >= 32 => {
                if let Some(ch) = char::from_u32(value) {
                    if self.ai_input.chars().count() < 4000 {
                        self.insert_ai_character(ch);
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_ai_keydown(&mut self, hwnd: HWND, key: u16) {
        let control = unsafe { GetKeyState(VK_CONTROL as i32) } < 0;
        let shift = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
        if control && key == b'A' as u16 {
            self.ai_input_all_selected = !self.ai_input.is_empty();
            self.ai_cursor = self.ai_input.chars().count();
            return;
        }
        self.ai_input_all_selected = false;
        match key {
            VK_LEFT => self.ai_cursor = self.ai_cursor.saturating_sub(1),
            VK_RIGHT => self.ai_cursor = (self.ai_cursor + 1).min(self.ai_input.chars().count()),
            VK_HOME => self.ai_cursor = 0,
            VK_END => self.ai_cursor = self.ai_input.chars().count(),
            VK_RETURN if shift => self.insert_ai_character('\n'),
            VK_RETURN => self.submit_ai_question(hwnd),
            _ => {}
        }
    }

    fn insert_ai_character(&mut self, ch: char) {
        if self.ai_input_all_selected {
            self.ai_input.clear();
            self.ai_cursor = 0;
            self.ai_input_all_selected = false;
        }
        let at = byte_index_at_char(&self.ai_input, self.ai_cursor);
        self.ai_input.insert(at, ch);
        self.ai_cursor += 1;
    }

    fn handle_launcher_char(&mut self, hwnd: HWND, ch: u32) {
        match ch {
            8 => {
                self.launcher_input.pop();
            }
            13 => self.start_launcher_process(hwnd),
            27 => {
                self.launcher_open = false;
                self.launcher_focus = false;
            }
            value if value >= 32 && value < 127 => {
                if let Some(ch) = char::from_u32(value) {
                    if self.launcher_input.len() < 260 {
                        self.launcher_input.push(ch);
                    }
                }
            }
            _ => {}
        }
    }

    fn start_launcher_process(&mut self, hwnd: HWND) {
        let input = self.launcher_input.trim().to_string();
        if input.is_empty() {
            self.status = "Type an app name or executable path first.".to_string();
            return;
        }
        let started = unsafe { launch_user_process(hwnd, &input) };
        if started {
            self.launcher_open = false;
            self.launcher_focus = false;
            self.status = format!("Started process command: {}", input);
            self.refresh();
        } else {
            unsafe {
                show_message(
                    hwnd,
                    "Start Process",
                    "Windows could not start that command. Try an executable name like notepad.exe or quote the full path.",
                    MB_OK | MB_ICONWARNING,
                );
            }
            self.status = "Start Process failed.".to_string();
        }
    }

    fn submit_ai_question(&mut self, hwnd: HWND) {
        let question = self.ai_input.trim().to_string();
        if question.is_empty() {
            self.status = "Type a question in the AI chat box first.".to_string();
            return;
        }
        if self.ai_running {
            self.status =
                "Sentinel is still answering. Wait for the current response first.".to_string();
            return;
        }
        self.ai_input.clear();
        self.ai_cursor = 0;
        self.ai_input_all_selected = false;
        self.ai_messages
            .push(AiMessage::new(SPEAKER_USER, question.clone()));
        self.ai_body_scroll = 0;
        self.sync_active_ai_session();
        let prompt = followup_prompt(&self.ai_context, &self.ai_prompt_history(), &question);
        self.start_ai_request(hwnd, prompt, "Sentinel is answering your follow-up...");
    }

    fn regenerate_ai(&mut self, hwnd: HWND) {
        if self.ai_running {
            self.status =
                "Sentinel is still answering. Wait for the current response first.".to_string();
            return;
        }
        if self.ai_context.trim().is_empty() {
            self.explain(hwnd);
            return;
        }
        self.ai_messages.push(AiMessage::new(
            SPEAKER_USER,
            "Regenerate the explanation with more useful detail.".to_string(),
        ));
        self.ai_body_scroll = 0;
        self.sync_active_ai_session();
        let prompt = regenerate_prompt(&self.ai_context, &self.ai_prompt_history());
        self.start_ai_request(hwnd, prompt, "Sentinel is regenerating the explanation...");
    }

    fn start_ai_request(&mut self, hwnd: HWND, prompt: String, status: &str) {
        self.ai_overlay = true;
        self.ai_running = true;
        self.ai_input_focus = true;
        self.status = status.to_string();
        let hwnd_value = hwnd as isize;
        let config = self.sentinel_config.clone();
        thread::spawn(move || {
            let text = sentinel_answer(&config, &prompt);
            unsafe {
                PostMessageW(
                    hwnd_value as HWND,
                    WM_AI_DONE,
                    0,
                    Box::into_raw(Box::new(text)) as LPARAM,
                );
            }
        });
    }

    fn selected_process_indices(&self) -> Vec<usize> {
        let mut pids = BTreeSet::new();
        let mut indices = Vec::new();
        for row in &self.visible_rows {
            if !self.selected.contains(&self.row_id(*row)) {
                continue;
            }
            let candidates: Vec<usize> = match *row {
                RowKind::Group(index) => self.groups[index].process_indices.clone(),
                RowKind::Process(index) => vec![index],
            };
            for index in candidates {
                if pids.insert(self.processes[index].pid) {
                    indices.push(index);
                }
            }
        }
        indices
    }

    fn selected_rule_keys(&self) -> BTreeSet<String> {
        self.selected_process_indices()
            .into_iter()
            .map(|index| process_group_key(&self.processes[index]))
            .collect()
    }

    fn open_monitor_report(&mut self, hwnd: HWND, title: &str, report: String) {
        self.monitor_title = title.to_string();
        self.monitor_report = report;
        self.monitor_scroll = 0;
        self.status = format!("Opened {} in Monitor Center.", title);
        unsafe { self.ensure_monitor_window(hwnd) };
    }

    unsafe fn ensure_monitor_window(&mut self, main_hwnd: HWND) {
        let window_title = wide(&format!("Process Guard Monitor - {}", self.monitor_title));
        if !self.monitor_hwnd.is_null() {
            SetWindowTextW(self.monitor_hwnd, window_title.as_ptr());
            ShowWindow(self.monitor_hwnd, SW_SHOWNORMAL);
            SetForegroundWindow(self.monitor_hwnd);
            InvalidateRect(self.monitor_hwnd, null(), 0);
            return;
        }
        let instance = GetModuleHandleW(null()) as HINSTANCE;
        let class_name = wide(MONITOR_WINDOW_CLASS);
        let sw = GetSystemMetrics(SM_CXSCREEN).max(900);
        let sh = GetSystemMetrics(SM_CYSCREEN).max(620);
        let width = 940.min((sw - 100).max(680));
        let height = 680.min((sh - 100).max(480));
        let monitor = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            ((sw - width) / 2).max(20),
            ((sh - height) / 2).max(20),
            width,
            height,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        if monitor.is_null() {
            self.status = "Windows could not open Monitor Center.".to_string();
            return;
        }
        SetWindowLongPtrW(monitor, GWLP_USERDATA, main_hwnd as isize);
        self.monitor_hwnd = monitor;
        load_app_icon(monitor);
        apply_dark_title_bar(monitor);
        ShowWindow(monitor, SW_SHOWNORMAL);
        SetForegroundWindow(monitor);
        UpdateWindow(monitor);
    }

    fn show_live_performance(&mut self, hwnd: HWND) {
        let report = self.build_live_performance_report();
        self.open_monitor_report(hwnd, "Live Performance", report);
    }

    fn build_live_performance_report(&self) -> String {
        let indices = self.sorted_process_indices();
        let mut report =
            String::from("LIVE PERFORMANCE\r\nCPU graph scale: low [.] to high [@]\r\n\r\n");
        for index in indices.into_iter().take(50) {
            let process = &self.processes[index];
            let graph = self
                .performance_history
                .get(&process.pid)
                .map(|values| performance_graph(values))
                .unwrap_or_else(|| ".".to_string());
            report.push_str(&format!(
                "{} (PID {})\r\nCPU: {:.1}% | RAM: {} | Disk I/O: {} | Network: {} connection(s) | Threads: {} | Handles: {}\r\nCPU history: {}\r\n\r\n",
                process.name,
                process.pid,
                process.cpu_percent,
                format_memory(process.memory_kb),
                format_rate(process.io_rate_kbps),
                process.network_connections,
                process.thread_count,
                process.handle_count,
                graph
            ));
        }
        report
    }

    fn show_process_tree(&mut self, hwnd: HWND) {
        let indices = self.selected_process_indices();
        if indices.is_empty() {
            self.status = "Select a process to inspect its parent and children.".to_string();
            return;
        }
        let by_pid: HashMap<u32, usize> = self
            .processes
            .iter()
            .enumerate()
            .map(|(index, process)| (process.pid, index))
            .collect();
        let mut report = String::from("PROCESS FAMILY TREE\r\n\r\n");
        for selected in indices.into_iter().take(20) {
            let process = &self.processes[selected];
            let mut chain = vec![selected];
            let mut parent = process.parent_pid;
            let mut seen = BTreeSet::from([process.pid]);
            while let Some(index) = by_pid.get(&parent).copied() {
                if !seen.insert(self.processes[index].pid) || chain.len() >= 16 {
                    break;
                }
                chain.push(index);
                parent = self.processes[index].parent_pid;
            }
            chain.reverse();
            for (depth, index) in chain.iter().enumerate() {
                let item = &self.processes[*index];
                let marker = if *index == selected {
                    " < SELECTED"
                } else {
                    ""
                };
                report.push_str(&format!(
                    "{}{} (PID {}, PPID {}){}\r\n",
                    "  ".repeat(depth),
                    item.name,
                    item.pid,
                    item.parent_pid,
                    marker
                ));
            }
            let children: Vec<_> = self
                .processes
                .iter()
                .filter(|item| item.parent_pid == process.pid)
                .collect();
            if children.is_empty() {
                report.push_str("  No active child processes.\r\n");
            } else {
                report.push_str("  Direct children:\r\n");
                for child in children.iter().take(30) {
                    report.push_str(&format!(
                        "    {} (PID {}) | CPU {:.1}% | RAM {}\r\n",
                        child.name,
                        child.pid,
                        child.cpu_percent,
                        format_memory(child.memory_kb)
                    ));
                }
            }
            report.push_str("\r\n");
        }
        self.open_monitor_report(hwnd, "Process Family Tree", report);
    }

    fn show_network_connections(&mut self, hwnd: HWND) {
        let indices = self.selected_process_indices();
        if indices.is_empty() {
            self.status = "Select processes before opening the network inspector.".to_string();
            return;
        }
        let ids = indices
            .iter()
            .map(|index| self.processes[*index].pid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let script = format!(
            "$ids=@({}); $tcp=Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object {{ $ids -contains $_.OwningProcess }} | Select-Object OwningProcess,State,LocalAddress,LocalPort,RemoteAddress,RemotePort; $udp=Get-NetUDPEndpoint -ErrorAction SilentlyContinue | Where-Object {{ $ids -contains $_.OwningProcess }} | Select-Object OwningProcess,LocalAddress,LocalPort; 'TCP CONNECTIONS'; $tcp | Format-Table -AutoSize | Out-String -Width 220; 'UDP ENDPOINTS'; $udp | Format-Table -AutoSize | Out-String -Width 220",
            ids
        );
        let report = powershell_output(&script).unwrap_or_else(|| {
            "Windows could not query network endpoints. Admin mode may be required.".to_string()
        });
        self.open_monitor_report(hwnd, "Network Connections", report);
    }

    fn show_signature_report(&mut self, hwnd: HWND) {
        let mut paths = BTreeSet::new();
        for index in self.selected_process_indices() {
            let path = self.processes[index].path.trim();
            if !path.is_empty() {
                paths.insert(path.to_string());
            }
        }
        if paths.is_empty() {
            self.status = "No readable executable path is available for the selection.".to_string();
            return;
        }
        let array = paths
            .iter()
            .take(20)
            .map(|path| format!("'{}'", path.replace("'", "''")))
            .collect::<Vec<_>>()
            .join(",");
        let script = format!(
            "$paths=@({}); $results=foreach($p in $paths){{ $s=Get-AuthenticodeSignature -LiteralPath $p; [pscustomobject]@{{Path=$p;Status=$s.Status;Signer=if($s.SignerCertificate){{$s.SignerCertificate.Subject}}else{{'Unsigned'}};Message=$s.StatusMessage}} }}; $results | Format-List | Out-String -Width 220",
            array
        );
        let report = powershell_output(&script)
            .unwrap_or_else(|| "Digital signature verification failed.".to_string());
        self.open_monitor_report(hwnd, "Digital Signature Verification", report);
    }

    fn show_executable_details(&mut self, hwnd: HWND) {
        let indices = self.selected_process_indices();
        if indices.is_empty() {
            self.status = "Select processes before opening executable details.".to_string();
            return;
        }
        let ids = indices
            .iter()
            .take(20)
            .map(|index| self.processes[*index].pid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let script = r#"
$ids=@($IDS)
function Get-PeArchitecture($path) {
  try {
    $bytes=[IO.File]::ReadAllBytes($path)
    $offset=[BitConverter]::ToInt32($bytes,60)
    $machine=[BitConverter]::ToUInt16($bytes,$offset+4)
    switch($machine){34404{'x64'};332{'x86'};43620{'ARM64'};default{"Machine $machine"}}
  } catch {'Unavailable'}
}
foreach($id in $ids) {
  $p=Get-CimInstance Win32_Process -Filter "ProcessId=$id" -ErrorAction SilentlyContinue
  if(!$p){continue}
  $owner=Invoke-CimMethod -InputObject $p -MethodName GetOwner -ErrorAction SilentlyContinue
  $item=Get-Item -LiteralPath $p.ExecutablePath -ErrorAction SilentlyContinue
  $hash=Get-FileHash -LiteralPath $p.ExecutablePath -Algorithm SHA256 -ErrorAction SilentlyContinue
  $sig=Get-AuthenticodeSignature -LiteralPath $p.ExecutablePath -ErrorAction SilentlyContinue
  [pscustomobject]@{
    Name=$p.Name; PID=$p.ProcessId; ParentPID=$p.ParentProcessId
    Owner=if($owner.User){"$($owner.Domain)\$($owner.User)"}else{'Unavailable'}
    CommandLine=$p.CommandLine; WorkingSet=$p.WorkingSetSize; Handles=$p.HandleCount
    Threads=$p.ThreadCount; Created=$p.CreationDate; Path=$p.ExecutablePath
    Architecture=Get-PeArchitecture $p.ExecutablePath
    FileVersion=$item.VersionInfo.FileVersion; ProductVersion=$item.VersionInfo.ProductVersion
    FileCreated=$item.CreationTime; FileModified=$item.LastWriteTime; FileBytes=$item.Length
    SHA256=$hash.Hash; Signature=$sig.Status
    Signer=if($sig.SignerCertificate){$sig.SignerCertificate.Subject}else{'Unsigned'}
  } | Format-List
}
"#
        .replace("$IDS", &ids);
        let report = powershell_output(&script).unwrap_or_else(|| {
            "Windows could not collect extended executable details.".to_string()
        });
        self.open_monitor_report(hwnd, "Full Executable Details", report);
    }

    fn toggle_selected_rule(&mut self, hwnd: HWND, kind: RuleKind) {
        let keys = self.selected_rule_keys();
        if keys.is_empty() {
            self.status = "Select one or more process rows first.".to_string();
            return;
        }
        let target = match kind {
            RuleKind::Watch => &mut self.watchlist,
            RuleKind::Alert => &mut self.alert_rules,
            RuleKind::Automation => &mut self.automation_rules,
        };
        let enable = keys.iter().any(|key| !target.contains(key));
        for key in &keys {
            if enable {
                target.insert(key.clone());
            } else {
                target.remove(key);
            }
        }
        save_guard_settings(
            &self.watchlist,
            &self.alert_rules,
            &self.automation_rules,
            &self.sentinel_config,
        );
        if enable && !self.auto_refresh {
            self.auto_refresh = true;
            unsafe {
                SetTimer(hwnd, TIMER_AUTO_REFRESH, 5000, None);
            }
        }
        let name = match kind {
            RuleKind::Watch => "watchlist",
            RuleKind::Alert => "resource alert (CPU 80%, RAM 1 GB, or disk 50 MB/s)",
            RuleKind::Automation => "automatic efficiency rule",
        };
        self.status = format!(
            "{} {} for {} app key(s).",
            if enable { "Enabled" } else { "Disabled" },
            name,
            keys.len()
        );
    }

    fn apply_monitoring_rules(&mut self) -> Option<String> {
        let mut current_counts: HashMap<String, usize> = HashMap::new();
        for process in &self.processes {
            *current_counts
                .entry(process_group_key(process))
                .or_default() += 1;
        }
        let mut events = Vec::new();
        if !self.last_watch_counts.is_empty() {
            for key in &self.watchlist {
                let before = self.last_watch_counts.get(key).copied().unwrap_or(0);
                let now = current_counts.get(key).copied().unwrap_or(0);
                if now > before {
                    events.push(format!(
                        "watchlist started +{}: {}",
                        now - before,
                        short_rule_key(key)
                    ));
                } else if before > now {
                    events.push(format!(
                        "watchlist stopped -{}: {}",
                        before - now,
                        short_rule_key(key)
                    ));
                }
            }
        }
        self.last_watch_counts = current_counts;

        let mut still_alerting = BTreeSet::new();
        for process in &self.processes {
            let key = process_group_key(process);
            let over_limit = process.cpu_percent >= 80.0
                || process.memory_kb >= 1_048_576
                || process.io_rate_kbps >= 51_200.0;
            if self.alert_rules.contains(&key) && over_limit {
                still_alerting.insert(process.pid);
                if !self.active_alerts.contains(&process.pid) {
                    events.push(format!(
                        "resource alert: {} PID {} | CPU {:.1}% | RAM {} | disk {}",
                        process.name,
                        process.pid,
                        process.cpu_percent,
                        format_memory(process.memory_kb),
                        format_rate(process.io_rate_kbps)
                    ));
                }
            }
            if self.automation_rules.contains(&key)
                && over_limit
                && process.safety == SafetyLevel::Safe
            {
                unsafe {
                    set_process_priority(process.pid, BELOW_NORMAL_PRIORITY_CLASS);
                }
            }
        }
        self.active_alerts = still_alerting;
        if events.is_empty() {
            None
        } else {
            Some(events.join(" | "))
        }
    }

    fn set_selected_priority(&mut self, class: u32, label: &str) {
        let indices = self.selected_process_indices();
        let mut changed = 0;
        let mut skipped = 0;
        for index in indices {
            let process = &self.processes[index];
            if process.safety == SafetyLevel::Blocked {
                skipped += 1;
                continue;
            }
            if unsafe { set_process_priority(process.pid, class) } {
                changed += 1;
            } else {
                skipped += 1;
            }
        }
        self.status = format!(
            "{} applied to {} process(es); skipped/failed {}.",
            label, changed, skipped
        );
    }

    fn limit_selected_affinity(&mut self) {
        let indices = self.selected_process_indices();
        let mut changed = 0;
        let mut skipped = 0;
        for index in indices {
            let process = &self.processes[index];
            if process.safety == SafetyLevel::Blocked {
                skipped += 1;
                continue;
            }
            if unsafe { limit_process_affinity(process.pid) } {
                changed += 1;
            } else {
                skipped += 1;
            }
        }
        self.status = format!(
            "Limited CPU affinity to the first half of available cores for {} process(es); skipped/failed {}.",
            changed, skipped
        );
    }

    fn suspend_or_resume_selected(&mut self, hwnd: HWND, resume: bool) {
        let indices = self.selected_process_indices();
        if indices.is_empty() {
            self.status = "Select process rows first.".to_string();
            return;
        }
        if !resume {
            let confirmed = unsafe {
                ask_yes_no(
                    hwnd,
                    "Suspend Processes",
                    "Suspend selected Safe processes? They will stop responding until resumed.",
                )
            };
            if !confirmed {
                return;
            }
        }
        let mut changed = 0;
        let mut skipped = 0;
        for index in indices {
            let process = &self.processes[index];
            if !resume && process.safety != SafetyLevel::Safe {
                skipped += 1;
                continue;
            }
            if unsafe { suspend_or_resume_process(process.pid, resume) } {
                changed += 1;
            } else {
                skipped += 1;
            }
        }
        self.status = format!(
            "{} {} process(es); skipped/failed {}.",
            if resume { "Resumed" } else { "Suspended" },
            changed,
            skipped
        );
    }

    fn open_windows_manager(&mut self, hwnd: HWND, startup: bool) {
        let target = if startup {
            "ms-settings:startupapps"
        } else {
            "services.msc"
        };
        let opened = unsafe { shell_open(hwnd, target) };
        self.status = if opened {
            if startup {
                "Opened Windows Startup Apps, where entries can be enabled or disabled.".to_string()
            } else {
                "Opened Windows Services, where services can be started, stopped, or configured."
                    .to_string()
            }
        } else {
            "Windows could not open the requested manager.".to_string()
        };
    }

    fn sentinel_security_report(&mut self, hwnd: HWND) {
        if self.selected.is_empty() {
            self.status =
                "Select one or more processes for a Sentinel security report.".to_string();
            return;
        }
        if self.ai_running {
            self.status = "Sentinel is already answering another request.".to_string();
            return;
        }
        let details = self.build_selection_details();
        let title = format!("Security report - {}", self.selected_ai_title());
        let messages = vec![AiMessage::new(
            SPEAKER_LOCAL,
            format!(
                "Security scan context ready.\r\n\r\n{}",
                truncate_text(&details, 5000)
            ),
        )];
        self.create_ai_session(title, details.clone(), messages);
        let prompt = format!(
            "{}\nCreate a combined security report for every selected process. Include identity, trust signals, resource behavior, parent relationships, concrete risks, safe actions, and a final severity-ranked recommendation. Never claim malware without evidence.\n\nPROCESS DATA:\n{}",
            ai_system_instruction(),
            details
        );
        self.start_ai_request(
            hwnd,
            prompt,
            "Sentinel is creating the combined security report...",
        );
        unsafe {
            self.ensure_ai_window(hwnd);
        }
    }

    fn save_snapshot(&mut self, hwnd: HWND) {
        let mut out = String::from("PROCESS_GUARD_SNAPSHOT_V1\n");
        for process in &self.processes {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                process.pid,
                escape_snapshot_field(&process.name),
                process.memory_kb,
                escape_snapshot_field(&process.path)
            ));
        }
        let path = snapshot_path();
        match std::fs::write(&path, out) {
            Ok(()) => {
                self.status = format!(
                    "Saved snapshot with {} processes to {}.",
                    self.processes.len(),
                    path.display()
                );
            }
            Err(error) => unsafe {
                show_message(
                    hwnd,
                    "Snapshot Failed",
                    &error.to_string(),
                    MB_OK | MB_ICONERROR,
                );
            },
        }
    }

    fn compare_snapshot(&mut self, hwnd: HWND) {
        let path = snapshot_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            self.status = "No saved snapshot exists yet. Use Monitor > Save System Snapshot first."
                .to_string();
            return;
        };
        let mut old: BTreeMap<String, (usize, u64, String)> = BTreeMap::new();
        for line in text.lines().skip(1) {
            let fields: Vec<_> = line.splitn(4, '\t').collect();
            if fields.len() != 4 {
                continue;
            }
            let name = fields[1].to_string();
            let memory = fields[2].parse::<u64>().unwrap_or(0);
            let path = fields[3].to_string();
            let key = format!(
                "{}|{}",
                name.to_ascii_lowercase(),
                path.to_ascii_lowercase()
            );
            let entry = old.entry(key).or_insert((0, 0, name));
            entry.0 += 1;
            entry.1 += memory;
        }
        let mut current: BTreeMap<String, (usize, u64, String)> = BTreeMap::new();
        for process in &self.processes {
            let key = format!(
                "{}|{}",
                process.name.to_ascii_lowercase(),
                process.path.to_ascii_lowercase()
            );
            let entry = current.entry(key).or_insert((0, 0, process.name.clone()));
            entry.0 += 1;
            entry.1 += process.memory_kb;
        }
        let mut report = String::from("SYSTEM SNAPSHOT COMPARISON\r\n\r\nNEW APPS / PROCESSES\r\n");
        let mut changes = 0;
        for (key, (count, memory, name)) in &current {
            if !old.contains_key(key) {
                changes += 1;
                report.push_str(&format!(
                    "+ {} | {} process(es) | {}\r\n",
                    name,
                    count,
                    format_memory(*memory)
                ));
            }
        }
        report.push_str("\r\nSTOPPED OR MISSING\r\n");
        for (key, (count, memory, name)) in &old {
            if !current.contains_key(key) {
                changes += 1;
                report.push_str(&format!(
                    "- {} | {} process(es) | {}\r\n",
                    name,
                    count,
                    format_memory(*memory)
                ));
            }
        }
        report.push_str("\r\nCHANGED COUNTS OR MEMORY\r\n");
        for (key, (count, memory, name)) in &current {
            if let Some((old_count, old_memory, _)) = old.get(key) {
                let memory_delta = (*memory as i128 - *old_memory as i128).unsigned_abs() as u64;
                if count != old_count || memory_delta >= 10_240 {
                    changes += 1;
                    report.push_str(&format!(
                        "* {} | count {} -> {} | RAM {} -> {}\r\n",
                        name,
                        old_count,
                        count,
                        format_memory(*old_memory),
                        format_memory(*memory)
                    ));
                }
            }
        }
        if changes == 0 {
            report.push_str("No meaningful changes found.\r\n");
        }
        self.open_monitor_report(hwnd, "Snapshot Comparison", report);
    }

    fn run_action(&mut self, hwnd: HWND, action: Action) {
        match action {
            Action::Refresh => self.refresh(),
            Action::EndSafe => self.end_safe(hwnd),
            Action::Explain => {
                self.explain(hwnd);
                unsafe {
                    self.ensure_ai_window(hwnd);
                }
            }
            Action::History => {
                self.open_ai_history();
                unsafe {
                    self.ensure_ai_window(hwnd);
                }
            }
            Action::ToggleView => {
                self.grouped_view = !self.grouped_view;
                self.apply_filters();
                self.status = self.status_base();
            }
            Action::SelectSafe => {
                self.selected.clear();
                for row in &self.visible_rows {
                    if self.row_has_safe_target(*row) {
                        self.selected.insert(self.row_id(*row));
                    }
                }
                self.details = self.build_selection_details();
                self.status = format!("Selected {} safe row(s).", self.selected.len());
            }
            Action::Export => self.export_report(hwnd),
            Action::AutoRefresh => {
                self.auto_refresh = !self.auto_refresh;
                unsafe {
                    if self.auto_refresh {
                        SetTimer(hwnd, TIMER_AUTO_REFRESH, 5000, None);
                    } else {
                        KillTimer(hwnd, TIMER_AUTO_REFRESH);
                    }
                }
                self.status = if self.auto_refresh {
                    "Auto-refresh enabled: every 5 seconds.".to_string()
                } else {
                    "Auto-refresh disabled.".to_string()
                };
            }
            Action::Admin => self.toggle_admin_mode(hwnd),
        }
    }

    fn toggle_admin_mode(&mut self, hwnd: HWND) {
        self.elevated = unsafe { is_process_elevated() };
        let launched = unsafe {
            if self.elevated {
                launch_unelevated_copy(hwnd)
            } else {
                launch_elevated_copy(hwnd)
            }
        };

        if launched {
            self.status = if self.elevated {
                "Opened normal mode. Closing this admin instance...".to_string()
            } else {
                "Opened admin mode. Closing this normal instance...".to_string()
            };
            unsafe {
                DestroyWindow(hwnd);
            }
        } else {
            self.status = if self.elevated {
                "Windows did not start the normal-mode copy.".to_string()
            } else {
                "Windows did not start the elevated copy.".to_string()
            };
            unsafe {
                show_message(hwnd, "Admin Mode", &self.status, MB_OK | MB_ICONWARNING);
            }
        }
    }

    fn run_menu_command(&mut self, hwnd: HWND, command: MenuCommand) {
        match command {
            MenuCommand::Action(action) => self.run_action(hwnd, action),
            MenuCommand::FocusSearch => {
                self.search_focus = true;
                self.status =
                    "Search box focused. Type to filter process name, PID, type, or path."
                        .to_string();
            }
            MenuCommand::ClearFilters => {
                self.search.clear();
                self.field_filter = None;
                self.apply_filters();
                self.status = "Search and filters cleared.".to_string();
            }
            MenuCommand::StartProcess => {
                self.launcher_open = true;
                self.launcher_focus = true;
                self.search_focus = false;
                self.status = "Start Process opened.".to_string();
            }
            MenuCommand::MinimizeTray => self.minimize_to_tray(hwnd),
            MenuCommand::Exit => unsafe {
                DestroyWindow(hwnd);
            },
            MenuCommand::ExpandSelected => {
                self.expand_selected_groups();
                self.status = "Selected groups expanded.".to_string();
            }
            MenuCommand::CollapseAll => {
                self.expanded_groups.clear();
                self.apply_filters();
                self.status = "All groups collapsed.".to_string();
            }
            MenuCommand::ClearSelection => {
                self.selected.clear();
                self.details = self.build_selection_details();
                self.status = "Selection cleared.".to_string();
            }
            MenuCommand::OpenLocation => self.open_selected_location(hwnd),
            MenuCommand::FilterSelectedType => {
                if let Some(value) = self.selected_type() {
                    self.field_filter = Some(FieldFilter::Type(value.clone()));
                    self.search.clear();
                    self.apply_filters();
                    self.status = format!("Filtered to type: {}", value);
                } else {
                    self.status = "Select a row first to filter by type.".to_string();
                }
            }
            MenuCommand::LivePerformance => self.show_live_performance(hwnd),
            MenuCommand::ProcessTree => self.show_process_tree(hwnd),
            MenuCommand::NetworkInspector => self.show_network_connections(hwnd),
            MenuCommand::VerifySignature => self.show_signature_report(hwnd),
            MenuCommand::FullExecutableDetails => self.show_executable_details(hwnd),
            MenuCommand::ToggleAlerts => self.toggle_selected_rule(hwnd, RuleKind::Alert),
            MenuCommand::ToggleWatchlist => self.toggle_selected_rule(hwnd, RuleKind::Watch),
            MenuCommand::SaveSnapshot => self.save_snapshot(hwnd),
            MenuCommand::CompareSnapshot => self.compare_snapshot(hwnd),
            MenuCommand::PriorityHigh => {
                self.set_selected_priority(HIGH_PRIORITY_CLASS, "High priority")
            }
            MenuCommand::PriorityNormal => {
                self.set_selected_priority(NORMAL_PRIORITY_CLASS, "Normal priority")
            }
            MenuCommand::EfficiencyMode => {
                self.set_selected_priority(BELOW_NORMAL_PRIORITY_CLASS, "Efficiency mode")
            }
            MenuCommand::LimitAffinity => self.limit_selected_affinity(),
            MenuCommand::SuspendSelected => self.suspend_or_resume_selected(hwnd, false),
            MenuCommand::ResumeSelected => self.suspend_or_resume_selected(hwnd, true),
            MenuCommand::ToggleAutomation => self.toggle_selected_rule(hwnd, RuleKind::Automation),
            MenuCommand::StartupManager => self.open_windows_manager(hwnd, true),
            MenuCommand::ServicesManager => self.open_windows_manager(hwnd, false),
            MenuCommand::SentinelSecurityReport => self.sentinel_security_report(hwnd),
            MenuCommand::SentinelSettings => self.open_sentinel_settings(),
            MenuCommand::About => unsafe {
                show_message(
                    hwnd,
                    "About Process Guard",
                    &format!(
                        "Process Guard {} is an open-source native Rust process manager with grouped safety labels, live monitoring, safe-ending controls, and configurable Sentinel AI providers.",
                        APP_VERSION
                    ),
                    MB_OK | MB_ICONINFORMATION,
                );
            },
        }
    }

    fn open_selected_location(&mut self, hwnd: HWND) {
        let Some(path) = self.primary_selected_path() else {
            self.status = "Select a process row first to open its file location.".to_string();
            return;
        };
        if path.trim().is_empty() || !std::path::Path::new(&path).exists() {
            self.status = "This process path is not available or no longer exists.".to_string();
            return;
        }
        let opened = unsafe { open_file_location(hwnd, &path) };
        self.status = if opened {
            format!("Opened file location: {}", path)
        } else {
            "Windows could not open that file location.".to_string()
        };
    }

    fn primary_selected_path(&self) -> Option<String> {
        for row in &self.visible_rows {
            if !self.selected.contains(&self.row_id(*row)) {
                continue;
            }
            return match *row {
                RowKind::Group(index) => Some(self.groups[index].path.clone()),
                RowKind::Process(index) => Some(self.processes[index].path.clone()),
            };
        }
        None
    }

    fn minimize_to_tray(&mut self, hwnd: HWND) {
        unsafe {
            if !self.tray_visible {
                self.tray_visible = add_tray_icon(hwnd);
            }
            if self.tray_visible {
                ShowWindow(hwnd, SW_HIDE);
                self.status = "Process Guard is running in the system tray.".to_string();
            }
        }
    }

    fn restore_from_tray(&mut self, hwnd: HWND) {
        unsafe {
            remove_tray_icon(hwnd);
            self.tray_visible = false;
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
            self.status = "Process Guard restored from the system tray.".to_string();
        }
    }

    fn change_ai_popup_size(&mut self, client: RECT, dw: i32, dh: i32) {
        if !self.ai_hwnd.is_null() {
            unsafe {
                let mut window: RECT = zeroed();
                if GetWindowRect(self.ai_hwnd, &mut window) != 0 {
                    let width = (window.right - window.left + dw).clamp(620, 1320);
                    let height = (window.bottom - window.top + dh).clamp(420, 860);
                    SetWindowPos(
                        self.ai_hwnd,
                        null_mut(),
                        0,
                        0,
                        width,
                        height,
                        SWP_NOMOVE | SWP_NOZORDER,
                    );
                    InvalidateRect(self.ai_hwnd, null(), 0);
                }
            }
            return;
        }
        let ai = self.ai_layout(client);
        self.ai_popup_size = Some((
            ai.popup.right - ai.popup.left + dw,
            ai.popup.bottom - ai.popup.top + dh,
        ));
        let ai = self.ai_layout(client);
        self.ai_popup_pos = Some((ai.popup.left, ai.popup.top));
        self.ai_popup_size = Some((
            ai.popup.right - ai.popup.left,
            ai.popup.bottom - ai.popup.top,
        ));
    }

    unsafe fn fit_ai_window(&mut self) {
        if self.ai_hwnd.is_null() {
            return;
        }
        let sw = GetSystemMetrics(SM_CXSCREEN).max(760);
        let sh = GetSystemMetrics(SM_CYSCREEN).max(520);
        let width = 980.min((sw - 80).max(620));
        let height = 640.min((sh - 90).max(420));
        SetWindowPos(
            self.ai_hwnd,
            null_mut(),
            ((sw - width) / 2).max(20),
            ((sh - height) / 2).max(20),
            width,
            height,
            SWP_NOZORDER,
        );
        InvalidateRect(self.ai_hwnd, null(), 0);
    }

    fn handle_header(&mut self, column: Column) {
        match column {
            Column::Name => self.sort_mode = SortMode::Name,
            Column::Cpu => self.sort_mode = SortMode::Cpu,
            Column::Memory => self.sort_mode = SortMode::Memory,
            Column::Io => self.sort_mode = SortMode::Io,
            Column::Network => self.sort_mode = SortMode::Network,
            Column::Threads => self.sort_mode = SortMode::Threads,
            Column::Handles => self.sort_mode = SortMode::Handles,
            Column::Risk => self.sort_mode = SortMode::Risk,
            Column::Type => {
                let value = self.selected_type().or_else(|| self.next_type_filter());
                if let Some(value) = value {
                    self.field_filter = Some(FieldFilter::Type(value));
                    self.search.clear();
                }
            }
            Column::Safety => {
                self.field_filter = Some(FieldFilter::Safety(self.next_safety_filter()));
                self.search.clear();
            }
            Column::Items => {
                self.grouped_view = true;
            }
            Column::Reason => {
                self.field_filter = Some(FieldFilter::SafeOnly);
                self.search.clear();
            }
        }
        self.apply_filters();
        self.status = self.status_base();
    }

    fn toggle_group(&mut self, row: RowKind) {
        if let RowKind::Group(index) = row {
            let key = self.groups[index].key.clone();
            if self.expanded_groups.contains(&key) {
                self.expanded_groups.remove(&key);
            } else {
                self.expanded_groups.insert(key);
            }
            let anchor = self.row_id(row);
            self.apply_filters();
            if let Some(pos) = self
                .visible_rows
                .iter()
                .position(|r| self.row_id(*r) == anchor)
            {
                self.scroll = self.scroll.min(pos);
            }
            self.status = "Group expanded/collapsed without rescanning.".to_string();
        }
    }

    fn expand_selected_groups(&mut self) {
        let selected = self.selected.clone();
        for row in &self.visible_rows {
            if selected.contains(&self.row_id(*row)) {
                if let RowKind::Group(index) = *row {
                    self.expanded_groups.insert(self.groups[index].key.clone());
                }
            }
        }
        self.apply_filters();
    }

    fn end_safe(&mut self, hwnd: HWND) {
        let (targets, skipped) = self.targets_from_selection();
        if targets.is_empty() {
            unsafe {
                show_message(
                    hwnd,
                    "Nothing Safe Selected",
                    "Select one or more Safe rows first. Blocked, caution, and unknown rows are never killed by bulk action.",
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            return;
        }

        let confirm = format!(
            "Force end {} safe process(es)?\r\nSkipped non-safe: {}\r\n\r\nUnsaved work can be lost.",
            targets.len(),
            skipped.len()
        );
        let yes = unsafe { ask_yes_no(hwnd, "End Safe Processes", &confirm) };
        if !yes {
            return;
        }

        let mut ended = 0usize;
        let mut failed = Vec::new();
        for (pid, name) in targets {
            match unsafe { terminate_pid(pid) } {
                Ok(()) => ended += 1,
                Err(error) => failed.push(format!("{} ({}) error {}", name, pid, error)),
            }
        }
        self.status = format!("Ended {} process(es). Failed: {}", ended, failed.len());
        self.refresh();
    }

    fn explain(&mut self, hwnd: HWND) {
        if self.ai_running {
            self.status =
                "Sentinel is still answering. Wait before starting a new chat.".to_string();
            self.ai_overlay = true;
            return;
        }
        let details = self.build_selection_details();
        if self.selected.is_empty() {
            if let Some(index) = self.ordered_ai_session_indices().first().copied() {
                self.load_ai_session(index);
                self.ai_overlay = true;
                return;
            }
            unsafe {
                show_message(
                    hwnd,
                    "Ask Sentinel",
                    "Select a row first, or create a Sentinel chat history by asking about a process.",
                    MB_OK | MB_ICONINFORMATION,
                );
            }
            return;
        }
        let title = self.selected_ai_title();
        self.details = details.clone();
        self.ai_overlay = true;
        self.ai_input_focus = true;
        let messages = vec![AiMessage::new(
            SPEAKER_LOCAL,
            format!(
                "Quick local analysis is ready now.\r\n\r\n{}",
                truncate_text(&details, 1800)
            ),
        )];
        self.create_ai_session(title, details.clone(), messages);
        let prompt = explain_prompt(&details);
        self.start_ai_request(
            hwnd,
            prompt,
            "Sentinel is analyzing. The popup already shows a quick local summary.",
        );
    }

    fn export_report(&self, hwnd: HWND) {
        let path = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("ProcessGuard_Report.txt");
        let mut out = String::from("Process Guard Report\r\n\r\n");
        for group in &self.groups {
            out.push_str(&format!(
                "{} | {} item(s) | {} | risk {} | {} | {} | {}\r\n",
                group.name,
                group.process_indices.len(),
                format_memory(group.total_memory_kb),
                group.max_risk_score,
                group_reason(group),
                display_or_dash(&group.company),
                display_or_dash(&group.product)
            ));
            for idx in &group.process_indices {
                let p = &self.processes[*idx];
                out.push_str(&format!(
                    "  PID {} | {} | {} | {} | {} | {} | {}\r\n",
                    p.pid,
                    p.safety.label(),
                    format_memory(p.memory_kb),
                    p.name,
                    display_or_dash(&p.description),
                    display_or_dash(&p.original_filename),
                    p.path
                ));
            }
        }
        let result = std::fs::write(&path, out);
        unsafe {
            match result {
                Ok(()) => show_message(
                    hwnd,
                    "Export Complete",
                    &format!("Saved {}", path.display()),
                    MB_OK | MB_ICONINFORMATION,
                ),
                Err(e) => show_message(hwnd, "Export Failed", &e.to_string(), MB_OK | MB_ICONERROR),
            }
        }
    }

    fn status_base(&self) -> String {
        let safe = self
            .processes
            .iter()
            .filter(|p| p.safety == SafetyLevel::Safe)
            .count();
        let caution = self
            .processes
            .iter()
            .filter(|p| p.safety == SafetyLevel::Caution)
            .count();
        let blocked = self
            .processes
            .iter()
            .filter(|p| p.safety == SafetyLevel::Blocked)
            .count();
        format!(
            "{} visible | {} processes | safe {} / caution {} / blocked {} | filter {}",
            self.visible_rows.len(),
            self.processes.len(),
            safe,
            caution,
            blocked,
            self.filter_label()
        )
    }

    fn ai_prompt_history(&self) -> String {
        let mut text = String::new();
        for message in self
            .ai_messages
            .iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .iter()
            .rev()
        {
            text.push_str(message.speaker);
            text.push_str(": ");
            text.push_str(message.text.trim());
            text.push_str("\n\n");
        }
        truncate_text(&text, 5000)
    }

    fn visible_table_rows_for_client(&self, client: RECT) -> usize {
        let layout = self.layout(client);
        let rows_top = layout.table.top + HEADER_H;
        ((layout.table.bottom - HSCROLL_H - rows_top) / ROW_H).max(0) as usize
    }

    fn max_scroll_for_client(&self, client: RECT) -> usize {
        self.visible_rows
            .len()
            .saturating_sub(self.visible_table_rows_for_client(client).max(1))
    }

    fn clamp_scroll_to_window(&mut self, hwnd: HWND) {
        let mut client: RECT = unsafe { zeroed() };
        unsafe {
            GetClientRect(hwnd, &mut client);
        }
        self.scroll = self.scroll.min(self.max_scroll_for_client(client));
        let table = self.layout(client).table;
        self.hscroll = self.hscroll.clamp(0, self.max_hscroll_for_table(table));
    }

    fn max_hscroll_for_table(&self, table: RECT) -> i32 {
        let visible_scrollable = (table.right - table.left - FROZEN_NAME_W).max(1);
        (table_content_width() - FROZEN_NAME_W - visible_scrollable).max(0)
    }

    fn ai_layout(&self, client: RECT) -> AiLayout {
        if !self.ai_hwnd.is_null() {
            return ai_window_layout(client);
        }
        ai_layout(client, self.ai_popup_pos, self.ai_popup_size)
    }

    fn selected_ai_title(&self) -> String {
        let mut names = Vec::new();
        for row in &self.visible_rows {
            if !self.selected.contains(&self.row_id(*row)) {
                continue;
            }
            match *row {
                RowKind::Group(index) => names.push(self.groups[index].name.clone()),
                RowKind::Process(index) => names.push(format!(
                    "{} ({})",
                    self.processes[index].name, self.processes[index].pid
                )),
            }
            if names.len() >= 2 {
                break;
            }
        }
        if names.is_empty() {
            "Selected process".to_string()
        } else if self.selected.len() > names.len() {
            format!(
                "{} +{} more",
                names.join(", "),
                self.selected.len() - names.len()
            )
        } else {
            names.join(", ")
        }
    }

    fn build_selection_details(&self) -> String {
        if self.selected.is_empty() {
            return "No row selected.\r\n\r\nClick a group or process. Double-click a group to expand. Header clicks sort/filter.".to_string();
        }

        let mut lines = Vec::new();
        let mut memory = 0u64;
        let mut safe_targets = 0usize;
        for row in &self.visible_rows {
            if !self.selected.contains(&self.row_id(*row)) {
                continue;
            }
            match *row {
                RowKind::Group(i) => {
                    let g = &self.groups[i];
                    memory += g.total_memory_kb;
                    safe_targets += g.safe_count;
                    let watch = self.watchlist.contains(&g.key);
                    let alert = self.alert_rules.contains(&g.key);
                    let automatic = self.automation_rules.contains(&g.key);
                    lines.push(format!(
                        "GROUP {}\r\nItems: {} | CPU: {:.1}% | RAM: {} | Disk: {} | Net: {} | Threads: {} | Handles: {} | Risk: {}%\r\nRules: watch {} | alert {} | automatic {}\r\nSafety: {}\r\nCompany: {}\r\nProduct: {}\r\nDescription: {}\r\nPath: {}\r\n{}",
                        g.name,
                        g.process_indices.len(),
                        g.total_cpu_percent,
                        format_memory(g.total_memory_kb),
                        format_rate(g.total_io_rate_kbps),
                        g.total_network_connections,
                        g.total_threads,
                        g.total_handles,
                        g.max_risk_score,
                        if watch { "on" } else { "off" },
                        if alert { "on" } else { "off" },
                        if automatic { "on" } else { "off" },
                        group_reason(g),
                        display_or_dash(&g.company),
                        display_or_dash(&g.product),
                        display_or_dash(&g.description),
                        display_or_dash(&g.path),
                        group_explanation(g)
                    ));
                }
                RowKind::Process(i) => {
                    let p = &self.processes[i];
                    memory += p.memory_kb;
                    if p.safety.can_end() {
                        safe_targets += 1;
                    }
                    let key = process_group_key(p);
                    let graph = self
                        .performance_history
                        .get(&p.pid)
                        .map(performance_graph)
                        .unwrap_or_else(|| ".".to_string());
                    lines.push(format!(
                        "PROCESS {}\r\nPID: {} | PPID: {} | CPU: {:.1}% | RAM: {} | Disk: {} | Net: {} | Threads: {} | Handles: {} | Risk: {}%\r\nCPU history: {}\r\nRules: watch {} | alert {} | automatic {}\r\nType: {} | Safety: {}\r\nCompany: {}\r\nProduct: {}\r\nDescription: {}\r\nOriginal file: {}\r\nPath: {}\r\n{}",
                        p.name,
                        p.pid,
                        p.parent_pid,
                        p.cpu_percent,
                        format_memory(p.memory_kb),
                        format_rate(p.io_rate_kbps),
                        p.network_connections,
                        p.thread_count,
                        p.handle_count,
                        p.risk_score,
                        graph,
                        if self.watchlist.contains(&key) { "on" } else { "off" },
                        if self.alert_rules.contains(&key) { "on" } else { "off" },
                        if self.automation_rules.contains(&key) { "on" } else { "off" },
                        p.category,
                        p.safety.label(),
                        display_or_dash(&p.company),
                        display_or_dash(&p.product),
                        display_or_dash(&p.description),
                        display_or_dash(&p.original_filename),
                        display_or_dash(&p.path),
                        process_explanation(p)
                    ));
                }
            }
        }

        format!(
            "Selected: {}\r\nSafe process targets: {}\r\nSelected RAM: {}\r\n\r\n{}",
            self.selected.len(),
            safe_targets,
            format_memory(memory),
            lines.join("\r\n\r\n")
        )
    }

    fn row_cells(&self, row: RowKind) -> Vec<String> {
        match row {
            RowKind::Group(i) => {
                let g = &self.groups[i];
                let icon = if self.expanded_groups.contains(&g.key) {
                    "[-]"
                } else {
                    "[+]"
                };
                vec![
                    format!("{} {}", icon, g.name),
                    g.process_indices.len().to_string(),
                    display_or_dash(&g.category),
                    format!("{:.1}%", g.total_cpu_percent),
                    format_memory(g.total_memory_kb),
                    format_rate(g.total_io_rate_kbps),
                    g.total_network_connections.to_string(),
                    g.total_threads.to_string(),
                    g.total_handles.to_string(),
                    format!("{}%", g.max_risk_score),
                    group_safety_label(g).to_string(),
                    group_reason(g),
                ]
            }
            RowKind::Process(i) => {
                let p = &self.processes[i];
                vec![
                    format!("  {}", p.name),
                    format!("PID {}", p.pid),
                    display_or_dash(&p.category),
                    format!("{:.1}%", p.cpu_percent),
                    format_memory(p.memory_kb),
                    format_rate(p.io_rate_kbps),
                    p.network_connections.to_string(),
                    p.thread_count.to_string(),
                    p.handle_count.to_string(),
                    format!("{}%", p.risk_score),
                    p.safety.label().to_string(),
                    p.reason.clone(),
                ]
            }
        }
    }

    fn row_colors(&self, row: RowKind, selected: bool, screen_row: usize) -> (COLORREF, COLORREF) {
        if selected {
            return (C_SELECT_TEXT, C_SELECT_BG);
        }
        match row {
            RowKind::Group(i) => {
                let group = &self.groups[i];
                if group.blocked_count > 0 || group.unknown_count > 0 {
                    (C_BLOCK_TEXT, C_DANGER_BG)
                } else if group.caution_count > 0 {
                    (C_WARN_TEXT, C_CAUTION_BG)
                } else {
                    (
                        C_SAFE_TEXT,
                        if screen_row % 2 == 0 {
                            C_SAFE_BG_A
                        } else {
                            C_SAFE_BG_B
                        },
                    )
                }
            }
            RowKind::Process(i) => match self.processes[i].safety {
                SafetyLevel::Safe => (
                    C_SAFE_TEXT,
                    if screen_row % 2 == 0 {
                        C_SAFE_BG_A
                    } else {
                        C_SAFE_BG_B
                    },
                ),
                SafetyLevel::Caution => (C_WARN_TEXT, C_CAUTION_BG),
                SafetyLevel::Unknown => (C_BLOCK_TEXT, C_DANGER_BG),
                SafetyLevel::Blocked => (C_BLOCK_TEXT, C_BLOCK_BG),
            },
        }
    }

    fn row_id(&self, row: RowKind) -> String {
        match row {
            RowKind::Group(i) => format!("G:{}", self.groups[i].key),
            RowKind::Process(i) => format!("P:{}", self.processes[i].pid),
        }
    }

    fn row_has_safe_target(&self, row: RowKind) -> bool {
        match row {
            RowKind::Group(i) => self.groups[i].safe_count > 0,
            RowKind::Process(i) => self.processes[i].safety.can_end(),
        }
    }

    fn targets_from_selection(&self) -> (Vec<(u32, String)>, Vec<String>) {
        let mut seen = BTreeSet::new();
        let mut targets = Vec::new();
        let mut skipped = Vec::new();
        for row in &self.visible_rows {
            if !self.selected.contains(&self.row_id(*row)) {
                continue;
            }
            match *row {
                RowKind::Group(i) => {
                    for idx in &self.groups[i].process_indices {
                        self.collect_target(*idx, &mut seen, &mut targets, &mut skipped);
                    }
                }
                RowKind::Process(i) => {
                    self.collect_target(i, &mut seen, &mut targets, &mut skipped);
                }
            }
        }
        (targets, skipped)
    }

    fn collect_target(
        &self,
        idx: usize,
        seen: &mut BTreeSet<u32>,
        targets: &mut Vec<(u32, String)>,
        skipped: &mut Vec<String>,
    ) {
        let p = &self.processes[idx];
        if p.safety.can_end() {
            if seen.insert(p.pid) {
                targets.push((p.pid, p.name.clone()));
            }
        } else {
            skipped.push(format!("{} ({}) {}", p.name, p.pid, p.safety.label()));
        }
    }

    fn sorted_group_indices(&self) -> Vec<usize> {
        let mut v: Vec<_> = (0..self.groups.len()).collect();
        v.sort_by(|a, b| {
            let a = &self.groups[*a];
            let b = &self.groups[*b];
            match self.sort_mode {
                SortMode::Memory => b.total_memory_kb.cmp(&a.total_memory_kb),
                SortMode::Cpu => b.total_cpu_percent.total_cmp(&a.total_cpu_percent),
                SortMode::Io => b.total_io_rate_kbps.total_cmp(&a.total_io_rate_kbps),
                SortMode::Network => b
                    .total_network_connections
                    .cmp(&a.total_network_connections),
                SortMode::Threads => b.total_threads.cmp(&a.total_threads),
                SortMode::Handles => b.total_handles.cmp(&a.total_handles),
                SortMode::Risk => b.max_risk_score.cmp(&a.max_risk_score),
                SortMode::Name => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase()),
            }
        });
        v
    }

    fn sorted_process_indices(&self) -> Vec<usize> {
        let mut v: Vec<_> = (0..self.processes.len()).collect();
        v.sort_by(|a, b| {
            let a = &self.processes[*a];
            let b = &self.processes[*b];
            match self.sort_mode {
                SortMode::Memory => b.memory_kb.cmp(&a.memory_kb),
                SortMode::Cpu => b.cpu_percent.total_cmp(&a.cpu_percent),
                SortMode::Io => b.io_rate_kbps.total_cmp(&a.io_rate_kbps),
                SortMode::Network => b.network_connections.cmp(&a.network_connections),
                SortMode::Threads => b.thread_count.cmp(&a.thread_count),
                SortMode::Handles => b.handle_count.cmp(&a.handle_count),
                SortMode::Risk => b.risk_score.cmp(&a.risk_score),
                SortMode::Name => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase()),
            }
        });
        v
    }

    fn sorted_child_indices(&self, group: &ProcessGroup) -> Vec<usize> {
        let mut v = group.process_indices.clone();
        v.sort_by(|a, b| {
            self.processes[*b]
                .memory_kb
                .cmp(&self.processes[*a].memory_kb)
        });
        v
    }

    fn group_matches_filter(&self, group: &ProcessGroup) -> bool {
        let text = if self.search.is_empty() {
            true
        } else {
            let n = self.search.to_ascii_lowercase();
            group.name.to_ascii_lowercase().contains(&n)
                || group.category.to_ascii_lowercase().contains(&n)
                || group.company.to_ascii_lowercase().contains(&n)
                || group.path.to_ascii_lowercase().contains(&n)
        };
        text && match &self.field_filter {
            None => true,
            Some(FieldFilter::Type(t)) => group.category.eq_ignore_ascii_case(t),
            Some(FieldFilter::Safety(SafetyLevel::Safe)) => {
                group.safe_count == group.process_indices.len()
            }
            Some(FieldFilter::Safety(SafetyLevel::Caution)) => {
                group.caution_count == group.process_indices.len()
            }
            Some(FieldFilter::Safety(SafetyLevel::Unknown)) => {
                group.unknown_count == group.process_indices.len()
            }
            Some(FieldFilter::Safety(SafetyLevel::Blocked)) => {
                group.blocked_count == group.process_indices.len()
            }
            Some(FieldFilter::SafeOnly) => group.safe_count > 0,
        }
    }

    fn process_matches_filter(&self, p: &ProcessInfo) -> bool {
        let text = if self.search.is_empty() {
            true
        } else {
            let n = self.search.to_ascii_lowercase();
            p.name.to_ascii_lowercase().contains(&n)
                || p.category.to_ascii_lowercase().contains(&n)
                || p.company.to_ascii_lowercase().contains(&n)
                || p.path.to_ascii_lowercase().contains(&n)
                || p.pid.to_string().contains(&n)
        };
        text && match &self.field_filter {
            None => true,
            Some(FieldFilter::Type(t)) => p.category.eq_ignore_ascii_case(t),
            Some(FieldFilter::Safety(s)) => p.safety == *s,
            Some(FieldFilter::SafeOnly) => p.safety.can_end(),
        }
    }

    fn selected_type(&self) -> Option<String> {
        for row in &self.visible_rows {
            if self.selected.contains(&self.row_id(*row)) {
                return match *row {
                    RowKind::Group(i) => Some(self.groups[i].category.clone()),
                    RowKind::Process(i) => Some(self.processes[i].category.clone()),
                };
            }
        }
        None
    }

    fn next_type_filter(&self) -> Option<String> {
        let mut types = self
            .groups
            .iter()
            .map(|g| g.category.clone())
            .filter(|s| !s.trim().is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        types.sort();
        if types.is_empty() {
            None
        } else if let Some(FieldFilter::Type(current)) = &self.field_filter {
            let next = types
                .iter()
                .position(|t| t == current)
                .map(|i| (i + 1) % types.len())
                .unwrap_or(0);
            Some(types[next].clone())
        } else {
            Some(types[0].clone())
        }
    }

    fn next_safety_filter(&self) -> SafetyLevel {
        let order = [
            SafetyLevel::Safe,
            SafetyLevel::Caution,
            SafetyLevel::Unknown,
            SafetyLevel::Blocked,
        ];
        if let Some(FieldFilter::Safety(current)) = self.field_filter {
            let next = order
                .iter()
                .position(|s| *s == current)
                .map(|i| (i + 1) % order.len())
                .unwrap_or(0);
            order[next]
        } else {
            SafetyLevel::Safe
        }
    }

    fn filter_label(&self) -> String {
        if !self.search.is_empty() {
            return format!("search '{}'", self.search);
        }
        match &self.field_filter {
            None => "none".to_string(),
            Some(FieldFilter::Type(t)) => format!("type {}", t),
            Some(FieldFilter::Safety(s)) => format!("safety {}", s.label()),
            Some(FieldFilter::SafeOnly) => "safe only".to_string(),
        }
    }
}

impl Column {
    fn label(self) -> &'static str {
        match self {
            Column::Name => "Process / Group",
            Column::Items => "Items",
            Column::Type => "Type",
            Column::Cpu => "CPU",
            Column::Memory => "RAM",
            Column::Io => "Disk I/O",
            Column::Network => "Net",
            Column::Threads => "Threads",
            Column::Handles => "Handles",
            Column::Risk => "Risk",
            Column::Safety => "Safety",
            Column::Reason => "Reason",
        }
    }
}

fn enumerate_processes() -> Vec<ProcessInfo> {
    let mut processes = Vec::new();
    let network_counts = query_network_connections();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return processes;
        }
        let current_pid = GetCurrentProcessId();
        let windows_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let mut cache: HashMap<String, FileMetadata> = HashMap::new();
        let mut entry: PROCESSENTRY32W = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let pid = entry.th32ProcessID;
                let name = from_wide_z(&entry.szExeFile);
                let (path, memory_kb, cpu_total_100ns, io_total_bytes, handle_count) =
                    query_process_info(pid);
                let (safety, reason) =
                    classify_process(pid, current_pid, &name, &path, &windows_dir);
                let metadata = cached_file_metadata(&mut cache, &path);
                let category = categorize_process(&name, &path, &metadata, safety, &windows_dir);
                let risk_score = risk_score(safety, &name, &path, &metadata, &windows_dir);
                processes.push(ProcessInfo {
                    pid,
                    parent_pid: entry.th32ParentProcessID,
                    name,
                    category,
                    description: metadata.description,
                    company: metadata.company,
                    product: metadata.product,
                    original_filename: metadata.original_filename,
                    path,
                    memory_kb,
                    cpu_total_100ns,
                    io_total_bytes,
                    cpu_percent: 0.0,
                    io_rate_kbps: 0.0,
                    network_connections: network_counts.get(&pid).copied().unwrap_or(0),
                    thread_count: entry.cntThreads,
                    handle_count,
                    safety,
                    reason,
                    risk_score,
                });
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    processes
}

fn query_network_connections() -> HashMap<u32, u32> {
    let mut counts = HashMap::new();
    let Some(output) = Command::new("netstat.exe")
        .args(["-ano"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
    else {
        return counts;
    };
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("TCP") && !trimmed.starts_with("UDP") {
            continue;
        }
        if let Some(pid) = trimmed
            .split_whitespace()
            .last()
            .and_then(|value| value.parse::<u32>().ok())
        {
            if pid != 0 {
                *counts.entry(pid).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn build_groups(processes: &[ProcessInfo]) -> Vec<ProcessGroup> {
    let mut by_key: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, p) in processes.iter().enumerate() {
        by_key.entry(process_group_key(p)).or_default().push(i);
    }
    let mut groups = Vec::new();
    for (key, indices) in by_key {
        let first = &processes[indices[0]];
        let mut g = ProcessGroup {
            key,
            name: first.name.clone(),
            category: first.category.clone(),
            description: first.description.clone(),
            company: first.company.clone(),
            product: first.product.clone(),
            path: first.path.clone(),
            process_indices: indices,
            total_memory_kb: 0,
            total_cpu_percent: 0.0,
            total_io_rate_kbps: 0.0,
            total_network_connections: 0,
            total_threads: 0,
            total_handles: 0,
            safe_count: 0,
            caution_count: 0,
            unknown_count: 0,
            blocked_count: 0,
            max_risk_score: 0,
        };
        for idx in &g.process_indices {
            let p = &processes[*idx];
            g.total_memory_kb += p.memory_kb;
            g.total_cpu_percent += p.cpu_percent;
            g.total_io_rate_kbps += p.io_rate_kbps;
            g.total_network_connections += p.network_connections;
            g.total_threads += p.thread_count;
            g.total_handles += p.handle_count;
            g.max_risk_score = g.max_risk_score.max(p.risk_score);
            match p.safety {
                SafetyLevel::Safe => g.safe_count += 1,
                SafetyLevel::Caution => g.caution_count += 1,
                SafetyLevel::Unknown => g.unknown_count += 1,
                SafetyLevel::Blocked => g.blocked_count += 1,
            }
        }
        groups.push(g);
    }
    groups
}

fn process_group_key(process: &ProcessInfo) -> String {
    if process.path.is_empty() {
        process.name.to_ascii_lowercase()
    } else {
        format!(
            "{}|{}",
            process.name.to_ascii_lowercase(),
            process.path.to_ascii_lowercase()
        )
    }
}

fn classify_process(
    pid: u32,
    current_pid: u32,
    name: &str,
    path: &str,
    windows_dir: &str,
) -> (SafetyLevel, String) {
    let n = name.to_ascii_lowercase();
    let p = path.to_ascii_lowercase();
    let windir = windows_dir.trim_end_matches('\\').to_ascii_lowercase();
    let critical = [
        "system",
        "registry",
        "idle",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "services.exe",
        "lsass.exe",
        "lsaiso.exe",
        "winlogon.exe",
        "svchost.exe",
        "fontdrvhost.exe",
        "dwm.exe",
        "memory compression",
        "secure system",
    ];
    let security = [
        "msmpeng.exe",
        "securityhealthservice.exe",
        "securityhealthsystray.exe",
        "nissrv.exe",
        "mssense.exe",
    ];
    if pid == current_pid {
        return (SafetyLevel::Blocked, "Process Guard itself".to_string());
    }
    if pid <= 4 || critical.contains(&n.as_str()) {
        return (SafetyLevel::Blocked, "Windows core process".to_string());
    }
    if security.contains(&n.as_str()) {
        return (SafetyLevel::Blocked, "Security process".to_string());
    }
    if p.is_empty() {
        return (SafetyLevel::Unknown, "Path unavailable".to_string());
    }
    if n == "explorer.exe" {
        return (SafetyLevel::Caution, "Windows shell".to_string());
    }
    if p.starts_with(&(windir + "\\")) {
        return (SafetyLevel::Caution, "Windows component".to_string());
    }
    if n.contains("service") || n.ends_with("svc.exe") {
        return (SafetyLevel::Caution, "Service-like process".to_string());
    }
    if p.contains("\\program files\\")
        || p.contains("\\program files (x86)\\")
        || p.contains("\\appdata\\")
        || p.contains("\\users\\")
    {
        return (SafetyLevel::Safe, "User/application location".to_string());
    }
    (SafetyLevel::Unknown, "Unrecognized location".to_string())
}

fn categorize_process(
    name: &str,
    path: &str,
    metadata: &FileMetadata,
    safety: SafetyLevel,
    windows_dir: &str,
) -> String {
    let n = name.to_ascii_lowercase();
    let p = path.to_ascii_lowercase();
    let product = metadata.product.to_ascii_lowercase();
    let company = metadata.company.to_ascii_lowercase();
    let windir = windows_dir.trim_end_matches('\\').to_ascii_lowercase();
    if safety == SafetyLevel::Blocked {
        return "Windows/Core".to_string();
    }
    if matches!(
        n.as_str(),
        "chrome.exe" | "msedge.exe" | "firefox.exe" | "brave.exe" | "opera.exe" | "vivaldi.exe"
    ) {
        return "Browser".to_string();
    }
    if n.contains("discord") || n.contains("slack") || n.contains("teams") || n.contains("zoom") {
        return "Communication".to_string();
    }
    if n.contains("steam") || n.contains("epic") || n.contains("riot") || company.contains("valve")
    {
        return "Gaming".to_string();
    }
    if n.contains("onedrive") || product.contains("onedrive") {
        return "Sync".to_string();
    }
    if p.starts_with(&(windir + "\\")) {
        return "Windows Component".to_string();
    }
    if p.contains("\\program files\\") || p.contains("\\program files (x86)\\") {
        return "Installed App".to_string();
    }
    if p.contains("\\appdata\\") {
        return "User Background App".to_string();
    }
    if path.is_empty() {
        return "Protected/Unknown".to_string();
    }
    "Other".to_string()
}

fn risk_score(
    safety: SafetyLevel,
    name: &str,
    path: &str,
    metadata: &FileMetadata,
    windows_dir: &str,
) -> u8 {
    let mut score = match safety {
        SafetyLevel::Safe => 18,
        SafetyLevel::Caution => 58,
        SafetyLevel::Unknown => 72,
        SafetyLevel::Blocked => 98,
    };
    let p = path.to_ascii_lowercase();
    let windir = windows_dir.trim_end_matches('\\').to_ascii_lowercase();
    if metadata.company.is_empty() {
        score += 8;
    }
    if p.contains("\\temp\\") || p.contains("\\downloads\\") {
        score += 12;
    }
    if !p.is_empty() && p.starts_with(&(windir + "\\")) {
        score += 8;
    }
    if name.eq_ignore_ascii_case("explorer.exe") {
        score += 12;
    }
    score.min(100)
}

fn cached_file_metadata(cache: &mut HashMap<String, FileMetadata>, path: &str) -> FileMetadata {
    if path.is_empty() {
        return FileMetadata::default();
    }
    if let Some(m) = cache.get(path) {
        return m.clone();
    }
    let m = read_file_metadata(path);
    cache.insert(path.to_string(), m.clone());
    m
}

fn read_file_metadata(path: &str) -> FileMetadata {
    unsafe {
        let path_w = wide(path);
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(path_w.as_ptr(), &mut handle);
        if size == 0 {
            return FileMetadata::default();
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(
            path_w.as_ptr(),
            handle,
            size,
            data.as_mut_ptr() as *mut c_void,
        ) == 0
        {
            return FileMetadata::default();
        }
        let translations = version_translations(&data);
        let fallback = [(0x0409u16, 0x04b0u16), (0x0409u16, 0x04e4u16)];
        let pairs = if translations.is_empty() {
            &fallback[..]
        } else {
            &translations[..]
        };
        FileMetadata {
            description: first_version_string(&data, pairs, "FileDescription"),
            company: first_version_string(&data, pairs, "CompanyName"),
            product: first_version_string(&data, pairs, "ProductName"),
            original_filename: first_version_string(&data, pairs, "OriginalFilename"),
        }
    }
}

unsafe fn version_translations(data: &[u8]) -> Vec<(u16, u16)> {
    let sub = wide("\\VarFileInfo\\Translation");
    let mut ptr: *mut c_void = null_mut();
    let mut len = 0u32;
    if VerQueryValueW(
        data.as_ptr() as *const c_void,
        sub.as_ptr(),
        &mut ptr,
        &mut len,
    ) == 0
        || ptr.is_null()
        || len < 4
    {
        return Vec::new();
    }
    slice::from_raw_parts(ptr as *const u16, len as usize / 2)
        .chunks_exact(2)
        .map(|c| (c[0], c[1]))
        .collect()
}

unsafe fn first_version_string(data: &[u8], pairs: &[(u16, u16)], key: &str) -> String {
    for (lang, codepage) in pairs {
        let value = version_string(data, *lang, *codepage, key);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

unsafe fn version_string(data: &[u8], lang: u16, codepage: u16, key: &str) -> String {
    let sub = wide(&format!(
        "\\StringFileInfo\\{:04x}{:04x}\\{}",
        lang, codepage, key
    ));
    let mut ptr: *mut c_void = null_mut();
    let mut len = 0u32;
    if VerQueryValueW(
        data.as_ptr() as *const c_void,
        sub.as_ptr(),
        &mut ptr,
        &mut len,
    ) == 0
        || ptr.is_null()
        || len <= 1
    {
        return String::new();
    }
    from_wide_z(slice::from_raw_parts(ptr as *const u16, len as usize))
        .trim()
        .to_string()
}

unsafe fn query_process_info(pid: u32) -> (String, u64, u64, u64, u32) {
    let mut can_read_memory = true;
    let mut handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, 0, pid);
    if handle == null_mut() {
        can_read_memory = false;
        handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    }
    if handle == null_mut() {
        return (String::new(), 0, 0, 0, 0);
    }
    let path = query_process_path(handle);
    let memory = if can_read_memory {
        query_memory_kb(handle)
    } else {
        0
    };
    let cpu = query_cpu_time(handle);
    let io = query_io_bytes(handle);
    let handles = query_handle_count(handle);
    CloseHandle(handle);
    (path, memory, cpu, io, handles)
}

unsafe fn query_handle_count(handle: HANDLE) -> u32 {
    let mut count = 0u32;
    if GetProcessHandleCount(handle, &mut count) == 0 {
        0
    } else {
        count
    }
}

unsafe fn query_cpu_time(handle: HANDLE) -> u64 {
    let mut created: FILETIME = zeroed();
    let mut exited: FILETIME = zeroed();
    let mut kernel: FILETIME = zeroed();
    let mut user: FILETIME = zeroed();
    if GetProcessTimes(handle, &mut created, &mut exited, &mut kernel, &mut user) == 0 {
        return 0;
    }
    filetime_value(kernel).saturating_add(filetime_value(user))
}

fn filetime_value(value: FILETIME) -> u64 {
    ((value.dwHighDateTime as u64) << 32) | value.dwLowDateTime as u64
}

unsafe fn query_io_bytes(handle: HANDLE) -> u64 {
    let mut counters: IO_COUNTERS = zeroed();
    if GetProcessIoCounters(handle, &mut counters) == 0 {
        return 0;
    }
    counters
        .ReadTransferCount
        .saturating_add(counters.WriteTransferCount)
        .saturating_add(counters.OtherTransferCount)
}

unsafe fn query_process_path(handle: HANDLE) -> String {
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    if QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..size as usize])
}

unsafe fn query_memory_kb(handle: HANDLE) -> u64 {
    let mut counters: PROCESS_MEMORY_COUNTERS = zeroed();
    counters.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    if K32GetProcessMemoryInfo(
        handle,
        &mut counters,
        size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
    ) == 0
    {
        return 0;
    }
    counters.WorkingSetSize as u64 / 1024
}

unsafe fn terminate_pid(pid: u32) -> Result<(), u32> {
    let handle = OpenProcess(
        PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
        0,
        pid,
    );
    if handle == null_mut() {
        return Err(GetLastError());
    }
    let result = TerminateProcess(handle, 1);
    let error = GetLastError();
    CloseHandle(handle);
    if result == 0 { Err(error) } else { Ok(()) }
}

unsafe fn set_process_priority(pid: u32, class: u32) -> bool {
    let handle = OpenProcess(
        PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION,
        0,
        pid,
    );
    if handle.is_null() {
        return false;
    }
    let changed = SetPriorityClass(handle, class) != 0;
    CloseHandle(handle);
    changed
}

unsafe fn limit_process_affinity(pid: u32) -> bool {
    let handle = OpenProcess(
        PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION,
        0,
        pid,
    );
    if handle.is_null() {
        return false;
    }
    let mut process_mask = 0usize;
    let mut system_mask = 0usize;
    if GetProcessAffinityMask(handle, &mut process_mask, &mut system_mask) == 0 {
        CloseHandle(handle);
        return false;
    }
    let wanted = (system_mask.count_ones().max(1) + 1) / 2;
    let mut selected_mask = 0usize;
    let mut selected = 0;
    for bit in 0..usize::BITS {
        let value = 1usize << bit;
        if system_mask & value != 0 && selected < wanted {
            selected_mask |= value;
            selected += 1;
        }
    }
    let changed = selected_mask != 0 && SetProcessAffinityMask(handle, selected_mask) != 0;
    CloseHandle(handle);
    changed
}

unsafe fn suspend_or_resume_process(pid: u32, resume: bool) -> bool {
    let handle = OpenProcess(
        PROCESS_SUSPEND_RESUME_ACCESS | PROCESS_QUERY_LIMITED_INFORMATION,
        0,
        pid,
    );
    if handle.is_null() {
        return false;
    }
    let status = if resume {
        NtResumeProcess(handle)
    } else {
        NtSuspendProcess(handle)
    };
    CloseHandle(handle);
    status >= 0
}

fn explain_prompt(details: &str) -> String {
    format!(
        "{}\n\nSelected process data:\n{}",
        ai_system_instruction(),
        details
    )
}

fn followup_prompt(context: &str, history: &str, question: &str) -> String {
    format!(
        "{}\n\nSelected process data:\n{}\n\nConversation history:\n{}\n\nUser follow-up question:\n{}\n\nAnswer the follow-up directly. Keep the answer under 220 words unless the user asks for more.",
        ai_system_instruction(),
        context,
        history,
        question
    )
}

fn regenerate_prompt(context: &str, history: &str) -> String {
    format!(
        "{}\n\nSelected process data:\n{}\n\nConversation history:\n{}\n\nRegenerate the latest explanation with clearer practical detail. Do not ask for more data.",
        ai_system_instruction(),
        context,
        history
    )
}

fn ai_system_instruction() -> &'static str {
    "You are Process Guard's internal AI. Use only the selected Windows process data below; do not ask the user for a PID, path, or publisher because it is already included. Return a complete but fast explanation with these labeled lines when possible: Identity, Why it is running, Safety, RAM/CPU concern, What happens if ended, Recommended action. Never recommend ending Windows core, security, service, caution, unknown, or blocked processes."
}

fn provider_index(id: &str) -> Option<usize> {
    SENTINEL_PROVIDERS
        .iter()
        .position(|provider| provider.id == id)
}

fn provider_by_id(id: &str) -> Option<&'static SentinelProvider> {
    SENTINEL_PROVIDERS.iter().find(|provider| provider.id == id)
}

fn sentinel_model_tier(provider: &SentinelProvider, index: usize) -> SentinelModelTier {
    match provider.id {
        "codex-cli" | "claude-cli" | "gemini-cli" => match index {
            0 => SentinelModelTier::Included,
            2 => SentinelModelTier::Premium,
            _ => SentinelModelTier::Standard,
        },
        "gemini-api" => match index {
            0 | 1 => SentinelModelTier::Free,
            _ => SentinelModelTier::Premium,
        },
        "groq" => match index {
            0 => SentinelModelTier::Free,
            1 => SentinelModelTier::Premium,
            _ => SentinelModelTier::Standard,
        },
        "openrouter" => match index {
            0 => SentinelModelTier::Free,
            1 => SentinelModelTier::Standard,
            _ => SentinelModelTier::Premium,
        },
        "openai" | "anthropic" => {
            if index == 0 {
                SentinelModelTier::Standard
            } else {
                SentinelModelTier::Premium
            }
        }
        "xai" | "mistral" | "cohere" | "deepseek" => {
            if index + 1 == provider.models.len() {
                SentinelModelTier::Premium
            } else {
                SentinelModelTier::Standard
            }
        }
        "together" => {
            if index == 0 {
                SentinelModelTier::Standard
            } else {
                SentinelModelTier::Premium
            }
        }
        _ => SentinelModelTier::Standard,
    }
}

fn selected_sentinel_model_tier(provider: &SentinelProvider, model: &str) -> SentinelModelTier {
    provider
        .models
        .iter()
        .position(|candidate| *candidate == model)
        .map(|index| sentinel_model_tier(provider, index))
        .unwrap_or(SentinelModelTier::Standard)
}

fn sentinel_model_note(tier: SentinelModelTier) -> &'static str {
    match tier {
        SentinelModelTier::Included => {
            "CLI account model: uses the selected CLI's existing sign-in; account or plan limits may apply."
        }
        SentinelModelTier::Free => {
            "Free-tier model: provider account, region, quotas, rate limits, and current availability still apply."
        }
        SentinelModelTier::Standard => {
            "Standard/lower-cost choice: no permanent free API model is guaranteed; provider billing may apply."
        }
        SentinelModelTier::Premium => {
            "Premium model: higher usage cost and paid model access may apply. The test request may be billable."
        }
    }
}

fn sentinel_answer(config: &SentinelConfig, prompt: &str) -> String {
    let provider = provider_by_id(&config.provider_id).unwrap_or(&SENTINEL_PROVIDERS[0]);
    let model = if config.model.trim().is_empty() {
        provider.models.first().copied().unwrap_or("default")
    } else {
        config.model.trim()
    };
    let result = match provider.kind {
        SentinelBackendKind::CodexCli => run_codex_exec(prompt, model),
        SentinelBackendKind::ClaudeCli => run_claude_cli(prompt, model),
        SentinelBackendKind::GeminiCli => run_gemini_cli(prompt, model),
        _ => {
            if !is_installed_copy() {
                Err("API engines are disabled in the portable build. Install Process Guard with Setup first.".to_string())
            } else if let Some(key) = read_api_key(provider.id) {
                run_sentinel_api(provider.kind, &key, model, prompt)
            } else {
                Err(format!(
                    "No {} API key is stored. Open Help > Sentinel AI Settings.",
                    provider.label
                ))
            }
        }
    };
    match result {
        Ok(text) if !text.trim().is_empty() => {
            truncate_text(&clean_codex_output(text.trim()), 6000)
        }
        Ok(_) => format!(
            "Sentinel received an empty response from {}. Try Regenerate or choose another model.",
            provider.label
        ),
        Err(error) => format!(
            "Sentinel could not get a response from {}.\r\n\r\n{}\r\n\r\nThe local process summary remains available in this chat.",
            provider.label, error
        ),
    }
}

fn test_sentinel_config(config: &SentinelConfig, api_key: Option<&str>) -> Result<String, String> {
    let provider = provider_by_id(&config.provider_id)
        .ok_or_else(|| "The selected Sentinel provider is no longer available.".to_string())?;
    let model = if config.model.trim().is_empty() {
        provider.models.first().copied().unwrap_or("default")
    } else {
        config.model.trim()
    };
    let prompt = "Process Guard Sentinel connectivity test. Reply with exactly SENTINEL_OK and nothing else. Do not use tools or inspect files.";
    let result = match provider.kind {
        SentinelBackendKind::CodexCli => run_codex_exec(prompt, model),
        SentinelBackendKind::ClaudeCli => run_claude_cli(prompt, model),
        SentinelBackendKind::GeminiCli => run_gemini_cli(prompt, model),
        _ => {
            if !is_installed_copy() {
                Err("API engines can only be tested from the installed application.".to_string())
            } else if let Some(key) = api_key.filter(|value| !value.trim().is_empty()) {
                run_sentinel_api(provider.kind, key, model, prompt)
            } else {
                Err(format!(
                    "No {} API key is available for the test.",
                    provider.label
                ))
            }
        }
    };
    let response = result?;
    if response.trim().is_empty() {
        Err(format!(
            "{} connected but returned an empty response for model {}. Settings were not changed.",
            provider.label, model
        ))
    } else {
        Ok(format!(
            "TEST PASSED | {} responded using {}. Saving settings and restarting...",
            provider.label, model
        ))
    }
}

fn run_codex_exec(prompt: &str, model: &str) -> Result<String, String> {
    if let Some(appdata) = std::env::var("APPDATA").ok() {
        let codex_js = format!(
            "{}\\npm\\node_modules\\@openai\\codex\\bin\\codex.js",
            appdata
        );
        if std::path::Path::new(&codex_js).exists() {
            let local_node = format!("{}\\npm\\node.exe", appdata);
            let node = if std::path::Path::new(&local_node).exists() {
                local_node
            } else {
                "node.exe".to_string()
            };
            let mut command = Command::new(node);
            command
                .arg(codex_js)
                .arg("exec")
                .arg("--ephemeral")
                .arg("--skip-git-repo-check")
                .arg("-s")
                .arg("read-only")
                .arg("-c")
                .arg("model_reasoning_effort=\"low\"");
            if model != "default" {
                command.arg("-m").arg(model);
            }
            let output = command
                .arg(prompt)
                .creation_flags(CREATE_NO_WINDOW)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .map_err(|_| "Codex CLI could not be started.".to_string())?;
            return command_result(output, "Codex CLI");
        }
    }

    let codex = npm_cli_path("codex");
    let script = r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$cliArgs=@('exec','--ephemeral','--skip-git-repo-check','-s','read-only','-c','model_reasoning_effort="low"')
if($env:PROCESS_GUARD_AI_MODEL -ne 'default'){$cliArgs += @('-m',$env:PROCESS_GUARD_AI_MODEL)}
$cliArgs += $env:PROCESS_GUARD_AI_PROMPT
& $env:PROCESS_GUARD_AI_CLI @cliArgs
exit $LASTEXITCODE
"#;
    run_cli_powershell(script, &codex, model, prompt, "Codex CLI")
}

fn run_claude_cli(prompt: &str, model: &str) -> Result<String, String> {
    let cli = npm_cli_path("claude");
    let script = r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$cliArgs=@('-p',$env:PROCESS_GUARD_AI_PROMPT,'--output-format','text','--permission-mode','plan')
if($env:PROCESS_GUARD_AI_MODEL -ne 'default'){$cliArgs += @('--model',$env:PROCESS_GUARD_AI_MODEL)}
& $env:PROCESS_GUARD_AI_CLI @cliArgs
exit $LASTEXITCODE
"#;
    run_cli_powershell(script, &cli, model, prompt, "Claude CLI")
}

fn run_gemini_cli(prompt: &str, model: &str) -> Result<String, String> {
    let cli = npm_cli_path("gemini");
    let script = r#"
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$cliArgs=@('-p',$env:PROCESS_GUARD_AI_PROMPT,'-o','text')
if($env:PROCESS_GUARD_AI_MODEL -ne 'default' -and $env:PROCESS_GUARD_AI_MODEL -ne 'auto'){$cliArgs += @('-m',$env:PROCESS_GUARD_AI_MODEL)}
& $env:PROCESS_GUARD_AI_CLI @cliArgs
exit $LASTEXITCODE
"#;
    run_cli_powershell(script, &cli, model, prompt, "Gemini CLI")
}

fn npm_cli_path(name: &str) -> String {
    std::env::var("APPDATA")
        .ok()
        .map(|path| format!("{}\\npm\\{}.cmd", path, name))
        .filter(|path| std::path::Path::new(path).exists())
        .unwrap_or_else(|| format!("{}.cmd", name))
}

fn run_cli_powershell(
    script: &str,
    cli: &str,
    model: &str,
    prompt: &str,
    label: &str,
) -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("PROCESS_GUARD_AI_CLI", cli)
        .env("PROCESS_GUARD_AI_MODEL", model)
        .env("PROCESS_GUARD_AI_PROMPT", prompt)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|_| {
            format!(
                "{} could not be started. Install it, sign in, or choose another Sentinel engine.",
                label
            )
        })?;
    command_result(output, label)
}

fn command_result(output: std::process::Output, label: &str) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !stdout.is_empty() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err(format!(
            "{} returned no answer. Check that it is installed and signed in.",
            label
        ))
    } else {
        Err(format!(
            "{} failed: {}",
            label,
            truncate_text(&stderr.replace(['\r', '\n'], " "), 420)
        ))
    }
}

fn run_sentinel_api(
    kind: SentinelBackendKind,
    key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    match kind {
        SentinelBackendKind::OpenAi => run_openai_compatible_api(
            "https://api.openai.com/v1/chat/completions",
            key,
            model,
            prompt,
        ),
        SentinelBackendKind::XAi => {
            run_openai_compatible_api("https://api.x.ai/v1/chat/completions", key, model, prompt)
        }
        SentinelBackendKind::Groq => run_openai_compatible_api(
            "https://api.groq.com/openai/v1/chat/completions",
            key,
            model,
            prompt,
        ),
        SentinelBackendKind::Mistral => run_openai_compatible_api(
            "https://api.mistral.ai/v1/chat/completions",
            key,
            model,
            prompt,
        ),
        SentinelBackendKind::DeepSeek => run_openai_compatible_api(
            "https://api.deepseek.com/chat/completions",
            key,
            model,
            prompt,
        ),
        SentinelBackendKind::OpenRouter => run_openai_compatible_api(
            "https://openrouter.ai/api/v1/chat/completions",
            key,
            model,
            prompt,
        ),
        SentinelBackendKind::Together => run_openai_compatible_api(
            "https://api.together.xyz/v1/chat/completions",
            key,
            model,
            prompt,
        ),
        SentinelBackendKind::Anthropic => run_anthropic_api(key, model, prompt),
        SentinelBackendKind::GeminiApi => run_gemini_api(key, model, prompt),
        SentinelBackendKind::Cohere => run_cohere_api(key, model, prompt),
        _ => Err("The selected Sentinel engine is not an API provider.".to_string()),
    }
}

fn run_openai_compatible_api(
    endpoint: &str,
    key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let script = r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$headers=@{Authorization=('Bearer ' + $env:PROCESS_GUARD_AI_KEY)}
$body=@{model=$env:PROCESS_GUARD_AI_MODEL;messages=@(@{role='user';content=$env:PROCESS_GUARD_AI_PROMPT})} | ConvertTo-Json -Depth 8 -Compress
$response=Invoke-RestMethod -Method Post -Uri $env:PROCESS_GUARD_AI_ENDPOINT -Headers $headers -ContentType 'application/json; charset=utf-8' -Body $body -TimeoutSec 90
$content=$response.choices[0].message.content
if($content -is [string]){$content}else{($content | ForEach-Object { if($_.text){$_.text}else{"$_"} }) -join [Environment]::NewLine}
"#;
    powershell_api_output(script, endpoint, key, model, prompt)
}

fn run_anthropic_api(key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let script = r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$headers=@{'x-api-key'=$env:PROCESS_GUARD_AI_KEY;'anthropic-version'='2023-06-01'}
$body=@{model=$env:PROCESS_GUARD_AI_MODEL;max_tokens=1200;messages=@(@{role='user';content=$env:PROCESS_GUARD_AI_PROMPT})} | ConvertTo-Json -Depth 8 -Compress
$response=Invoke-RestMethod -Method Post -Uri $env:PROCESS_GUARD_AI_ENDPOINT -Headers $headers -ContentType 'application/json; charset=utf-8' -Body $body -TimeoutSec 90
($response.content | Where-Object {$_.type -eq 'text'} | ForEach-Object {$_.text}) -join [Environment]::NewLine
"#;
    powershell_api_output(
        script,
        "https://api.anthropic.com/v1/messages",
        key,
        model,
        prompt,
    )
}

fn run_gemini_api(key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let script = r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$headers=@{'x-goog-api-key'=$env:PROCESS_GUARD_AI_KEY}
$model=[Uri]::EscapeDataString($env:PROCESS_GUARD_AI_MODEL)
$uri=$env:PROCESS_GUARD_AI_ENDPOINT.Replace('{model}',$model)
$body=@{contents=@(@{parts=@(@{text=$env:PROCESS_GUARD_AI_PROMPT})})} | ConvertTo-Json -Depth 8 -Compress
$response=Invoke-RestMethod -Method Post -Uri $uri -Headers $headers -ContentType 'application/json; charset=utf-8' -Body $body -TimeoutSec 90
($response.candidates[0].content.parts | ForEach-Object {$_.text}) -join [Environment]::NewLine
"#;
    powershell_api_output(
        script,
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent",
        key,
        model,
        prompt,
    )
}

fn run_cohere_api(key: &str, model: &str, prompt: &str) -> Result<String, String> {
    let script = r#"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$headers=@{Authorization=('Bearer ' + $env:PROCESS_GUARD_AI_KEY)}
$body=@{model=$env:PROCESS_GUARD_AI_MODEL;messages=@(@{role='user';content=$env:PROCESS_GUARD_AI_PROMPT})} | ConvertTo-Json -Depth 8 -Compress
$response=Invoke-RestMethod -Method Post -Uri $env:PROCESS_GUARD_AI_ENDPOINT -Headers $headers -ContentType 'application/json; charset=utf-8' -Body $body -TimeoutSec 90
($response.message.content | ForEach-Object {$_.text}) -join [Environment]::NewLine
"#;
    powershell_api_output(script, "https://api.cohere.com/v2/chat", key, model, prompt)
}

fn powershell_api_output(
    script: &str,
    endpoint: &str,
    key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("PROCESS_GUARD_AI_ENDPOINT", endpoint)
        .env("PROCESS_GUARD_AI_KEY", key)
        .env("PROCESS_GUARD_AI_MODEL", model)
        .env("PROCESS_GUARD_AI_PROMPT", prompt)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| "The secure API worker could not be started.".to_string())?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() && !text.is_empty() {
        Ok(text)
    } else {
        Err("The provider rejected or could not complete the request. Check the key, model ID, internet connection, account access, and billing status.".to_string())
    }
}

fn clean_codex_output(value: &str) -> String {
    if let Some((first, _)) = value.split_once("Reading additional input from stdin") {
        return first.trim().to_string();
    }
    if let Some((first, _)) = value.split_once("OpenAI Codex") {
        return first.trim().to_string();
    }
    value.trim().to_string()
}

fn process_explanation(p: &ProcessInfo) -> String {
    format!(
        "{} is marked {} because {}. It appears to be a {} from {}.",
        p.name,
        p.safety.label(),
        p.reason,
        p.category,
        display_or_dash(&p.company)
    )
}

fn group_explanation(g: &ProcessGroup) -> String {
    format!(
        "This group contains {} process instance(s) from the same executable identity. Safe child processes: {}. Highest risk: {}%.",
        g.process_indices.len(),
        g.safe_count,
        g.max_risk_score
    )
}

fn group_reason(g: &ProcessGroup) -> String {
    format!(
        "{} safe / {} caution / {} unknown / {} blocked",
        g.safe_count, g.caution_count, g.unknown_count, g.blocked_count
    )
}

fn group_safety_label(g: &ProcessGroup) -> &'static str {
    if g.safe_count == g.process_indices.len() {
        "Safe"
    } else if g.blocked_count == g.process_indices.len() {
        "Blocked"
    } else {
        "Mixed"
    }
}

fn format_memory(kb: u64) -> String {
    if kb == 0 {
        "-".to_string()
    } else if kb >= 1024 * 1024 {
        format!("{:.1} GB", kb as f64 / 1024.0 / 1024.0)
    } else if kb >= 1024 {
        format!("{:.1} MB", kb as f64 / 1024.0)
    } else {
        format!("{} KB", kb)
    }
}

fn format_rate(kbps: f32) -> String {
    if kbps >= 1024.0 {
        format!("{:.1} MB/s", kbps / 1024.0)
    } else {
        format!("{:.0} KB/s", kbps)
    }
}

fn performance_graph(values: &VecDeque<f32>) -> String {
    const LEVELS: &[u8] = b".:-=+*#%@";
    values
        .iter()
        .map(|value| {
            let index = ((*value / 100.0) * (LEVELS.len() - 1) as f32)
                .round()
                .clamp(0.0, (LEVELS.len() - 1) as f32) as usize;
            LEVELS[index] as char
        })
        .collect()
}

fn truncate_text(value: &str, max: usize) -> String {
    let mut out = value.chars().take(max).collect::<String>();
    if value.chars().count() > max {
        out.push_str("\r\n\r\n[truncated]");
    }
    out
}

fn clean_ai_message(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("Codex AI")
        .trim_start()
        .trim_matches('\r')
        .trim_matches('\n')
        .trim()
        .to_string()
}

fn history_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|dir| dir.join("ProcessGuard_Sentinel_History.txt"))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("ProcessGuard_Sentinel_History.txt"))
}

fn settings_path() -> std::path::PathBuf {
    executable_sibling("ProcessGuard_Settings.txt")
}

fn snapshot_path() -> std::path::PathBuf {
    executable_sibling("ProcessGuard_Snapshot.txt")
}

fn executable_sibling(name: &str) -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|directory| directory.join(name)))
        .unwrap_or_else(|| std::path::PathBuf::from(name))
}

fn is_installed_copy() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(directory) = exe.parent() else {
        return false;
    };
    directory.join("ProcessGuard.installed").is_file()
        && directory.join("Uninstall").join("unins000.exe").is_file()
}

fn load_guard_settings() -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) {
    let mut watch = BTreeSet::new();
    let mut alerts = BTreeSet::new();
    let mut automation = BTreeSet::new();
    if let Ok(text) = std::fs::read_to_string(settings_path()) {
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("WATCH\t") {
                watch.insert(value.to_string());
            } else if let Some(value) = line.strip_prefix("ALERT\t") {
                alerts.insert(value.to_string());
            } else if let Some(value) = line.strip_prefix("AUTO\t") {
                automation.insert(value.to_string());
            }
        }
    }
    (watch, alerts, automation)
}

fn load_sentinel_config() -> SentinelConfig {
    let mut config = SentinelConfig::default();
    if let Ok(text) = std::fs::read_to_string(settings_path()) {
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("SENTINEL_PROVIDER\t") {
                config.provider_id = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("SENTINEL_MODEL\t") {
                config.model = value.trim().to_string();
            }
        }
    }
    let Some(provider) = provider_by_id(&config.provider_id) else {
        return SentinelConfig::default();
    };
    if config.model.is_empty() {
        config.model = provider
            .models
            .first()
            .copied()
            .unwrap_or("default")
            .to_string();
    }
    config
}

fn save_guard_settings(
    watch: &BTreeSet<String>,
    alerts: &BTreeSet<String>,
    automation: &BTreeSet<String>,
    sentinel: &SentinelConfig,
) {
    let mut out = String::new();
    out.push_str(&format!(
        "SENTINEL_PROVIDER\t{}\nSENTINEL_MODEL\t{}\n",
        sanitize_setting(&sentinel.provider_id),
        sanitize_setting(&sentinel.model)
    ));
    for value in watch {
        out.push_str(&format!("WATCH\t{}\n", value));
    }
    for value in alerts {
        out.push_str(&format!("ALERT\t{}\n", value));
    }
    for value in automation {
        out.push_str(&format!("AUTO\t{}\n", value));
    }
    let _ = std::fs::write(settings_path(), out);
}

fn sanitize_setting(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ").trim().to_string()
}

fn credential_target(provider_id: &str) -> Vec<u16> {
    wide(&format!("{}{}", SENTINEL_CREDENTIAL_PREFIX, provider_id))
}

fn write_api_key(provider_id: &str, key: &str) -> Result<(), u32> {
    if !is_installed_copy() || key.is_empty() || key.len() > 2400 {
        return Err(5);
    }
    unsafe {
        let mut target = credential_target(provider_id);
        let mut username = wide("Process Guard Sentinel");
        let mut blob = key.as_bytes().to_vec();
        let credential = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: null_mut(),
            LastWritten: zeroed(),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: null_mut(),
            TargetAlias: null_mut(),
            UserName: username.as_mut_ptr(),
        };
        if CredWriteW(&credential, 0) == 0 {
            Err(GetLastError())
        } else {
            Ok(())
        }
    }
}

fn read_api_key(provider_id: &str) -> Option<String> {
    if !is_installed_copy() {
        return None;
    }
    unsafe {
        let target = credential_target(provider_id);
        let mut credential: *mut CREDENTIALW = null_mut();
        if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) == 0
            || credential.is_null()
        {
            return None;
        }
        let item = &*credential;
        let bytes = if item.CredentialBlob.is_null() || item.CredentialBlobSize == 0 {
            Vec::new()
        } else {
            slice::from_raw_parts(item.CredentialBlob, item.CredentialBlobSize as usize).to_vec()
        };
        CredFree(credential as *const c_void);
        String::from_utf8(bytes)
            .ok()
            .filter(|value| !value.is_empty())
    }
}

fn delete_api_key(provider_id: &str) -> bool {
    if !is_installed_copy() {
        return false;
    }
    unsafe {
        let target = credential_target(provider_id);
        CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) != 0 || GetLastError() == 1168
    }
}

unsafe fn clipboard_text(hwnd: HWND) -> Option<String> {
    if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 || OpenClipboard(hwnd) == 0 {
        return None;
    }
    let handle = GetClipboardData(CF_UNICODETEXT);
    if handle.is_null() {
        CloseClipboard();
        return None;
    }
    let pointer = GlobalLock(handle) as *const u16;
    if pointer.is_null() {
        CloseClipboard();
        return None;
    }
    let mut length = 0usize;
    while length < 8192 && *pointer.add(length) != 0 {
        length += 1;
    }
    let text = String::from_utf16_lossy(slice::from_raw_parts(pointer, length));
    GlobalUnlock(handle);
    CloseClipboard();
    Some(text)
}

fn short_rule_key(value: &str) -> &str {
    value.split('|').next().unwrap_or(value)
}

fn escape_snapshot_field(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

fn powershell_output(script: &str) -> Option<String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stdout.is_empty() {
        Some(stdout)
    } else if !stderr.is_empty() {
        Some(stderr)
    } else {
        None
    }
}

fn save_ai_sessions(sessions: &[AiSession]) {
    let mut out = String::from("PROCESS_GUARD_SENTINEL_HISTORY_V1\r\n");
    for session in sessions.iter().rev().take(80).rev() {
        out.push_str(&format!(
            "SESSION\t{}\t{}\t{}\r\n",
            if session.pinned { "1" } else { "0" },
            escape_history_field(&session.title),
            escape_history_field(&session.context)
        ));
        for message in &session.messages {
            out.push_str(&format!(
                "MSG\t{}\t{}\r\n",
                escape_history_field(message.speaker),
                escape_history_field(&message.text)
            ));
        }
        out.push_str("END\r\n");
    }
    let _ = std::fs::write(history_path(), out);
}

fn load_ai_sessions() -> Vec<AiSession> {
    let Ok(text) = std::fs::read_to_string(history_path()) else {
        return Vec::new();
    };
    let mut sessions = Vec::new();
    let mut current: Option<AiSession> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("SESSION\t") {
            if let Some(session) = current.take() {
                sessions.push(session);
            }
            let parts = rest.splitn(3, '\t').collect::<Vec<_>>();
            if parts.len() == 3 {
                current = Some(AiSession {
                    pinned: parts[0] == "1",
                    title: unescape_history_field(parts[1]),
                    context: unescape_history_field(parts[2]),
                    messages: Vec::new(),
                });
            }
        } else if let Some(rest) = line.strip_prefix("MSG\t") {
            if let Some(session) = current.as_mut() {
                let parts = rest.splitn(2, '\t').collect::<Vec<_>>();
                if parts.len() == 2 {
                    session.messages.push(AiMessage::new(
                        speaker_from_history(&unescape_history_field(parts[0])),
                        unescape_history_field(parts[1]),
                    ));
                }
            }
        } else if line == "END" {
            if let Some(session) = current.take() {
                sessions.push(session);
            }
        }
    }
    if let Some(session) = current {
        sessions.push(session);
    }
    sessions
}

fn speaker_from_history(value: &str) -> &'static str {
    match value {
        SPEAKER_USER => SPEAKER_USER,
        SPEAKER_LOCAL => SPEAKER_LOCAL,
        "Status" => "Status",
        _ => SPEAKER_AI,
    }
}

fn escape_history_field(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\r', "")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn unescape_history_field(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push_str("\r\n"),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

unsafe fn load_app_icon(hwnd: HWND) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let icon = dir.join("ProcessGuard.ico");
    let Some(icon) = icon.to_str() else {
        return;
    };
    let icon = wide(icon);
    let big = LoadImageW(
        null_mut(),
        icon.as_ptr(),
        IMAGE_ICON,
        32,
        32,
        LR_LOADFROMFILE,
    );
    if big != null_mut() {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, big as LPARAM);
    }
    let small = LoadImageW(
        null_mut(),
        icon.as_ptr(),
        IMAGE_ICON,
        16,
        16,
        LR_LOADFROMFILE,
    );
    if small != null_mut() {
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, small as LPARAM);
    }
}

unsafe fn apply_dark_title_bar(hwnd: HWND) {
    let dark = 1i32;
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
        &dark as *const _ as *const c_void,
        size_of::<i32>() as u32,
    );

    let caption = C_BG;
    let border = C_BORDER;
    let text = C_TEXT;
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_CAPTION_COLOR as u32,
        &caption as *const _ as *const c_void,
        size_of::<COLORREF>() as u32,
    );
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_BORDER_COLOR as u32,
        &border as *const _ as *const c_void,
        size_of::<COLORREF>() as u32,
    );
    DwmSetWindowAttribute(
        hwnd,
        DWMWA_TEXT_COLOR as u32,
        &text as *const _ as *const c_void,
        size_of::<COLORREF>() as u32,
    );
}

unsafe fn is_process_elevated() -> bool {
    let mut token: HANDLE = null_mut();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
        return false;
    }

    let mut elevation: TOKEN_ELEVATION = zeroed();
    let mut returned = 0u32;
    let ok = GetTokenInformation(
        token,
        TokenElevation,
        &mut elevation as *mut _ as *mut c_void,
        size_of::<TOKEN_ELEVATION>() as u32,
        &mut returned,
    ) != 0;
    CloseHandle(token);
    ok && elevation.TokenIsElevated != 0
}

unsafe fn launch_elevated_copy(hwnd: HWND) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(exe) = exe.to_str() else {
        return false;
    };
    let operation = wide("runas");
    let exe = wide(exe);
    ShellExecuteW(
        hwnd,
        operation.as_ptr(),
        exe.as_ptr(),
        null(),
        null(),
        SW_SHOWNORMAL,
    ) as isize
        > 32
}

unsafe fn launch_unelevated_copy(hwnd: HWND) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(exe) = exe.to_str() else {
        return false;
    };

    let operation = wide("open");
    let explorer = wide("explorer.exe");
    let args = wide(&format!("\"{}\"", exe));
    ShellExecuteW(
        hwnd,
        operation.as_ptr(),
        explorer.as_ptr(),
        args.as_ptr(),
        null(),
        SW_SHOWNORMAL,
    ) as isize
        > 32
}

unsafe fn relaunch_current_copy(hwnd: HWND) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(exe_text) = exe.to_str() else {
        return false;
    };
    let operation = wide("open");
    let executable = wide(exe_text);
    let directory = exe.parent().and_then(|path| path.to_str()).map(wide);
    ShellExecuteW(
        hwnd,
        operation.as_ptr(),
        executable.as_ptr(),
        null(),
        directory.as_ref().map_or(null(), |value| value.as_ptr()),
        SW_SHOWNORMAL,
    ) as isize
        > 32
}

unsafe fn show_message(hwnd: HWND, title: &str, text: &str, flags: u32) {
    let title = wide(title);
    let text = wide(text);
    MessageBoxW(hwnd, text.as_ptr(), title.as_ptr(), flags);
}

unsafe fn ask_yes_no(hwnd: HWND, title: &str, text: &str) -> bool {
    let title = wide(title);
    let text = wide(text);
    MessageBoxW(
        hwnd,
        text.as_ptr(),
        title.as_ptr(),
        MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
    ) == 6
}

unsafe fn launch_user_process(hwnd: HWND, input: &str) -> bool {
    let (file, params) = split_launch_command(input);
    if file.trim().is_empty() {
        return false;
    }
    let operation = wide("open");
    let file = wide(&file);
    let params = wide(&params);
    let result = ShellExecuteW(
        hwnd,
        operation.as_ptr(),
        file.as_ptr(),
        if params.len() > 1 {
            params.as_ptr()
        } else {
            null()
        },
        null(),
        SW_SHOWNORMAL,
    );
    result as isize > 32
}

fn split_launch_command(input: &str) -> (String, String) {
    let trimmed = input.trim();
    if trimmed.starts_with('"') {
        if let Some(end) = trimmed[1..].find('"') {
            let end = end + 1;
            let file = trimmed[1..end].to_string();
            let params = trimmed[end + 1..].trim().to_string();
            return (file, params);
        }
    }
    if let Some(index) = trimmed.find(char::is_whitespace) {
        (
            trimmed[..index].to_string(),
            trimmed[index..].trim().to_string(),
        )
    } else {
        (trimmed.to_string(), String::new())
    }
}

unsafe fn open_file_location(hwnd: HWND, path: &str) -> bool {
    let operation = wide("open");
    let explorer = wide("explorer.exe");
    let args = wide(&format!("/select,\"{}\"", path));
    let result = ShellExecuteW(
        hwnd,
        operation.as_ptr(),
        explorer.as_ptr(),
        args.as_ptr(),
        null(),
        SW_SHOWNORMAL,
    );
    result as isize > 32
}

unsafe fn shell_open(hwnd: HWND, target: &str) -> bool {
    let verb = wide("open");
    let target = wide(target);
    ShellExecuteW(
        hwnd,
        verb.as_ptr(),
        target.as_ptr(),
        null(),
        null(),
        SW_SHOWNORMAL,
    ) as isize
        > 32
}

unsafe fn add_tray_icon(hwnd: HWND) -> bool {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_UID;
    data.uFlags = NIF_MESSAGE | NIF_TIP;
    data.uCallbackMessage = WM_TRAY_ICON;
    let icon = load_icon_handle(16);
    if !icon.is_null() {
        data.uFlags |= NIF_ICON;
        data.hIcon = icon;
    }
    let tip = wide("Process Guard - double-click to restore");
    for (index, value) in tip.into_iter().take(data.szTip.len()).enumerate() {
        data.szTip[index] = value;
    }
    Shell_NotifyIconW(NIM_ADD, &data) != 0
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_UID;
    Shell_NotifyIconW(NIM_DELETE, &data);
}

unsafe fn show_tray_notification(hwnd: HWND, title: &str, message: &str) {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_UID;
    data.uFlags = NIF_INFO;
    data.dwInfoFlags = NIIF_WARNING;
    let title = wide(title);
    let message = wide(message);
    for (index, value) in title
        .iter()
        .take(data.szInfoTitle.len().saturating_sub(1))
        .enumerate()
    {
        data.szInfoTitle[index] = *value;
    }
    for (index, value) in message
        .iter()
        .take(data.szInfo.len().saturating_sub(1))
        .enumerate()
    {
        data.szInfo[index] = *value;
    }
    Shell_NotifyIconW(NIM_MODIFY, &data);
}

unsafe fn load_icon_handle(size: i32) -> HANDLE {
    let Ok(exe) = std::env::current_exe() else {
        return null_mut();
    };
    let Some(dir) = exe.parent() else {
        return null_mut();
    };
    let icon = dir.join("ProcessGuard.ico");
    let Some(icon) = icon.to_str() else {
        return null_mut();
    };
    let icon = wide(icon);
    LoadImageW(
        null_mut(),
        icon.as_ptr(),
        IMAGE_ICON,
        size,
        size,
        LR_LOADFROMFILE,
    )
}

fn display_or_dash(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value.trim().to_string()
    }
}

fn point_in(r: RECT, x: i32, y: i32) -> bool {
    x >= r.left && x <= r.right && y >= r.top && y <= r.bottom
}

fn ai_layout(
    client: RECT,
    popup_pos: Option<(i32, i32)>,
    popup_size: Option<(i32, i32)>,
) -> AiLayout {
    let client_w = (client.right - client.left).max(1);
    let client_h = (client.bottom - client.top).max(1);
    let available_w = (client_w - 24).max(360);
    let available_h = (client_h - 24).max(300);
    let min_w = if available_w < 620 {
        available_w.max(360)
    } else {
        620
    };
    let min_h = if available_h < 380 {
        available_h.max(300)
    } else {
        380
    };
    let default_w = available_w.min(1120).max(min_w);
    let default_h = available_h.min(620).max(min_h);
    let (requested_w, requested_h) = popup_size.unwrap_or((default_w, default_h));
    let w = requested_w.clamp(min_w, available_w.min(1120).max(min_w));
    let h = requested_h.clamp(min_h, available_h.min(720).max(min_h));
    let (left, top) = popup_pos.unwrap_or(((client.right - w) / 2, (client.bottom - h) / 2));
    let max_left = (client.right - w - 8).max(client.left + 8);
    let max_top = (client.bottom - h - 8).max(client.top + 8);
    let left = left.clamp(client.left + 8, max_left);
    let top = top.clamp(client.top + 8, max_top);
    let popup = rect(left, top, w, h);
    let drag = rect(popup.left, popup.top, w, 44);
    let fit = rect(popup.right - 62, popup.top + 12, 44, 26);
    let larger = rect(fit.left - 36, popup.top + 12, 28, 26);
    let smaller = rect(larger.left - 34, popup.top + 12, 28, 26);
    let resize = rect(popup.right - 28, popup.bottom - 28, 28, 28);
    let history_w = if w < 560 {
        120
    } else if w < 760 {
        170
    } else {
        230
    };
    let history = rect(popup.left + 18, popup.top + 52, history_w, h - 78);
    let right_left = history.right + 14;
    let right_width = popup.right - right_left - 18;
    let compact_suggestions = right_width < 560;
    let suggestion_rows_h = if compact_suggestions { 60 } else { 26 };
    let input_h = 76;
    let input_top = popup.bottom - 24 - 8 - suggestion_rows_h - 8 - input_h;
    let regenerate_w = if right_width < 360 { 92 } else { 112 };
    let input = rect(
        right_left,
        input_top,
        (right_width - 58 - regenerate_w - 16).max(120),
        input_h,
    );
    let ask = rect(input.right + 8, input.top, 58, 32);
    let regenerate = rect(ask.right + 8, input.top, regenerate_w, 32);
    let suggestion_top = input.bottom + 8;
    let suggestions = if compact_suggestions {
        let suggestion_w = ((right_width - 8) / 2).max(130);
        [
            rect(right_left, suggestion_top, suggestion_w, 26),
            rect(
                right_left + suggestion_w + 8,
                suggestion_top,
                suggestion_w,
                26,
            ),
            rect(right_left, suggestion_top + 34, suggestion_w, 26),
            rect(
                right_left + suggestion_w + 8,
                suggestion_top + 34,
                suggestion_w,
                26,
            ),
        ]
    } else {
        let suggestion_w = ((right_width - 24) / 4).max(110);
        [
            rect(right_left, suggestion_top, suggestion_w, 26),
            rect(
                right_left + (suggestion_w + 8),
                suggestion_top,
                suggestion_w,
                26,
            ),
            rect(
                right_left + ((suggestion_w + 8) * 2),
                suggestion_top,
                suggestion_w,
                26,
            ),
            rect(
                right_left + ((suggestion_w + 8) * 3),
                suggestion_top,
                suggestion_w,
                26,
            ),
        ]
    };
    let body = rect(
        right_left,
        popup.top + 52,
        right_width,
        input.top - popup.top - 62,
    );
    let pin = rect(history.left + 8, history.bottom - 84, 58, 26);
    let clear_one = rect(pin.right + 6, pin.top, 78, 26);
    let clear_all = rect(
        history.left + 8,
        pin.bottom + 6,
        history.right - history.left - 16,
        26,
    );
    AiLayout {
        popup,
        drag,
        resize,
        history,
        body,
        input,
        ask,
        regenerate,
        pin,
        clear_one,
        clear_all,
        smaller,
        larger,
        fit,
        suggestions,
    }
}

fn ai_window_layout(client: RECT) -> AiLayout {
    let w = (client.right - client.left).max(520);
    let h = (client.bottom - client.top).max(360);
    let popup = rect(client.left, client.top, w, h);
    let drag = rect(-100, -100, 0, 0);
    let resize = rect(-100, -100, 0, 0);
    let fit = rect(popup.right - 62, popup.top + 12, 44, 26);
    let larger = rect(fit.left - 36, popup.top + 12, 28, 26);
    let smaller = rect(larger.left - 34, popup.top + 12, 28, 26);
    let history_w = if w < 720 { 170 } else { 230 };
    let history = rect(popup.left + 14, popup.top + 50, history_w, h - 76);
    let right_left = history.right + 14;
    let right_width = (popup.right - right_left - 14).max(260);
    let compact_suggestions = right_width < 560;
    let suggestion_rows_h = if compact_suggestions { 60 } else { 26 };
    let input_h = 76;
    let input_top = popup.bottom - 24 - 8 - suggestion_rows_h - 8 - input_h;
    let regenerate_w = if right_width < 360 { 92 } else { 112 };
    let input = rect(
        right_left,
        input_top,
        (right_width - 58 - regenerate_w - 16).max(120),
        input_h,
    );
    let ask = rect(input.right + 8, input.top, 58, 32);
    let regenerate = rect(ask.right + 8, input.top, regenerate_w, 32);
    let suggestion_top = input.bottom + 8;
    let suggestions = if compact_suggestions {
        let suggestion_w = ((right_width - 8) / 2).max(120);
        [
            rect(right_left, suggestion_top, suggestion_w, 26),
            rect(
                right_left + suggestion_w + 8,
                suggestion_top,
                suggestion_w,
                26,
            ),
            rect(right_left, suggestion_top + 34, suggestion_w, 26),
            rect(
                right_left + suggestion_w + 8,
                suggestion_top + 34,
                suggestion_w,
                26,
            ),
        ]
    } else {
        let suggestion_w = ((right_width - 24) / 4).max(110);
        [
            rect(right_left, suggestion_top, suggestion_w, 26),
            rect(
                right_left + (suggestion_w + 8),
                suggestion_top,
                suggestion_w,
                26,
            ),
            rect(
                right_left + ((suggestion_w + 8) * 2),
                suggestion_top,
                suggestion_w,
                26,
            ),
            rect(
                right_left + ((suggestion_w + 8) * 3),
                suggestion_top,
                suggestion_w,
                26,
            ),
        ]
    };
    let body = rect(
        right_left,
        popup.top + 50,
        right_width,
        input.top - popup.top - 60,
    );
    let pin = rect(history.left + 8, history.bottom - 84, 58, 26);
    let clear_one = rect(pin.right + 6, pin.top, 78, 26);
    let clear_all = rect(
        history.left + 8,
        pin.bottom + 6,
        history.right - history.left - 16,
        26,
    );
    AiLayout {
        popup,
        drag,
        resize,
        history,
        body,
        input,
        ask,
        regenerate,
        pin,
        clear_one,
        clear_all,
        smaller,
        larger,
        fit,
        suggestions,
    }
}

fn ai_session_row_rect(ai: &AiLayout, row: usize) -> RECT {
    rect(
        ai.history.left + 8,
        ai.history.top + 34 + (row as i32 * 32),
        ai.history.right - ai.history.left - 16,
        29,
    )
}

fn ai_session_row_capacity(ai: &AiLayout) -> usize {
    ((ai.history.bottom - ai.history.top - 128) / 32).max(0) as usize
}

fn launcher_layout(client: RECT) -> LauncherLayout {
    let client_w = (client.right - client.left).max(1);
    let client_h = (client.bottom - client.top).max(1);
    let w = (client_w - 48).clamp(420, 680);
    let h = 180.min((client_h - 48).max(150));
    let left = client.left + ((client_w - w) / 2).max(12);
    let top = client.top + ((client_h - h) / 2).max(12);
    let popup = rect(left, top, w, h);
    let close = rect(popup.right - 48, popup.top + 12, 30, 24);
    let input = rect(popup.left + 16, popup.top + 82, w - 32, 30);
    let start = rect(popup.right - 184, popup.bottom - 46, 78, 28);
    let cancel = rect(popup.right - 96, popup.bottom - 46, 78, 28);
    LauncherLayout {
        popup,
        input,
        start,
        cancel,
        close,
    }
}

fn sentinel_settings_layout(client: RECT) -> SentinelSettingsLayout {
    let client_w = (client.right - client.left).max(1);
    let client_h = (client.bottom - client.top).max(1);
    let width = 1020.min((client_w - 32).max(760));
    let height = 680.min((client_h - 28).max(600));
    let left = client.left + ((client_w - width) / 2).max(8);
    let top = client.top + ((client_h - height) / 2).max(8);
    let popup = rect(left, top, width, height);
    let close = rect(popup.right - 46, popup.top + 11, 28, 26);
    let providers = rect(popup.left + 14, popup.top + 58, 270, height - 74);
    let content_left = providers.right + 20;
    let content_width = popup.right - content_left - 18;
    let model_input = rect(content_left, popup.top + 164, content_width, 32);
    let choice_width = ((content_width - 24) / 4).max(92);
    let model_choices = [
        rect(content_left, popup.top + 226, choice_width, 44),
        rect(
            content_left + choice_width + 8,
            popup.top + 226,
            choice_width,
            44,
        ),
        rect(
            content_left + (choice_width + 8) * 2,
            popup.top + 226,
            choice_width,
            44,
        ),
        rect(
            content_left + (choice_width + 8) * 3,
            popup.top + 226,
            choice_width,
            44,
        ),
    ];
    let clear_width = 150;
    let key_input = rect(
        content_left,
        popup.top + 350,
        content_width - clear_width - 8,
        34,
    );
    let clear_key = rect(key_input.right + 8, key_input.top, clear_width, 34);
    let cancel = rect(popup.right - 106, popup.bottom - 48, 88, 30);
    let save = rect(cancel.left - 202, popup.bottom - 48, 192, 30);
    SentinelSettingsLayout {
        popup,
        close,
        providers,
        model_input,
        model_choices,
        key_input,
        clear_key,
        save,
        cancel,
    }
}

fn sentinel_provider_row(layout: &SentinelSettingsLayout, index: usize) -> RECT {
    rect(
        layout.providers.left + 1,
        layout.providers.top + 36 + index as i32 * 31,
        layout.providers.right - layout.providers.left - 2,
        31,
    )
}

fn mouse_xy(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam & 0xffff) as i16 as i32;
    let y = ((lparam >> 16) & 0xffff) as i16 as i32;
    (x, y)
}

fn table_content_width() -> i32 {
    1708
}

fn rect(left: i32, top: i32, width: i32, height: i32) -> RECT {
    RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    }
}

fn inset(r: RECT, x: i32, y: i32) -> RECT {
    RECT {
        left: r.left + x,
        top: r.top + y,
        right: r.right - x,
        bottom: r.bottom - y,
    }
}

fn intersect_rect(a: RECT, b: RECT) -> Option<RECT> {
    let result = RECT {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    };
    if result.left < result.right && result.top < result.bottom {
        Some(result)
    } else {
        None
    }
}

const C_BG: COLORREF = 0x0f0a05;
const C_PANEL: COLORREF = 0x18110a;
const C_MENU_BG: COLORREF = 0x100b06;
const C_MENU_ACTIVE: COLORREF = 0x2a1a09;
const C_TABLE: COLORREF = 0x140e08;
const C_SIDE: COLORREF = 0x17100a;
const C_MODAL: COLORREF = 0x1d1308;
const C_HEADER: COLORREF = 0x241608;
const C_BORDER: COLORREF = 0x684014;
const C_GRID: COLORREF = 0x352414;
const C_TEXT: COLORREF = 0xd6f5d4;
const C_MUTED: COLORREF = 0x86a386;
const C_GREEN: COLORREF = 0x61ff4f;
const C_CYAN: COLORREF = 0xffe35d;
const C_ROW_B: COLORREF = 0x191008;
const C_SAFE_TEXT: COLORREF = 0x74ff66;
const C_SAFE_BG_A: COLORREF = 0x102416;
const C_SAFE_BG_B: COLORREF = 0x142a19;
const C_WARN_TEXT: COLORREF = 0x48c6ff;
const C_CAUTION_BG: COLORREF = 0x0c2848;
const C_BLOCK_BG: COLORREF = 0x200909;
const C_DANGER_BG: COLORREF = 0x171242;
const C_BLOCK_TEXT: COLORREF = 0x7777ff;
const C_SELECT_BG: COLORREF = 0x5b3600;
const C_SELECT_TEXT: COLORREF = 0xffffff;

unsafe fn create_font(height: i32, weight: i32) -> isize {
    let face = wide("Consolas");
    CreateFontW(
        -height,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET as u32,
        OUT_DEFAULT_PRECIS as u32,
        0,
        DEFAULT_QUALITY as u32,
        FF_DONTCARE as u32,
        face.as_ptr(),
    ) as isize
}

unsafe fn select(hdc: HDC, font: isize, color: COLORREF) {
    SelectObject(hdc, font as HGDIOBJ);
    SetTextColor(hdc, color);
}

unsafe fn fill(hdc: HDC, r: RECT, color: COLORREF) {
    let brush = CreateSolidBrush(color);
    FillRect(hdc, &r, brush);
    DeleteObject(brush as HGDIOBJ);
}

unsafe fn frame(hdc: HDC, r: RECT, color: COLORREF) {
    fill(hdc, rect(r.left, r.top, r.right - r.left, 1), color);
    fill(hdc, rect(r.left, r.bottom - 1, r.right - r.left, 1), color);
    fill(hdc, rect(r.left, r.top, 1, r.bottom - r.top), color);
    fill(hdc, rect(r.right - 1, r.top, 1, r.bottom - r.top), color);
}

unsafe fn line_bottom(hdc: HDC, r: RECT, color: COLORREF) {
    fill(hdc, rect(r.left, r.bottom - 1, r.right - r.left, 1), color);
}

unsafe fn line_left(hdc: HDC, r: RECT, color: COLORREF) {
    fill(hdc, rect(r.left, r.top, 1, r.bottom - r.top), color);
}

unsafe fn draw_text(hdc: HDC, text: &str, mut r: RECT, flags: u32) {
    let text = wide(text);
    DrawTextW(hdc, text.as_ptr(), -1, &mut r, flags);
}

unsafe fn draw_multiline(hdc: HDC, text: &str, r: RECT, line_h: i32) {
    let mut y = r.top;
    let max_chars = (((r.right - r.left) / 8).max(24)) as usize;
    'lines: for source in text.replace('\n', "\r").split('\r') {
        let mut rest = source.trim_end().to_string();
        if rest.is_empty() {
            y += line_h;
            continue;
        }
        while !rest.is_empty() {
            if y + line_h > r.bottom {
                break 'lines;
            }
            let (line, next) = wrapped_segment(&rest, max_chars);
            draw_text(
                hdc,
                &line,
                rect(r.left, y, r.right - r.left, line_h),
                DT_LEFT | DT_SINGLELINE,
            );
            rest = next;
            y += line_h;
        }
    }
}

fn ai_message_colors(speaker: &str) -> (COLORREF, COLORREF) {
    match speaker {
        SPEAKER_USER => (C_GREEN, C_TEXT),
        SPEAKER_LOCAL => (C_WARN_TEXT, C_MUTED),
        "Status" => (C_WARN_TEXT, C_MUTED),
        _ => (C_CYAN, C_TEXT),
    }
}

fn push_ai_block(
    lines: &mut Vec<AiRenderLine>,
    speaker: &str,
    text: &str,
    header_color: COLORREF,
    body_color: COLORREF,
    max_chars: usize,
) {
    lines.push(AiRenderLine {
        text: speaker.to_string(),
        color: header_color,
        bold: true,
        marker: Some(header_color),
    });
    for segment in wrapped_lines(text.trim(), max_chars) {
        lines.push(AiRenderLine {
            text: segment,
            color: body_color,
            bold: false,
            marker: None,
        });
    }
    lines.push(AiRenderLine {
        text: String::new(),
        color: C_MUTED,
        bold: false,
        marker: None,
    });
}

fn wrapped_lines(value: &str, max_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for source in value.replace('\n', "\r").split('\r') {
        let mut rest = source.trim_end().to_string();
        if rest.is_empty() {
            lines.push(String::new());
            continue;
        }
        while !rest.is_empty() {
            let (line, next) = wrapped_segment(&rest, max_chars);
            lines.push(line);
            rest = next;
        }
    }
    lines
}

fn wrapped_segment(value: &str, max_chars: usize) -> (String, String) {
    if value.chars().count() <= max_chars {
        return (value.to_string(), String::new());
    }

    let mut hard_split = value.len();
    let mut last_space = None;
    for (count, (idx, ch)) in value.char_indices().enumerate() {
        if count >= max_chars {
            hard_split = idx;
            break;
        }
        if ch.is_whitespace() {
            last_space = Some(idx);
        }
    }

    let split = last_space.filter(|idx| *idx > 0).unwrap_or(hard_split);
    let line = value[..split].trim_end().to_string();
    let next = value[split..].trim_start().to_string();
    (line, next)
}

fn byte_index_at_char(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

unsafe fn draw_button(hdc: HDC, r: RECT, label: &str, active: bool) {
    fill(hdc, r, if active { 0x4e2e00 } else { 0x1b1208 });
    frame(hdc, r, if active { C_CYAN } else { C_BORDER });
    SetTextColor(hdc, if active { C_CYAN } else { C_TEXT });
    draw_text(
        hdc,
        label,
        inset(r, 8, 0),
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}

unsafe fn draw_ai_size_icon(hdc: HDC, r: RECT, plus: bool, active: bool) {
    if active {
        fill(hdc, r, C_MENU_ACTIVE);
    }
    let color = if active { C_CYAN } else { C_TEXT };
    let center_x = (r.left + r.right) / 2;
    let center_y = (r.top + r.bottom) / 2;
    fill(hdc, rect(center_x - 7, center_y, 15, 1), color);
    if plus {
        fill(hdc, rect(center_x, center_y - 7, 1, 15), color);
    }
}

unsafe fn draw_sentinel_model_choice(
    hdc: HDC,
    r: RECT,
    model: &str,
    tier: SentinelModelTier,
    active: bool,
    body_font: isize,
    bold_font: isize,
) {
    fill(hdc, r, if active { C_SELECT_BG } else { C_TABLE });
    frame(hdc, r, tier.color());
    fill(hdc, rect(r.left, r.top, r.right - r.left, 2), tier.color());
    select(hdc, body_font, tier.color());
    draw_text(
        hdc,
        tier.label(),
        rect(r.left + 7, r.top + 3, r.right - r.left - 14, 14),
        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
    select(hdc, if active { bold_font } else { body_font }, C_TEXT);
    draw_text(
        hdc,
        model,
        rect(r.left + 7, r.top + 19, r.right - r.left - 14, 20),
        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}

unsafe fn draw_search(hdc: HDC, r: RECT, clear: RECT, value: &str, focus: bool) {
    draw_text_box(hdc, r, value, "search pid/name/type/path...", focus);
    if !value.is_empty() {
        fill(hdc, clear, if focus { C_MENU_ACTIVE } else { C_TABLE });
        frame(hdc, clear, if focus { C_CYAN } else { C_BORDER });
        SetTextColor(hdc, C_CYAN);
        draw_text(
            hdc,
            "X",
            inset(clear, 6, 0),
            DT_LEFT | DT_SINGLELINE | DT_VCENTER,
        );
    }
}

unsafe fn draw_ai_input(
    hdc: HDC,
    r: RECT,
    value: &str,
    focus: bool,
    all_selected: bool,
    cursor: usize,
) {
    fill(hdc, r, if all_selected { C_SELECT_BG } else { 0x0b0804 });
    frame(
        hdc,
        r,
        if all_selected {
            C_WARN_TEXT
        } else if focus {
            C_CYAN
        } else {
            C_BORDER
        },
    );
    let chars = value.chars().count();
    let max_chars = (((r.right - r.left - 16) / 8).max(12)) as usize;
    let line_capacity = (((r.bottom - r.top - 18) / 17).max(1)) as usize;
    let mut display = if value.is_empty() {
        "ask a follow-up question...".to_string()
    } else {
        value.to_string()
    };
    if focus && !value.is_empty() && !all_selected {
        let at = byte_index_at_char(&display, cursor.min(chars));
        display.insert(at, '|');
    }
    let lines = wrapped_lines(&display, max_chars);
    let start = lines.len().saturating_sub(line_capacity);
    SetTextColor(
        hdc,
        if value.is_empty() {
            C_MUTED
        } else if all_selected {
            C_SELECT_TEXT
        } else {
            C_GREEN
        },
    );
    for (row, line) in lines.iter().skip(start).take(line_capacity).enumerate() {
        draw_text(
            hdc,
            line,
            rect(
                r.left + 8,
                r.top + 4 + row as i32 * 17,
                r.right - r.left - 16,
                17,
            ),
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
    }
    SetTextColor(hdc, C_MUTED);
    draw_text(
        hdc,
        &format!("{}/4000  Shift+Enter: new line", chars),
        rect(r.left + 8, r.bottom - 17, r.right - r.left - 16, 15),
        DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS,
    );
}

unsafe fn draw_text_box(hdc: HDC, r: RECT, value: &str, placeholder: &str, focus: bool) {
    fill(hdc, r, 0x0b0804);
    frame(hdc, r, if focus { C_CYAN } else { C_BORDER });
    let text = if value.is_empty() { placeholder } else { value };
    SetTextColor(hdc, if value.is_empty() { C_MUTED } else { C_GREEN });
    draw_text(
        hdc,
        text,
        inset(r, 8, 0),
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

fn from_wide_z(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinel_provider_matrix_has_required_backends() {
        let cli_count = SENTINEL_PROVIDERS
            .iter()
            .filter(|provider| !provider.is_api())
            .count();
        let api_count = SENTINEL_PROVIDERS
            .iter()
            .filter(|provider| provider.is_api())
            .count();
        assert_eq!(cli_count, 3);
        assert!(api_count >= 10);
    }

    #[test]
    fn sentinel_provider_ids_are_unique_and_models_are_present() {
        let mut ids = BTreeSet::new();
        for provider in SENTINEL_PROVIDERS {
            assert!(ids.insert(provider.id));
            assert!(!provider.models.is_empty());
            assert!(provider.models.iter().all(|model| !model.trim().is_empty()));
        }
    }

    #[test]
    fn free_or_included_models_are_first_and_premium_is_never_default() {
        for provider in SENTINEL_PROVIDERS {
            let tiers = (0..provider.models.len())
                .map(|index| sentinel_model_tier(provider, index))
                .collect::<Vec<_>>();
            if tiers
                .iter()
                .any(|tier| matches!(tier, SentinelModelTier::Included | SentinelModelTier::Free))
            {
                assert!(matches!(
                    tiers[0],
                    SentinelModelTier::Included | SentinelModelTier::Free
                ));
            }
            assert_ne!(tiers[0], SentinelModelTier::Premium);
        }
    }
}
