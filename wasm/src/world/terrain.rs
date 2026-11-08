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
    /// The rock face of a cliff, below the high ground's edge.
    Cliff,
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
        Feature::BroadleafStump(_) | Feature::Stump(_) | Feature::Rock | Feature::Bush | Feature::Driftwood(_) | Feature::Vine | Feature::VineStake => true,
        _ => false,
    };
    TileProps {
        solid: !matches!(feature, Feature::Ladder(_)) && (feature_solid || matches!(terrain, Terrain::DeepWater | Terrain::River) || terrain.is_cliff()),
        wading: terrain == Terrain::ShallowWater,
    }
}
