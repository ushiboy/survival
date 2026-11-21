#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Terrain {
    DeepWater,
    ShallowWater,
    Sand,
    Grass,
    /// The river: fast fresh water, too deep to wade.
    River,
    /// High ground above a cliff (the plateau toward the island's centre).
    Highland,
    /// The edge of the high ground, where it drops away: no footing.
    CliffTop,
    /// The rock face of a cliff, below the high ground's edge (or below a
    /// crag's).
    Cliff,
    /// The top of a crag: bare rock heaped up on the high ground, too
    /// steep and broken to climb.
    Crag,
    /// An earthen slope cut through the cliff, climbing up onto the high
    /// ground from the land below.
    Slope,
}

impl Terrain {
    /// The sea.
    pub fn is_water(self) -> bool {
        matches!(self, Terrain::DeepWater | Terrain::ShallowWater)
    }

    /// Dry ground.
    pub fn is_land(self) -> bool {
        matches!(self, Terrain::Sand | Terrain::Grass | Terrain::Highland)
    }

    /// Water of any kind: the sea or the river.
    pub fn holds_water(self) -> bool {
        self.is_water() || self == Terrain::River
    }

    /// Part of a cliff: its edge or its face.
    pub fn is_cliff(self) -> bool {
        matches!(self, Terrain::CliffTop | Terrain::Cliff)
    }
}

/// What the ground of a cell is made of, where it isn't plain grass: the
/// sea's beaches are sand; the lake's shore (and its bed) is wet soil.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Soil {
    #[default]
    Sand,
    /// Damp, dark earth: the lake's shore.
    Mud,
}

/// Which cell of a 3x4 palm tree a map cell holds. Only the trunk base
/// (bottom centre) is solid; everything else can be walked behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PalmPart {
    pub dx: u8,
    pub dy: u8,
    /// Mirrored tree (uses the PPU's h-flip, no extra tiles).
    pub flip: bool,
}

impl PalmPart {
    pub const W: i32 = 3;
    pub const H: i32 = 4;

    pub fn is_trunk_base(&self) -> bool {
        self.dx as i32 == Self::W / 2 && self.dy as i32 == Self::H - 1
    }

    /// The canopy cell the coconuts hang in.
    pub fn is_crown(&self) -> bool {
        self.dx as i32 == Self::W / 2 && self.dy == 1
    }

    /// Top-left cell of the tree this part at `(x, y)` belongs to.
    pub fn anchor(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.dx as i32, y - self.dy as i32)
    }
}

/// Which cell of a 3x4 broadleaf tree (a lauan) a map cell holds: a round
/// crown over a straight trunk. As with palms only the trunk base is solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BroadleafPart {
    pub dx: u8,
    pub dy: u8,
    pub flip: bool,
}

impl BroadleafPart {
    pub const W: i32 = 3;
    pub const H: i32 = 4;

    pub fn is_trunk_base(&self) -> bool {
        self.dx as i32 == Self::W / 2 && self.dy as i32 == Self::H - 1
    }

    /// Top-left cell of the tree this part at `(x, y)` belongs to.
    pub fn anchor(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.dx as i32, y - self.dy as i32)
    }
}

/// Which cell of a 3x4 pine (a conifer of the high ground) a map cell
/// holds: tiers of needles narrowing to a point over a straight trunk. As
/// with the other trees only the trunk base is solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PinePart {
    pub dx: u8,
    pub dy: u8,
    pub flip: bool,
}

impl PinePart {
    pub const W: i32 = 3;
    pub const H: i32 = 4;

    pub fn is_trunk_base(&self) -> bool {
        self.dx as i32 == Self::W / 2 && self.dy as i32 == Self::H - 1
    }

    /// Top-left cell of the tree this part at `(x, y)` belongs to.
    pub fn anchor(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.dx as i32, y - self.dy as i32)
    }
}

