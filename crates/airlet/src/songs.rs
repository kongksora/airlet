use crate::{
    NoteEvent, Pitch, Score,
    score::{ComposedScore, Dur, ScoreBuilder},
};

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

    pub fn intro_score() -> ComposedScore {
        ScoreBuilder::cypher("鳥の詩 intro", Pitch::D)
            .tempo_quarter_millis(QN)
            .voice("melody", |v| {
                // 第一小节
                v.n(6, 0, Dur::QUARTER + Dur::EIGHTH)
                    .n(7, 0, Dur::EIGHTH)
                    .n(1, 1, Dur::EIGHTH)
                    .n(5, 1, Dur::EIGHTH);
                // 第二小节
                v.n(3, 1, Dur::QUARTER)
                    .n(3, 1, Dur::EIGHTH)
                    .n(2, 1, Dur::SIXTEENTH)
                    .n(3, 1, Dur::SIXTEENTH + Dur::QUARTER)
                    .rest(Dur::QUARTER);
                // 第三小节
                v.rest(Dur::QUARTER)
                    .n(2, 1, Dur::EIGHTH)
                    .n(3, 1, Dur::EIGHTH)
                    .n(5, 1, Dur::EIGHTH)
                    .n(1, 1, Dur::EIGHTH)
                    .n(7, 0, Dur::EIGHTH)
                    .n(1, 1, Dur::EIGHTH);
                // 第四小节
                v.n(7, 0, Dur::QUARTER)
                    .n(7, 0, Dur::EIGHTH)
                    .n(6, 0, Dur::SIXTEENTH)
                    .n(3, 0, Dur::SIXTEENTH + Dur::QUARTER)
                    .rest(Dur::QUARTER);
                // 第五小节
                v.rest(Dur::QUARTER + Dur::EIGHTH)
                    .n(6, 0, Dur::QUARTER)
                    .n(7, 0, Dur::EIGHTH)
                    .n(1, 1, Dur::EIGHTH)
                    .n(5, 1, Dur::EIGHTH);
                // 第六小节
                v.n(3, 1, Dur::QUARTER)
                    .n(3, 1, Dur::EIGHTH)
                    .n(2, 1, Dur::SIXTEENTH)
                    .n(3, 1, Dur::SIXTEENTH + Dur::QUARTER)
                    .rest(Dur::QUARTER);
                // 第七小节
                v.rest(Dur::QUARTER)
                    .n(2, 1, Dur::EIGHTH)
                    .n(3, 1, Dur::EIGHTH)
                    .n(5, 1, Dur::EIGHTH)
                    .n(3, 1, Dur::EIGHTH)
                    .n(5, 1, Dur::EIGHTH)
                    .n(1, 2, Dur::EIGHTH);
                // 第八小节
                v.n(7, 1, Dur::EIGHTH + Dur::SIXTEENTH)
                    .n(6, 1, Dur::EIGHTH + Dur::SIXTEENTH)
                    .n(3, 1, Dur::QUARTER + Dur::EIGHTH)
                    .rest(Dur::EIGHTH)
                    .n(2, 1, Dur::EIGHTH);
            })
            .finish()
    }

    pub fn intro_melody() -> Vec<NoteEvent> {
        intro_score().to_note_events()
    }
}
