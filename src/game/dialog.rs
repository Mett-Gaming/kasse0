//! Dialoge mit den gesichtslosen Mitarbeitern.
//! Jede Frage hat eine gute, eine neutrale und eine schlechte Antwort (Reihenfolge zufaellig).

pub struct Question {
    pub text: &'static str,
    /// [gut, neutral, schlecht]
    pub answers: [&'static str; 3],
}

pub const QUESTIONS: &[Question] = &[
    Question {
        text: "Kann ich Ihnen helfen?",
        answers: [
            "Nein danke, ich schaue nur.",
            "Wo finde ich die Milch?",
            "Warum haben Sie kein Gesicht?",
        ],
    },
    Question {
        text: "Haben Sie eine Kundenkarte?",
        answers: [
            "Ja, die zeige ich an der Kasse.",
            "Nein.",
            "Was ist hier los?!",
        ],
    },
    Question {
        text: "Wir schließen gleich.",
        answers: [
            "Ich bin gleich fertig.",
            "Wann denn?",
            "Lassen Sie mich raus!",
        ],
    },
    Question {
        text: "Der Tomatensaft ist heute im Angebot.",
        answers: [
            "Danke, sehr freundlich.",
            "Ich mag keinen Tomatensaft.",
            "Der ist doch abgelaufen.",
        ],
    },
    Question {
        text: "Sie sind nicht von hier, oder?",
        answers: [
            "Ich kaufe hier jeden Tag ein.",
            "Doch, doch.",
            "Ich weiß nicht, wo ich bin.",
        ],
    },
    Question {
        text: "Sind Sie Kunde Nummer 0?",
        answers: ["Nein, ich habe eine andere Nummer.", "Vielleicht.", "Ja."],
    },
    Question {
        text: "Haben Sie alles gefunden?",
        answers: ["Ja, danke.", "Fast.", "Hier stimmt doch was nicht!"],
    },
    Question {
        text: "Bitte lächeln.",
        answers: [
            "(lächeln)",
            "Warum?",
            "Womit denn? Sie haben ja keinen Mund.",
        ],
    },
];

/// Wenn man einen Mitarbeiter anspricht, der gerade den eigenen Wagen wegschiebt.
pub const CART_OPENER: &str = "Hey, das ist mein Wagen!";
pub const CART_QUESTION: Question = Question {
    text: "Der stand mitten im Weg. Herrenlos.",
    answers: [
        "Entschuldigung, ich brauche ihn noch.",
        "Der stand doch gut da.",
        "Finger weg von meinem Wagen!",
    ],
};

pub const REPLY_GOOD: &str = "Schönen Tag noch.";
pub const REPLY_NEUTRAL: &str = "Hm.";
pub const REPLY_BAD: &str = "...";
pub const ANSWER_TIME: f32 = 9.0;

pub struct DialogState {
    pub employee: usize,
    pub question: usize,
    /// order[k] = Index in `answers` fuer die k-te angezeigte Option
    pub order: [usize; 3],
    pub timer: f32,
    /// Antwort des Mitarbeiters und verbleibende Anzeigezeit
    pub reply: Option<(&'static str, f32)>,
    pub chosen: Option<usize>,
    /// Gespraech ueber den weggeschobenen Wagen (vom Spieler begonnen)
    pub cart_talk: bool,
}

impl DialogState {
    pub fn question(&self) -> &'static Question {
        if self.cart_talk {
            &CART_QUESTION
        } else {
            &QUESTIONS[self.question]
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Good,
    Neutral,
    Bad,
}

pub fn verdict(answer_index: usize) -> Verdict {
    match answer_index {
        0 => Verdict::Good,
        1 => Verdict::Neutral,
        _ => Verdict::Bad,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_question_has_distinct_answers() {
        for q in QUESTIONS {
            assert_ne!(q.answers[0], q.answers[1]);
            assert_ne!(q.answers[1], q.answers[2]);
            assert!(
                q.answers.iter().all(|a| a.chars().count() <= 40),
                "{} zu lang",
                q.text
            );
        }
    }
}
