use crate::Emulator;

pub enum KeyState {
    Up,
    Down,
}

pub fn process_key(emulator: &mut Emulator, key: char, state: KeyState) {
    let key_value = match state {
        KeyState::Up => false,
        KeyState::Down => true,
    };

    match key {
        '1' => emulator.keypad[0x1] = key_value,
        '2' => emulator.keypad[0x2] = key_value,
        '3' => emulator.keypad[0x3] = key_value,
        '4' => emulator.keypad[0xC] = key_value,

        'q' => emulator.keypad[0x4] = key_value,
        'w' => emulator.keypad[0x5] = key_value,
        'e' => emulator.keypad[0x6] = key_value,
        'r' => emulator.keypad[0xD] = key_value,

        'a' => emulator.keypad[0x7] = key_value,
        's' => emulator.keypad[0x8] = key_value,
        'd' => emulator.keypad[0x9] = key_value,
        'f' => emulator.keypad[0xE] = key_value,

        'z' => emulator.keypad[0xA] = key_value,
        'x' => emulator.keypad[0x0] = key_value,
        'c' => emulator.keypad[0xB] = key_value,
        'v' => emulator.keypad[0xF] = key_value,
        _ => {}
    }
}
