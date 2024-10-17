// This Implementation is based on the Inputbot crate
// https://github.com/obv-mikhail/InputBot
//
// Since the original crate gives way more functionality than we need, we will only take what we
// need and tailor it to our needs.
// Additionally the original crate is cross platform, but we will only focus on Windows.

mod keyboard;

use crate::file_handlers::ClipboardAction;
use crate::logfile::{log, log_and_panic};
use keyboard::KeyboardKey;
use std::ffi::{c_int, c_ulong};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{mpsc, Mutex};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, KillTimer, SetTimer, SetWindowsHookExW, UnhookWindowsHookEx,
    HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WINDOWS_HOOK_ID, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

// these keep track of the state of the control keys
static L_CONTROL_PRESSED: AtomicBool = AtomicBool::new(false);
static R_CONTROL_PRESSED: AtomicBool = AtomicBool::new(false);
pub static LOADED_CLIPBOARD: Mutex<Option<String>> = Mutex::new(None);
pub static CLIPBOARD_ACTION_SENDER: Mutex<Option<mpsc::Sender<ClipboardAction>>> = Mutex::new(None);

pub struct KeyboardListener {
    // stores the hook for our event so it can be unset later
    keyboard_hhock: AtomicPtr<HHOOK>,
    timer_id: Option<c_ulong>,
}

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

        loop {
            let mut msg: MSG = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
            unsafe {
                let _ = GetMessageW(&mut msg, None, 0, 0);
            };
        }
    }

    /// register the hook into the win-api
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

    /// unregisters the hook from the win-api
    fn unset_hook(hook_ptr: &AtomicPtr<HHOOK>) {
        if !hook_ptr.load(Ordering::Relaxed).is_null() {
            unsafe { UnhookWindowsHookEx(*hook_ptr.load(Ordering::Relaxed)).unwrap() };
            hook_ptr.store(std::ptr::null_mut(), Ordering::Relaxed);
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

/// sets the sender for the clipboard actions
/// this sender will send the actions activated by the hotkeys
pub fn set_action_sender(sender: mpsc::Sender<ClipboardAction>) {
    CLIPBOARD_ACTION_SENDER
        .lock()
        .unwrap_or_else(|e| {
            let _ = &format!("could not aquire action sender {}", e);
            unreachable!();
        })
        .replace(sender);
}

/// sends the action to the file_handler via the established channel
fn send_action(action: ClipboardAction) {
    match CLIPBOARD_ACTION_SENDER.lock() {
        Ok(sender) => match sender.as_ref() {
            None => {
                log_and_panic("tried to use action-sender, but it was not set");
            }
            Some(sender) => {
                if let Err(e) = sender.send(action) {
                    log_and_panic(&format!("could not send action to the file-handler: {}", e));
                }
            }
        },
        Err(e) => {
            log_and_panic(&format!(
                "could not aquire lock to the action-sender.\nIt seems like the channel broke... {}",
                e
            ));
        }
    }
}

/// handler for the win-api
unsafe extern "system" fn keybd_proc(code: c_int, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let event_type = w_param.0 as u32;
    let key_code = u64::from((*(l_param.0 as *const KBDLLHOOKSTRUCT)).vkCode);
    let key = KeyboardKey::from(key_code);

    #[allow(non_snake_case)]
    if event_type == WM_KEYDOWN || event_type == WM_SYSKEYDOWN {
        match key {
            KeyboardKey::LControlKey => {
                L_CONTROL_PRESSED.store(true, Ordering::Relaxed);
                send_action(ClipboardAction::TryLoad);
            }
            KeyboardKey::RControlKey => {
                R_CONTROL_PRESSED.store(true, Ordering::Relaxed);
                send_action(ClipboardAction::TryLoad);
            }

            KeyboardKey::CKey => {
                if L_CONTROL_PRESSED.load(Ordering::Relaxed)
                    || R_CONTROL_PRESSED.load(Ordering::Relaxed)
                {
                    // check if atleast one crtl is currently pressed
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        let content = clipboard_win::get_clipboard(clipboard_win::formats::Unicode)
                            .unwrap_or_else(|e| {
                                log_and_panic(&format!("could not get clipboard: {}", e));
                                unreachable!();
                            });

                        log(&format!("Copied <{}>\n", &content));
                        send_action(ClipboardAction::Store(content));
                    });
                }
            }
            KeyboardKey::VKey => {
                if L_CONTROL_PRESSED.load(Ordering::Relaxed)
                    || R_CONTROL_PRESSED.load(Ordering::Relaxed)
                {
                    // check if atleast one crtl is currently pressed
                    let content = LOADED_CLIPBOARD.lock().unwrap_or_else(|e| {
                        let _ = &format!("Could not aquire lock for the loaded clipboard value. This means the loading of a value failed or is still locking it... {}", e);
                        unreachable!();
                    }).clone();

                    if let Some(content) = content {
                        log(&format!("Insertet <{}>\n", &content));
                        if let Err(e) =
                            clipboard_win::set_clipboard(clipboard_win::formats::Unicode, content)
                        {
                            log_and_panic(&format!("could not set clipboard: {}", e));
                        }
                    }
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
