#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Pitch {
    C,
    CSharp,
    DFlat,
    D,
    DSharp,
    EFlat,
    E,
    F,
    FSharp,
    GFlat,
    G,
    GSharp,
    AFlat,
    A,
    ASharp,
    BFlat,
    B,
}

impl Pitch {
    pub const fn to_midi(&self, octave: i32) -> i32 {
        let base = match self {
            Pitch::C => 0,
            Pitch::CSharp | Pitch::DFlat => 1,
            Pitch::D => 2,
            Pitch::DSharp | Pitch::EFlat => 3,
            Pitch::E => 4,
            Pitch::F => 5,
            Pitch::FSharp | Pitch::GFlat => 6,
            Pitch::G => 7,
            Pitch::GSharp | Pitch::AFlat => 8,
            Pitch::A => 9,
            Pitch::ASharp | Pitch::BFlat => 10,
            Pitch::B => 11,
        };
        base + (octave + 1) * 12
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CypherNotation {
    key: Pitch,
}

impl CypherNotation {
    pub fn new(key: Pitch) -> Self {
        Self { key }
    }

    pub fn midi(&self, note: i32, octave: i32) -> i32 {
        let offset = match note {
            1 => 0,
            2 => 2,
            3 => 4,
            4 => 5,
            5 => 7,
            6 => 9,
            7 => 11,
            _ => panic!("Invalid note: {}", note),
        };
        self.key.to_midi(4 + octave) + offset
    }
}
