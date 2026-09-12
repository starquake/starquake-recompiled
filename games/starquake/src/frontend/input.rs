//! Keyboard mapping: the host keyboard to the Spectrum's key matrix, and
//! the arrow keys to a Kempston joystick.

use starquake::controls::Input;
use winit::keyboard::KeyCode;

/// Spectrum matrix position (half-row, bit) for a host key.
fn matrix(key: KeyCode) -> &'static [(usize, u8)] {
    use KeyCode::*;
    match key {
        ShiftLeft | ShiftRight => &[(0, 0)],
        KeyZ => &[(0, 1)],
        KeyX => &[(0, 2)],
        KeyC => &[(0, 3)],
        KeyV => &[(0, 4)],
        KeyA => &[(1, 0)],
        KeyS => &[(1, 1)],
        KeyD => &[(1, 2)],
        KeyF => &[(1, 3)],
        KeyG => &[(1, 4)],
        KeyQ => &[(2, 0)],
        KeyW => &[(2, 1)],
        KeyE => &[(2, 2)],
        KeyR => &[(2, 3)],
        KeyT => &[(2, 4)],
        Digit1 => &[(3, 0)],
        Digit2 => &[(3, 1)],
        Digit3 => &[(3, 2)],
        Digit4 => &[(3, 3)],
        Digit5 => &[(3, 4)],
        Digit0 => &[(4, 0)],
        Digit9 => &[(4, 1)],
        Digit8 => &[(4, 2)],
        Digit7 => &[(4, 3)],
        Digit6 => &[(4, 4)],
        KeyP => &[(5, 0)],
        KeyO => &[(5, 1)],
        KeyI => &[(5, 2)],
        KeyU => &[(5, 3)],
        KeyY => &[(5, 4)],
        Enter => &[(6, 0)],
        KeyL => &[(6, 1)],
        KeyK => &[(6, 2)],
        KeyJ => &[(6, 3)],
        KeyH => &[(6, 4)],
        Space => &[(7, 0)],
        ControlLeft | ControlRight => &[(7, 1)],
        KeyM => &[(7, 2)],
        KeyN => &[(7, 3)],
        KeyB => &[(7, 4)],
        // Delete is Caps Shift + 0 on a Spectrum.
        Backspace => &[(0, 0), (4, 0)],
        // The Spectrum's cursor keys are 5, 6, 7 and 8 — the arrows are
        // printed on those very keys — so the host arrows press them as well
        // as moving the joystick below. That makes them work in the cursor
        // control method, and means they type those digits just as the real
        // keys do: an arrow at the title screen picks that menu option.
        ArrowLeft => &[(3, 4)],
        ArrowDown => &[(4, 4)],
        ArrowUp => &[(4, 3)],
        ArrowRight => &[(4, 2)],
        _ => &[],
    }
}

/// Kempston joystick bit for a host key.
///
/// A Kempston interface is a joystick port: the game reads five bits and
/// cannot tell what moved them, so pointing host keys at them is invisible
/// to it. Only keys the Spectrum itself has no use for are used here —
/// Space in particular is a Spectrum key (it is the pause key the game
/// ships with), so it stays out of this and does exactly what it does on
/// the real machine.
fn kempston(key: KeyCode) -> u8 {
    use KeyCode::*;
    match key {
        ArrowRight => 0x01,
        ArrowLeft => 0x02,
        ArrowDown => 0x04,
        ArrowUp => 0x08,
        // Alt is awkward on macOS, so a couple of spare keys fire too.
        AltLeft | AltRight | SuperRight | Period | Comma => 0x10,
        _ => 0,
    }
}

pub fn apply(input: &mut Input, key: KeyCode, pressed: bool) {
    for &(row, bit) in matrix(key) {
        if pressed {
            input.keys[row] &= !(1 << bit);
        } else {
            input.keys[row] |= 1 << bit;
        }
    }
    let k = kempston(key);
    if pressed {
        input.kempston |= k;
    } else {
        input.kempston &= !k;
    }
}
