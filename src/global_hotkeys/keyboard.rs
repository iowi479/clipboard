#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum KeyboardKey {
    CKey,
    VKey,
    LControlKey,
    RControlKey,
    OtherKey(u64),
}

impl From<KeyboardKey> for u64 {
    // https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
    fn from(key: KeyboardKey) -> u64 {
        match key {
            KeyboardKey::CKey => 0x43,
            KeyboardKey::VKey => 0x56,
            KeyboardKey::LControlKey => 0xA2,
            KeyboardKey::RControlKey => 0xA3,
            KeyboardKey::OtherKey(code) => code,
        }
    }
}

impl From<u64> for KeyboardKey {
    // https://docs.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes?redirectedfrom=MSDN
    fn from(code: u64) -> KeyboardKey {
        match code {
            0x43 => KeyboardKey::CKey,
            0x56 => KeyboardKey::VKey,
            0xA2 => KeyboardKey::LControlKey,
            0xA3 => KeyboardKey::RControlKey,
            _ => KeyboardKey::OtherKey(code),
        }
    }
}
