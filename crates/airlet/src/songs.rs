use crate::{CypherNotation, NoteEvent, Pitch, Score};

pub mod air {
    use super::*;

    #[allow(dead_code)]
    pub const WN: u64 = QN * 4;
    #[allow(dead_code)]
    pub const HN: u64 = QN * 2;
    pub const QN: u64 = 500;
    pub const EN: u64 = QN / 2;
    pub const SN: u64 = QN / 4;

    pub fn intro() -> Score {
        Score::new("鳥の詩 intro", intro_melody())
    }

    pub fn intro_melody() -> Vec<NoteEvent> {
        let music = CypherNotation::new(Pitch::D);
        let midi = |note, oct| music.midi(note, oct);

        vec![
            // 第一小节
            NoteEvent::new(midi(6, 0), QN + EN),
            NoteEvent::new(midi(7, 0), EN),
            NoteEvent::new(midi(1, 1), EN),
            NoteEvent::new(midi(5, 1), EN),
            // 第二小节
            NoteEvent::new(midi(3, 1), QN),
            NoteEvent::new(midi(3, 1), EN),
            NoteEvent::new(midi(2, 1), SN),
            NoteEvent::new(midi(3, 1), SN + QN),
            NoteEvent::rest(QN),
            // 第三小节
            NoteEvent::rest(QN),
            NoteEvent::new(midi(2, 1), EN),
            NoteEvent::new(midi(3, 1), EN),
            NoteEvent::new(midi(5, 1), EN),
            NoteEvent::new(midi(1, 1), EN),
            NoteEvent::new(midi(7, 0), EN),
            NoteEvent::new(midi(1, 1), EN),
            // 第四小节
            NoteEvent::new(midi(7, 0), QN),
            NoteEvent::new(midi(7, 0), EN),
            NoteEvent::new(midi(6, 0), SN),
            NoteEvent::new(midi(3, 0), SN + QN),
            NoteEvent::rest(QN),
            // 第五小节
            NoteEvent::rest(QN + EN),
            NoteEvent::new(midi(6, 0), QN),
            NoteEvent::new(midi(7, 0), EN),
            NoteEvent::new(midi(1, 1), EN),
            NoteEvent::new(midi(5, 1), EN),
            // 第六小节
            NoteEvent::new(midi(3, 1), QN),
            NoteEvent::new(midi(3, 1), EN),
            NoteEvent::new(midi(2, 1), SN),
            NoteEvent::new(midi(3, 1), SN + QN),
            NoteEvent::rest(QN),
            // 第七小节
            NoteEvent::rest(QN),
            NoteEvent::new(midi(2, 1), EN),
            NoteEvent::new(midi(3, 1), EN),
            NoteEvent::new(midi(5, 1), EN),
            NoteEvent::new(midi(3, 1), EN),
            NoteEvent::new(midi(5, 1), EN),
            NoteEvent::new(midi(1, 2), EN),
            // 第八小节
            NoteEvent::new(midi(7, 1), EN + SN),
            NoteEvent::new(midi(6, 1), EN + SN),
            NoteEvent::new(midi(3, 1), QN + EN),
            NoteEvent::rest(EN),
            NoteEvent::new(midi(2, 1), EN),
            // 第九小节
        ]
    }
}
