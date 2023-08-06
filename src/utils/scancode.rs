//! A set of more or less common scan codes, that represents physical key on a keyboard. See [`Scancode`] docs for more
//! info and usage examples.

/// A set of more or less common scan codes, that represents a physical key on a keyboard. It could be used if you need
/// to check for a key press of a physical button, and don't care about current keyboard layout of your OS. The engine
/// provides two ways of getting key's state - via scan codes and key codes. What's the difference between two? Scan
/// code, as was stated earlier, corresponds to a _physical_ location of a key, while key code may be different depending
/// on the keyboard layout of your OS. For example on QWERTY keyboard layout the `W` key will have the same key and scan
/// codes, however on AZERTY keyboard layout the `W` key will have `Z` key code and `W` scan code. As you can see it is
/// layout-independent.
///
/// ## How to use
///
/// Which one to use and when? Use key codes if you care about user's keyboard layout (for example in GUI). Use scan codes
/// if you don't care about keyboard layout (for example in games).
///
/// ## Examples
///
/// For example, a typical movement handling via WASD keys could be handled like so:
///
/// ```rust
/// # use fyrox::{event::KeyboardInput, utils::scancode::Scancode};
/// #
/// fn movement(keyboard_input: &KeyboardInput) {
///     if Scancode::W == keyboard_input.scancode {
///         // Move forward.
///     } else if Scancode::S == keyboard_input.scancode {
///         // Move backward.
///     } else if Scancode::A == keyboard_input.scancode {
///         // Move left.
///     } else if Scancode::D == keyboard_input.scancode {
///         // Move right.
///     }
/// }
/// ```
///
/// Note: It uses reversed comparison for scancodes (`scancode == u32`) because of technical reasons, it could be done in
/// a normal way by `keyboard_input.scancode == Scancode::W as u32` which adds a typecasting.
///
/// ## Source
///
/// Actual scan codes were taken from USB HID docs - https://www.usb.org/sites/default/files/documents/hut1_12v2.pdf. Some
/// inspiration was taken from SDL2 source code and docs.
#[derive(Copy, Clone, Debug, Hash, Eq)]
#[repr(u32)]
#[allow(missing_docs)] // TODO: This should be documented some day.
pub enum Scancode {
    A = 4,
    B = 5,
    C = 6,
    D = 7,
    E = 8,
    F = 9,
    G = 10,
    H = 11,
    I = 12,
    J = 13,
    K = 14,
    L = 15,
    M = 16,
    N = 17,
    O = 18,
    P = 19,
    Q = 20,
    R = 21,
    S = 22,
    T = 23,
    U = 24,
    V = 25,
    W = 26,
    X = 27,
    Y = 28,
    Z = 29,
    Key1 = 30,
    Key2 = 31,
    Key3 = 32,
    Key4 = 33,
    Key5 = 34,
    Key6 = 35,
    Key7 = 36,
    Key8 = 37,
    Key9 = 38,
    Key0 = 39,
    Return = 40,
    Escape = 41,
    Backspace = 42,
    Tab = 43,
    Space = 44,
    Minus = 45,
    Equals = 46,
    LeftBracket = 47,
    RightBracket = 48,
    BackSlash = 49,
    NonusHash = 50,
    Semicolon = 51,
    Apostrophe = 52,
    Grave = 53,
    Comma = 54,
    Period = 55,
    Slash = 56,
    CapsLock = 57,
    F1 = 58,
    F2 = 59,
    F3 = 60,
    F4 = 61,
    F5 = 62,
    F6 = 63,
    F7 = 64,
    F8 = 65,
    F9 = 66,
    F10 = 67,
    F11 = 68,
    F12 = 69,
    PrintScreen = 70,
    ScrollLock = 71,
    Pause = 72,
    Insert = 73,
    Home = 74,
    PageUp = 75,
    Delete = 76,
    End = 77,
    PageDown = 78,
    Right = 79,
    Left = 80,
    Down = 81,
    Up = 82,
    NumLockClear = 83,
    KeyPadDivide = 84,
    KeyPadMultiply = 85,
    KeyPadMinus = 86,
    KeyPadPlus = 87,
    KeyPadEnter = 88,
    KeyPad1 = 89,
    KeyPad2 = 90,
    KeyPad3 = 91,
    KeyPad4 = 92,
    KeyPad5 = 93,
    KeyPad6 = 94,
    KeyPad7 = 95,
    KeyPad8 = 96,
    KeyPad9 = 97,
    KeyPad0 = 98,
    KeyPadPeriod = 99,
    NonusBackslash = 100,
    Application = 101,
    Power = 102,
    KeyPadEquals = 103,
    F13 = 104,
    F14 = 105,
    F15 = 106,
    F16 = 107,
    F17 = 108,
    F18 = 109,
    F19 = 110,
    F20 = 111,
    F21 = 112,
    F22 = 113,
    F23 = 114,
    F24 = 115,
    Execute = 116,
    Help = 117,
    Menu = 118,
    Select = 119,
    Stop = 120,
    Again = 121,
    Undo = 122,
    Cut = 123,
    Copy = 124,
    Paste = 125,
    Find = 126,
    Mute = 127,
    VolumeUp = 128,
    VolumeDown = 129,
    KeyPadComma = 133,
    KeyPadEqualsAs400 = 134,
    International1 = 135,
    International2 = 136,
    International3 = 137,
    International4 = 138,
    International5 = 139,
    International6 = 140,
    International7 = 141,
    International8 = 142,
    International9 = 143,
    Lang1 = 144,
    Lang2 = 145,
    Lang3 = 146,
    Lang4 = 147,
    Lang5 = 148,
    Lang6 = 149,
    Lang7 = 150,
    Lang8 = 151,
    Lang9 = 152,
    AltErase = 153,
    SysReq = 154,
    Cancel = 155,
    Clear = 156,
    Prior = 157,
    Return2 = 158,
    Separator = 159,
    OUT = 160,
    Oper = 161,
    ClearAgain = 162,
    CrSel = 163,
    ExSel = 164,
    KeyPad00 = 176,
    KeyPad000 = 177,
    ThousandsSeparator = 178,
    DecimalSeparator = 179,
    CurrencyUnit = 180,
    CurrencySubunit = 181,
    KeyPadLeftParen = 182,
    KeyPadRightParen = 183,
    KeyPadLeftBrace = 184,
    KeyPadRightBrace = 185,
    KeyPadTab = 186,
    KeyPadBackspace = 187,
    KeyPadA = 188,
    KeyPadB = 189,
    KeyPadC = 190,
    KeyPadD = 191,
    KeyPadE = 192,
    KeyPadF = 193,
    KeyPadXor = 194,
    KeyPadPower = 195,
    KeyPadPercent = 196,
    KeyPadLess = 197,
    KeyPadGreater = 198,
    KeyPadAmpersand = 199,
    KeyPadDblAmpersand = 200,
    KeyPadVerticalBar = 201,
    KeyPadDblVerticalBar = 202,
    KeyPadColon = 203,
    KeyPadHash = 204,
    KeyPadSpace = 205,
    KeyPadAt = 206,
    KeyPadExclam = 207,
    KeyPadMemStore = 208,
    KeyPadMemRecall = 209,
    KeyPadMemClear = 210,
    KeyPadMemAdd = 211,
    KeyPadMemSubtract = 212,
    KeyPadMemMultiply = 213,
    KeyPadMemDivide = 214,
    KeyPadPlusMinus = 215,
    KeyPadClear = 216,
    KeyPadClearEntry = 217,
    KeyPadBinary = 218,
    KeyPadOctal = 219,
    KeyPadDecimal = 220,
    KeyPadHexadecimal = 221,
    LeftCtrl = 224,
    LeftShift = 225,
    LeftAlt = 226,
    LeftGui = 227,
    RightCtrl = 228,
    RightShift = 229,
    RightAlt = 230,
    RightGui = 231,
    Mode = 257,
    AudioNext = 258,
    AudioPrev = 259,
    AudioStop = 260,
    AudioPlay = 261,
    AudioMute = 262,
    MediaSelect = 263,
    WWW = 264,
    Mail = 265,
    Calculator = 266,
    Computer = 267,
    AudioControlSearch = 268,
    AudioControlHome = 269,
    AudioControlBack = 270,
    AudioControlForward = 271,
    AudioControlStop = 272,
    AudioControlRefresh = 273,
    AudioControlBookmarks = 274,
    BrightnessDown = 275,
    BrightnessUp = 276,
    DisplaySwitch = 277,
    Eject = 281,
    Sleep = 282,
    App1 = 283,
    App2 = 284,
    AudioRewind = 285,
    AudioFastForward = 286,
    SoftLeft = 287,
    SoftRight = 288,
    Call = 289,
    EndCall = 290,
}

impl PartialEq<u32> for Scancode {
    fn eq(&self, other: &u32) -> bool {
        (*self) as u32 == *other
    }
}

impl PartialEq for Scancode {
    fn eq(&self, other: &Self) -> bool {
        (*self) as u32 == (*other) as u32
    }
}
