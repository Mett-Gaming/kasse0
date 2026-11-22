//! Die Animationen der KASSE-0-Figuren (Reihenfolge wie CLIPS in tools/prepare_people.py).

use mettgine::anim::ClipId;

/// Animation einer Figur (Reihenfolge wie CLIPS in tools/prepare_people.py)
/// Nicht jeder Clip wird schon benutzt; die Reihenfolge muss trotzdem zur Datei passen.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Clip {
    #[default]
    Idle = 0,
    Walk,
    Reach,
    Run,
    /// zupacken (Ausfallschritt mit ausgestrecktem Arm)
    Grab,
    Wave,
    Hit,
    Death,
    CrouchWalk,
    CrouchIdle,
    ZombieWalk,
    ZombieIdle,
    Scratch,
    /// vom Boden aufstehen
    GetUp,
    FoldArms,
    Phone,
    Talk,
    PickUp,
    /// Wagen schieben
    Push,
    Carry,
    /// kniend ins Regal einraeumen
    Kneel,
    No,
    WalkFormal,
    /// Sitzen (Kassiererin auf dem Buerostuhl)
    Sit,
}

/// Sitzpose: so weit liegt das Gesaess hinter dem Figur-Ursprung (Meter, gegen die
/// Blickrichtung) - so stellt man die Figur, damit sie auf einem Stuhl landet
pub const SIT_BACK: f32 = 0.25;
/// und so viel hoeher, damit es auf der Sitzflaeche aufliegt
pub const SIT_UP: f32 = 0.03;

impl Clip {
    /// Laeuft die Figur mit diesem Clip vorwaerts (Zeit an die Strecke gekoppelt)?
    pub fn locomotion(self) -> bool {
        matches!(
            self,
            Clip::Walk
                | Clip::Run
                | Clip::CrouchWalk
                | Clip::ZombieWalk
                | Clip::Push
                | Clip::Carry
                | Clip::WalkFormal
        )
    }
}

impl ClipId for Clip {
    fn index(self) -> usize {
        self as usize
    }
    fn locomotion(self) -> bool {
        Clip::locomotion(self)
    }
    fn looping(self) -> bool {
        !matches!(
            self,
            Clip::Grab | Clip::Hit | Clip::Death | Clip::GetUp | Clip::PickUp
        )
    }
}
