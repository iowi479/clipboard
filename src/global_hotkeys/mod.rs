// This Implementation is based on the Inputbot crate
// https://github.com/obv-mikhail/InputBot
//
// Since the original crate gives way more functionality than we need, we will only take what we
// need and tailor it to our needs.
// Additionally the original crate is cross platform, but we will only focus on Windows.

mod keyboard;
use keyboard::KeyboardKey;

use std::{
    ffi::{c_int, c_ulong},
    mem::{size_of, MaybeUninit},
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        mpsc, Mutex,
    },
    thread::sleep,
    thread::spawn,
    time::Duration,
};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, GetKeyState, MapVirtualKeyW, SendInput, INPUT, INPUT_0,
            INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
            MAP_VIRTUAL_KEY_TYPE, VIRTUAL_KEY,
        },
        WindowsAndMessaging::{
            CallNextHookEx, GetMessageW, KillTimer, SetTimer, SetWindowsHookExW,
            UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WINDOWS_HOOK_ID,
            WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
        },
    },
};

use crate::file_handlers::ClipboardAction;

pub struct KeyboardListener {
    // stores the hook for our event so it can be unset later
    keyboard_hhock: AtomicPtr<HHOOK>,
    timer_id: Option<c_ulong>,
}

// these keep track of the state of the control keys
static L_CONTROL_PRESSED: AtomicBool = AtomicBool::new(false);
static R_CONTROL_PRESSED: AtomicBool = AtomicBool::new(false);
pub static LOADED_CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);
pub static CLIPBOARD_ACTION_SENDER: Mutex<Option<mpsc::Sender<ClipboardAction>>> = Mutex::new(None);

impl KeyboardListener {
    pub fn new() -> Self {
        Self {
            keyboard_hhock: AtomicPtr::default(),
            timer_id: None,
        }
    }
    pub fn handle_input_events(&mut self) {
        Self::set_hook(WH_KEYBOARD_LL, &self.keyboard_hhock, keybd_proc);

        let timer_id = unsafe { SetTimer(None, 0, 100, None) };
        self.timer_id = Some(timer_id as c_ulong);

        // HANDLE_EVENTS.store(true, Ordering::Relaxed);
        loop {
            let mut msg: MSG = unsafe { MaybeUninit::zeroed().assume_init() };
            unsafe {
                let _ = GetMessageW(&mut msg, None, 0, 0);
            };
        }
    }
}

impl Drop for KeyboardListener {
    fn drop(&mut self) {
        if let Some(timer_id) = self.timer_id {
            let _ = unsafe { KillTimer(None, timer_id as usize) };
        }

        // expected that KEYBD_HHOOK is alreadt set. Dont know what happens if we unset the default
        // ptr. Probably a unwrap panic... in unsets unsafe block
        Self::unset_hook(&self.keyboard_hhock);
    }
}

impl KeyboardKey {
    /// Returns true if a given `KeyboardKey` is currently pressed (in the down position).
    pub fn is_pressed(self) -> bool {
        (unsafe { GetAsyncKeyState(u64::from(self) as i32) } >> 15) != 0
    }

    /// Presses a given `KeyboardKey`. Note: this means the key will remain in the down
    /// position. You must manually call release to create a full 'press'.
    pub fn press(self) {
        KeyboardListener::send_keyboard_input(KEYEVENTF_SCANCODE, self);
    }

    /// Releases a given `KeyboardKey`. This means the key would be in the up position.
    pub fn release(self) {
        KeyboardListener::send_keyboard_input(KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP, self);
    }

    /// Returns true if a `KeyboardKey` which supports toggling (ScrollLock, NumLock,
    /// CapsLock) is on.
    pub fn is_toggled(self) -> bool {
        unsafe { GetKeyState(u64::from(self) as i32) & 15 != 0 }
    }
}

