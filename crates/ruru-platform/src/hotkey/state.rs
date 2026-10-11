use global_hotkey::GlobalHotKeyManager;

pub struct MacosHotkey {
    pub(super) _manager: GlobalHotKeyManager,
    pub(super) id: u32,
}