/// Which cell of a bamboo culm a map cell holds: one tall jointed stalk,
/// a cell wide and four high, with sprays of narrow leaves up top. `v`
/// picks one of a few shapes (lean, height), so a grove isn't all alike.
/// The foot (the bottom cell) is solid; the rest stands over the ground
/// behind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct BambooPart {
    pub dy: u8,
    pub v: u8,
    pub flip: bool,
}

impl BambooPart {
    pub const H: i32 = 4;
    /// How many shapes a culm comes in.
    pub const SHAPES: u8 = 3;

    pub fn is_foot(&self) -> bool {
        self.dy as i32 == Self::H - 1
    }

    /// Top cell of the culm this part at `(x, y)` belongs to.
    pub fn anchor(&self, x: i32, y: i32) -> (i32, i32) {
        (x, y - self.dy as i32)
    }
}

/// One cell of a driftwood log lying on the beach, two cells long:
/// `dx` 0 is the left cell. Mirrored logs have their roots on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DriftwoodPart {
    pub dx: u8,
    pub flip: bool,
}

impl DriftwoodPart {
    pub const W: i32 = 2;

    /// Left cell of the log this part at `(x, y)` belongs to.
    pub fn anchor(&self, x: i32, y: i32) -> (i32, i32) {
        (x - self.dx as i32, y)
    }
}

/// One cell of a bed of clay showing through the lake shore's mud, two
/// cells by two: `part` is `dx + 2 * dy` from its top-left cell. Once dug
/// (`dug`, the whole bed) it's a wet hole until the next morning, when the
/// lake's water has seeped back and softened it again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ClayPart {
    pub part: u8,
    pub dug: bool,
}

/// The cells of a two-by-two patch (clay or bare earth) whose top-left is
/// `anchor`, by part (`dx + 2 * dy`).
pub fn patch_cells((x, y): (i32, i32)) -> [(i32, i32); 4] {
    [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
}

/// The top-left cell of the two-by-two patch whose `part` is at `(x, y)`.
pub fn patch_anchor(x: i32, y: i32, part: u8) -> (i32, i32) {
    (x - (part % 2) as i32, y - (part / 2) as i32)
}

/// Where the mill is marked out: a stake at a corner of the ground, and
/// the rope run from it (or past it, between the stakes) along the edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Marking {
    /// A stake driven in here (a corner): it stands in the way.
    pub stake: bool,
    /// Which way rope runs out of the cell's middle (`Marking::NORTH` and
    /// so on, or'd together); the rope lies on the ground and is stepped
    /// over.
    pub ropes: u8,
}

impl Marking {
    pub const NORTH: u8 = 1;
    pub const EAST: u8 = 2;
    pub const SOUTH: u8 = 4;
    pub const WEST: u8 = 8;
}

/// A palm stump (left in the old trunk-base cell).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct StumpPart {
    pub flip: bool,
}