unsafe extern "system" fn keybd_proc(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let event_type = w_param.0 as u32;
    let key_code = u64::from((*(l_param.0 as *const KBDLLHOOKSTRUCT)).vkCode);
    let key = KeyboardKey::from(key_code);

    #[allow(non_snake_case)]
    if event_type == WM_KEYDOWN || event_type == WM_SYSKEYDOWN {
        match key {
            KeyboardKey::LControlKey => {
                L_CONTROL_PRESSED.store(true, Ordering::Relaxed);
                // TODO: maybe load data for clipboard on ctrl press to be able to instantly paste
                // it on V so no other hotkey is needed
                CLIPBOARD_ACTION_SENDER
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .send(ClipboardAction::TryLoad);
            }
            KeyboardKey::RControlKey => {
                R_CONTROL_PRESSED.store(true, Ordering::Relaxed);
                CLIPBOARD_ACTION_SENDER
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .send(ClipboardAction::TryLoad);
            }

            KeyboardKey::CKey => {
                if L_CONTROL_PRESSED.load(Ordering::Relaxed)
                    || R_CONTROL_PRESSED.load(Ordering::Relaxed)
                {
                    // TODO: save local clipboard and reset already loaded stuff
                    println!("Ctrl + C pressed");
                    spawn(move || {
                        sleep(Duration::from_secs(1));
                        let content = clipboard_win::get_clipboard(clipboard_win::formats::Unicode)
                            .expect("To get clipboard");

                        CLIPBOARD_ACTION_SENDER
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .send(ClipboardAction::Store(content));
                    });
                }
            }
            KeyboardKey::VKey => {
                if L_CONTROL_PRESSED.load(Ordering::Relaxed)
                    || R_CONTROL_PRESSED.load(Ordering::Relaxed)
                {
                    let content = LOADED_CLIPBOARD.lock().unwrap().clone();
                    if let Some(content) = content {
                        clipboard_win::set_clipboard(clipboard_win::formats::Unicode, content)
                            .expect("To set clipboard");
                    }
                    println!("Ctrl + V pressed");
                }
            }
            _ => {}
        }
    } else if event_type == WM_KEYUP || event_type == WM_SYSKEYUP {
        match key {
            KeyboardKey::LControlKey => {
                L_CONTROL_PRESSED.store(false, Ordering::Relaxed);
            }
            KeyboardKey::RControlKey => {
                R_CONTROL_PRESSED.store(false, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    return CallNextHookEx(None, code, w_param, l_param);
}

impl KeyboardListener {
    fn set_hook(
        hook_id: WINDOWS_HOOK_ID,
        hook_ptr: &AtomicPtr<HHOOK>,
        hook_proc: unsafe extern "system" fn(c_int, WPARAM, LPARAM) -> LRESULT,
    ) {
        hook_ptr.store(
            unsafe { &mut SetWindowsHookExW(hook_id, Some(hook_proc), None, 0).unwrap() },
            Ordering::Relaxed,
        );
    }

    fn unset_hook(hook_ptr: &AtomicPtr<HHOOK>) {
        if !hook_ptr.load(Ordering::Relaxed).is_null() {
            unsafe { UnhookWindowsHookEx(*hook_ptr.load(Ordering::Relaxed)).unwrap() };
            hook_ptr.store(null_mut(), Ordering::Relaxed);
        }
    }

    fn send_keyboard_input(flags: KEYBD_EVENT_FLAGS, key_code: KeyboardKey) {
        let keybd: KEYBDINPUT = unsafe {
            KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: MapVirtualKeyW(u64::from(key_code) as u32, MAP_VIRTUAL_KEY_TYPE(0)) as u16,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            }
        };

        // We need an "empty" winapi struct to union-ize
        let mut input_u: INPUT_0 = unsafe { std::mem::zeroed() };

        input_u.ki = keybd;

        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: input_u,
        };

        unsafe { SendInput(&[input], size_of::<INPUT>() as c_int) };
    }
}