/// Objects placed on top of terrain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Feature {
    None,
    Palm(PalmPart),
    Stump(StumpPart),
    Broadleaf(BroadleafPart),
    /// What's left of a felled broadleaf tree.
    BroadleafStump(StumpPart),
    Rock,
    Bush,
    /// A bleached log washed up on the beach; the axe splits it into
    /// branches.
    Driftwood(DriftwoodPart),
    Shell,
    Flower,
    /// A climbing plant whose vine can be pulled off (once).
    Vine,
    /// What's left once the vine is taken: the bare stake it climbed.
    VineStake,
    /// A ladder stood against a cliff: `0` on the cliff's edge at its top,
    /// then `1`, `2`, ... down the rock face. It can be climbed up and
    /// down, so these cells aren't solid.
    Ladder(u8),
    /// A log bridge laid across the river: `0` on the west bank at its
    /// end, `1` over the water, `2` on the east bank. It can be walked
    /// over, so these cells aren't solid.
    Bridge(u8),
    /// A water wheel set in the river at the mill site, two cells wide and
    /// two tall: `dx + 2 * dy` from its top-left cell.
    WaterWheel(u8),
    /// A wood lathe set on the bank beside the water wheel, which drives
    /// it, two cells wide and two tall: `dx + 2 * dy` from its top-left
    /// cell.
    Lathe(u8),
    Pine(PinePart),
    /// What's left of a felled pine.
    PineStump(StumpPart),
    /// A pine cone lying on the ground, fallen from a pine; picked up like
    /// a shell.
    PineCone,
    /// A bamboo culm (in the groves behind the west coast).
    Bamboo(BambooPart),
    /// What's left of a cut bamboo culm: a short stalk, cut aslant.
    BambooStump(StumpPart),
    /// A bed of clay on the lake's shore; a bamboo spade lifts clay out
    /// of it once a day.
    ClayBed(ClayPart),
    /// Bare earth showing through the grass, two cells by two (`dx + 2 *
    /// dy` from its top-left cell); a digging stick loosens soil out of it.
    BareEarth(u8),
    /// A signal fire set on open ground: a hearth of stones bedded in
    /// earth with timber stacked on it, one cell. `true` while it burns,
    /// sending up smoke (see `Game`'s signal fires for when it was lit).
    /// Drawn as a sprite over the ground, which shows as if the cell were
    /// bare. It can be walked over, like a campfire (set down anywhere, it
    /// never blocks the way).
    SignalFire(bool),
    /// The ground for the mill marked out: stakes at its corners with
    /// rope between them.
    Marking(Marking),
    /// The bench set up beside the mill's marked-out ground while it's
    /// built: a plank on two stones, the plan and the line on it. A at it
    /// opens the BUILD screen. It stands in the way.
    SiteBench,
    /// The mill's foundation: the ground tamped down hard, and (`true`) a
    /// flat stone bedded in it where a post will stand. Walked over.
    Foundation(bool),
    /// The mill's frame raised on its foundation: the tamped ground, and
    /// (`true`) a post stood on the stone there, which stands in the way.
    /// The plates and beams between the posts' heads are drawn over the
    /// ground round it (`render::tileset`).
    Framed(bool),
    /// A stretch of the mill's walls between its posts: bamboo lath
    /// plastered with clay and soil. It stands in the way. The faces of
    /// the walls are drawn over the ground round it (`render::tileset`).
    MillWall,
    /// The mill's doorway once the roof is on: a sliding plank door hung
    /// in it, that slides open into the wall as someone comes up to it
    /// (`WorldMap::mill_door`). It doesn't stand in the way.
    MillDoor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileProps {
    pub solid: bool,
    /// Walkable but slow, and the walker's legs are hidden.
    pub wading: bool,
}

impl TileProps {
    pub const SOLID: Self = Self { solid: true, wading: false };
}

pub fn tile_props(terrain: Terrain, feature: Feature) -> TileProps {
    let feature_solid = match feature {
        Feature::Palm(p) => p.is_trunk_base(),
        Feature::Broadleaf(p) => p.is_trunk_base(),
        Feature::Pine(p) => p.is_trunk_base(),
        Feature::Bamboo(p) => p.is_foot(),
        Feature::BroadleafStump(_)
        | Feature::PineStump(_)
        | Feature::BambooStump(_)
        | Feature::Stump(_)
        | Feature::Rock
        | Feature::Bush
        | Feature::Driftwood(_)
        | Feature::Vine
        | Feature::VineStake
        | Feature::Lathe(_)
        | Feature::SiteBench
        | Feature::MillWall => true,
        Feature::Marking(m) => m.stake,
        Feature::Framed(post) => post,
        _ => false,
    };
    TileProps {
        solid: !matches!(feature, Feature::Ladder(_) | Feature::Bridge(_))
            && (feature_solid || matches!(terrain, Terrain::DeepWater | Terrain::River | Terrain::Crag) || terrain.is_cliff()),
        wading: terrain == Terrain::ShallowWater,
    }
}
